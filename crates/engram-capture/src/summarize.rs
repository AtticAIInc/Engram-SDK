//! LLM-powered summarization for engram intent fields.
//!
//! When `ANTHROPIC_API_KEY` is set, sends a condensed transcript to Claude
//! to generate high-quality summary, interpreted_goal, dead_ends, and decisions.
//! Falls back silently to existing heuristic-extracted fields if the API key
//! is missing or the call fails.

use engram_core::model::{
    transcript::TranscriptContent, DeadEnd, Decision, EngramData, FileChangeType,
};

use crate::error::CaptureError;

const API_URL: &str = "https://api.anthropic.com/v1/messages";
const API_VERSION: &str = "2023-06-01";
const DEFAULT_MODEL: &str = "claude-haiku-4-5-20251001";
const MAX_PROMPT_CHARS: usize = 50_000;
const MAX_TOKENS: u32 = 2048;
const TIMEOUT_SECS: u64 = 60;

const SYSTEM_PROMPT: &str = r#"You are analyzing an AI coding session transcript. Extract structured metadata about what happened in this session.

Respond with ONLY valid JSON (no markdown fencing, no explanation) in this exact format:
{
  "summary": "1-2 sentence summary of what was accomplished in this session",
  "interpreted_goal": "What the AI agent understood the user wanted and the strategy it used to achieve it",
  "dead_ends": [{"approach": "what was tried", "reason": "why it was rejected or didn't work"}],
  "decisions": [{"description": "what was decided", "rationale": "why this choice was made over alternatives"}]
}

Rules:
- summary: Focus on outcomes, not process. What was built/fixed/changed?
- interpreted_goal: Describe the strategy, not just restate the request. How did the agent plan to solve it?
- dead_ends: Only include approaches that were actually tried and abandoned. Omit if none.
- decisions: Key architectural or design choices with rationale. Omit if none.
- Keep dead_ends and decisions arrays empty [] if there are genuinely none."#;

/// Enhance intent fields in an EngramData using LLM summarization.
///
/// Checks for API key in this order:
/// 1. `ANTHROPIC_API_KEY` environment variable
/// 2. `anthropic_api_key` in global config (`~/.config/engram/repos.toml`)
///
/// If neither is set or the API call fails, this is a no-op.
/// The existing heuristic-extracted fields are left unchanged in that case.
pub fn summarize_intent(data: &mut EngramData) -> Result<(), CaptureError> {
    // Load global config for fallback API key / model (best-effort)
    let global_config = engram_core::config::GlobalConfig::load().ok();

    let api_key = match std::env::var("ANTHROPIC_API_KEY") {
        Ok(k) if !k.is_empty() => k,
        _ => match global_config
            .as_ref()
            .and_then(|c| c.settings.anthropic_api_key.clone())
        {
            Some(k) if !k.is_empty() => k,
            _ => {
                tracing::debug!("No API key found (env or config), skipping LLM summarization");
                return Ok(());
            }
        },
    };

    let model = std::env::var("ENGRAM_SUMMARIZE_MODEL")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            global_config
                .as_ref()
                .and_then(|c| c.settings.summarize_model.clone())
        })
        .unwrap_or_else(|| DEFAULT_MODEL.to_string());

    let user_message = build_prompt(data);

    match call_claude(&api_key, &model, &user_message) {
        Ok(response_text) => match parse_response(&response_text) {
            Ok(parsed) => {
                apply_response(data, &parsed);
                tracing::debug!("LLM summarization applied successfully");
                Ok(())
            }
            Err(e) => {
                tracing::warn!("Failed to parse LLM response, keeping heuristic fields: {e}");
                Ok(())
            }
        },
        Err(e) => {
            tracing::warn!("LLM API call failed, keeping heuristic fields: {e}");
            Ok(())
        }
    }
}

/// Build a condensed transcript prompt for the LLM.
fn build_prompt(data: &EngramData) -> String {
    let mut parts = Vec::new();

    // Original request
    parts.push(format!("## User Request\n{}", data.intent.original_request));

    // Condensed transcript
    parts.push("\n## Session Transcript".to_string());
    for entry in &data.transcript.entries {
        let role = match entry.role {
            engram_core::model::transcript::Role::User => "USER",
            engram_core::model::transcript::Role::Assistant => "ASSISTANT",
            engram_core::model::transcript::Role::System => "SYSTEM",
            engram_core::model::transcript::Role::Tool => "TOOL",
        };

        match &entry.content {
            TranscriptContent::Text { text } => {
                let t = truncate_str(text, 500);
                parts.push(format!("[{role}] {t}"));
            }
            TranscriptContent::ToolUse {
                tool_name, input, ..
            } => {
                // Include tool name and a brief input summary
                let input_summary = summarize_json_input(input);
                parts.push(format!("[TOOL_USE] {tool_name}: {input_summary}"));
            }
            TranscriptContent::Thinking { text } => {
                let t = truncate_str(text, 300);
                parts.push(format!("[THINKING] {t}"));
            }
            TranscriptContent::ToolResult { .. } => {
                // Skip tool results — too verbose, low signal for summarization
            }
        }
    }

    // File changes
    if !data.operations.file_changes.is_empty() {
        parts.push("\n## File Changes".to_string());
        for fc in &data.operations.file_changes {
            let symbol = match &fc.change_type {
                FileChangeType::Created => "+",
                FileChangeType::Modified => "~",
                FileChangeType::Deleted => "-",
                FileChangeType::Renamed { .. } => "->",
            };
            let mut line = format!("{symbol} {}", fc.path);
            if let Some(added) = fc.lines_added {
                line.push_str(&format!(" (+{added})"));
            }
            if let Some(removed) = fc.lines_removed {
                line.push_str(&format!(" (-{removed})"));
            }
            parts.push(line);
        }
    }

    let full = parts.join("\n");
    if full.len() > MAX_PROMPT_CHARS {
        let mut end = MAX_PROMPT_CHARS;
        while end > 0 && !full.is_char_boundary(end) {
            end -= 1;
        }
        full[..end].to_string()
    } else {
        full
    }
}

/// Truncate a string to max_len chars, appending "..." if truncated.
fn truncate_str(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        // Find a safe char boundary
        let mut end = max_len;
        while end > 0 && !s.is_char_boundary(end) {
            end -= 1;
        }
        &s[..end]
    }
}

/// Summarize a JSON tool input to a brief string.
fn summarize_json_input(input: &serde_json::Value) -> String {
    match input {
        serde_json::Value::Object(map) => {
            let keys: Vec<&String> = map.keys().take(3).collect();
            if keys.is_empty() {
                "{}".to_string()
            } else {
                let pairs: Vec<String> = keys
                    .iter()
                    .map(|k| {
                        let v = &map[*k];
                        let v_str = match v {
                            serde_json::Value::String(s) => truncate_str(s, 60).to_string(),
                            _ => {
                                let s = v.to_string();
                                truncate_str(&s, 60).to_string()
                            }
                        };
                        format!("{k}={v_str}")
                    })
                    .collect();
                pairs.join(", ")
            }
        }
        serde_json::Value::String(s) => truncate_str(s, 100).to_string(),
        other => {
            let s = other.to_string();
            truncate_str(&s, 100).to_string()
        }
    }
}

/// Call the Anthropic Messages API.
fn call_claude(api_key: &str, model: &str, user_message: &str) -> Result<String, CaptureError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(TIMEOUT_SECS))
        .build()
        .map_err(|e| CaptureError::LlmError(format!("HTTP client error: {e}")))?;

    let body = serde_json::json!({
        "model": model,
        "max_tokens": MAX_TOKENS,
        "system": SYSTEM_PROMPT,
        "messages": [
            { "role": "user", "content": user_message }
        ]
    });

    let response = client
        .post(API_URL)
        .header("x-api-key", api_key)
        .header("anthropic-version", API_VERSION)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|e| CaptureError::LlmError(format!("API request failed: {e}")))?;

    if !response.status().is_success() {
        let status = response.status();
        let body_text = response.text().unwrap_or_default();
        return Err(CaptureError::LlmError(format!(
            "API returned {status}: {body_text}"
        )));
    }

    let resp_json: serde_json::Value = response
        .json()
        .map_err(|e| CaptureError::LlmError(format!("Failed to parse API response: {e}")))?;

    // Extract text from the first text content block (skip thinking blocks)
    resp_json["content"]
        .as_array()
        .and_then(|arr| {
            arr.iter()
                .find(|block| block["type"].as_str() == Some("text"))
        })
        .and_then(|block| block["text"].as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| {
            let content_preview = resp_json["content"].to_string();
            let preview = if content_preview.len() > 200 {
                format!("{}...", &content_preview[..200])
            } else {
                content_preview
            };
            CaptureError::LlmError(format!("No text block in API response: {preview}"))
        })
}

/// Response from LLM summarization.
struct SummarizeResponse {
    summary: Option<String>,
    interpreted_goal: Option<String>,
    dead_ends: Vec<DeadEnd>,
    decisions: Vec<Decision>,
}

/// Parse the LLM JSON response into structured fields.
fn parse_response(text: &str) -> Result<SummarizeResponse, CaptureError> {
    // Strip markdown code fencing if present
    let cleaned = text
        .trim()
        .strip_prefix("```json")
        .or_else(|| text.trim().strip_prefix("```"))
        .unwrap_or(text.trim());
    let cleaned = cleaned.strip_suffix("```").unwrap_or(cleaned).trim();

    if cleaned.is_empty() {
        return Err(CaptureError::LlmError(
            "LLM returned empty response".to_string(),
        ));
    }

    let json: serde_json::Value = serde_json::from_str(cleaned).map_err(|e| {
        let preview = if cleaned.len() > 200 {
            format!("{}...", &cleaned[..200])
        } else {
            cleaned.to_string()
        };
        tracing::debug!("LLM returned non-JSON text: {preview:?}");
        CaptureError::LlmError(format!("Invalid JSON from LLM: {e}"))
    })?;

    let summary = json["summary"].as_str().map(|s| s.to_string());
    let interpreted_goal = json["interpreted_goal"].as_str().map(|s| s.to_string());

    let dead_ends = json["dead_ends"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|de| {
                    Some(DeadEnd {
                        approach: de["approach"].as_str()?.to_string(),
                        reason: de["reason"].as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let decisions = json["decisions"]
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|d| {
                    Some(Decision {
                        description: d["description"].as_str()?.to_string(),
                        rationale: d["rationale"].as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    Ok(SummarizeResponse {
        summary,
        interpreted_goal,
        dead_ends,
        decisions,
    })
}

/// Apply LLM-generated fields to the EngramData.
///
/// Merges LLM insights with existing heuristic-extracted ones rather than
/// replacing them, so no extracted insight is lost.
fn apply_response(data: &mut EngramData, response: &SummarizeResponse) {
    if let Some(summary) = &response.summary {
        data.manifest.summary = Some(summary.clone());
        data.intent.summary = Some(summary.clone());
    }
    if let Some(goal) = &response.interpreted_goal {
        data.intent.interpreted_goal = Some(goal.clone());
    }
    // Merge dead ends: LLM findings first, then append heuristic ones not already covered
    if !response.dead_ends.is_empty() {
        let mut merged = response.dead_ends.clone();
        for existing in &data.intent.dead_ends {
            let dominated = merged
                .iter()
                .any(|de| de.approach.to_lowercase() == existing.approach.to_lowercase());
            if !dominated {
                merged.push(existing.clone());
            }
        }
        data.intent.dead_ends = merged;
    }
    // Same merge strategy for decisions
    if !response.decisions.is_empty() {
        let mut merged = response.decisions.clone();
        for existing in &data.intent.decisions {
            let dominated = merged.iter().any(|d| {
                d.description.to_lowercase() == existing.description.to_lowercase()
            });
            if !dominated {
                merged.push(existing.clone());
            }
        }
        data.intent.decisions = merged;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_response_valid() {
        let json = r#"{
            "summary": "Added OAuth2 authentication with PKCE flow",
            "interpreted_goal": "Implement a secure OAuth2 flow using PKCE for the SPA",
            "dead_ends": [
                {"approach": "passport.js", "reason": "Middleware conflict with existing stack"}
            ],
            "decisions": [
                {"description": "Custom middleware over Auth0 SDK", "rationale": "Auth0 added 2MB to bundle"}
            ]
        }"#;

        let result = parse_response(json).unwrap();
        assert_eq!(
            result.summary.as_deref(),
            Some("Added OAuth2 authentication with PKCE flow")
        );
        assert_eq!(result.dead_ends.len(), 1);
        assert_eq!(result.dead_ends[0].approach, "passport.js");
        assert_eq!(result.decisions.len(), 1);
    }

    #[test]
    fn test_parse_response_with_code_fencing() {
        let json = r#"```json
        {"summary": "Fixed the bug", "interpreted_goal": "Debug and fix", "dead_ends": [], "decisions": []}
        ```"#;

        let result = parse_response(json).unwrap();
        assert_eq!(result.summary.as_deref(), Some("Fixed the bug"));
    }

    #[test]
    fn test_parse_response_empty_arrays() {
        let json = r#"{"summary": "Quick fix", "interpreted_goal": "Fix typo", "dead_ends": [], "decisions": []}"#;

        let result = parse_response(json).unwrap();
        assert!(result.dead_ends.is_empty());
        assert!(result.decisions.is_empty());
    }

    #[test]
    fn test_parse_response_invalid_json() {
        let result = parse_response("not json at all");
        assert!(result.is_err());
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world", 5), "hello");
    }

    #[test]
    fn test_summarize_json_input() {
        let input = serde_json::json!({"file_path": "/src/auth.rs", "content": "fn main() {}"});
        let result = summarize_json_input(&input);
        assert!(result.contains("file_path"));
        assert!(result.contains("content"));
    }

    #[test]
    fn test_summarize_intent_no_api_key() {
        // When no API key is available (env or config), summarize_intent should be a no-op.
        // Skip if global config has an API key since we can't isolate that in tests.
        std::env::remove_var("ANTHROPIC_API_KEY");
        if let Ok(config) = engram_core::config::GlobalConfig::load() {
            if config
                .settings
                .anthropic_api_key
                .as_ref()
                .is_some_and(|k| !k.is_empty())
            {
                return; // Global config has an API key; can't test the no-key path here
            }
        }
        let mut data = make_test_data();
        let original_summary = data.manifest.summary.clone();
        summarize_intent(&mut data).unwrap();
        assert_eq!(data.manifest.summary, original_summary);
    }

    fn make_test_data() -> EngramData {
        use engram_core::model::*;
        EngramData {
            manifest: Manifest {
                id: EngramId::new(),
                version: 1,
                created_at: chrono::Utc::now(),
                finished_at: None,
                agent: AgentInfo {
                    name: "test".into(),
                    model: None,
                    version: None,
                },
                git_commits: vec![],
                token_usage: TokenUsage::default(),
                summary: Some("original summary".into()),
                tags: vec![],
                capture_mode: CaptureMode::Import,
                source_hash: None,
            },
            intent: Intent {
                original_request: "do something".into(),
                interpreted_goal: None,
                summary: None,
                dead_ends: vec![],
                decisions: vec![],
            },
            transcript: Transcript::default(),
            operations: Operations::default(),
            lineage: Lineage::default(),
        }
    }
}
