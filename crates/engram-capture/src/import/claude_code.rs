use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use engram_core::model::*;

use crate::error::CaptureError;

/// Import a Claude Code session from a JSONL file.
pub struct ClaudeCodeImporter;

impl ClaudeCodeImporter {
    /// Discover the Claude Code projects directory.
    pub fn projects_dir() -> Option<PathBuf> {
        dirs_for_claude_projects()
    }

    /// Discover all session files for a project.
    pub fn discover_sessions(project_path: &Path) -> Result<Vec<PathBuf>, CaptureError> {
        let project_key = path_to_claude_key(project_path);
        let projects_dir = Self::projects_dir()
            .ok_or_else(|| CaptureError::Import("Cannot find ~/.claude/projects".into()))?;
        let project_dir = projects_dir.join(&project_key);

        if !project_dir.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();
        for entry in std::fs::read_dir(&project_dir).map_err(CaptureError::Io)? {
            let entry = entry.map_err(CaptureError::Io)?;
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "jsonl") && path.is_file() {
                sessions.push(path);
            }
        }
        sessions.sort();
        Ok(sessions)
    }

    /// Import a single session JSONL file into an EngramData.
    pub fn import_session(path: &Path) -> Result<EngramData, CaptureError> {
        let content = std::fs::read_to_string(path).map_err(CaptureError::Io)?;
        let source_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
        let mut data = parse_claude_code_session(&content)?;
        data.manifest.source_hash = Some(source_hash);
        Ok(data)
    }
}

/// Internal Claude Code JSONL entry.
#[derive(Debug, Deserialize)]
struct ClaudeEntry {
    #[serde(rename = "type")]
    entry_type: String,
    #[serde(default)]
    _uuid: Option<String>,
    #[serde(default, rename = "parentUuid")]
    _parent_uuid: Option<String>,
    #[serde(default)]
    timestamp: Option<String>,
    #[serde(default)]
    message: Option<ClaudeMessage>,
    #[serde(default, rename = "isSidechain")]
    is_sidechain: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct ClaudeMessage {
    role: String,
    #[serde(default)]
    content: serde_json::Value,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<ClaudeUsage>,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    #[serde(default)]
    input_tokens: Option<u64>,
    #[serde(default)]
    output_tokens: Option<u64>,
    #[serde(default)]
    cache_creation_input_tokens: Option<u64>,
    #[serde(default)]
    cache_read_input_tokens: Option<u64>,
}

fn parse_claude_code_session(content: &str) -> Result<EngramData, CaptureError> {
    let mut entries = Vec::new();
    let mut first_timestamp: Option<DateTime<Utc>> = None;
    let mut last_timestamp: Option<DateTime<Utc>> = None;
    let mut model_name: Option<String> = None;
    let mut agent_version: Option<String> = None;
    let mut token_usage = TokenUsage::default();
    let mut transcript_entries = Vec::new();
    let mut tool_calls = Vec::new();
    let mut file_changes = Vec::new();
    let mut original_request = String::new();
    let mut reasoning_text = String::new(); // Collect assistant text for insight extraction
    let mut first_assistant_text: Option<String> = None; // For interpreted_goal

    // Parse all lines
    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<ClaudeEntry>(line) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                tracing::debug!("Skipping unparseable JSONL line: {e}");
            }
        }
    }

    for entry in &entries {
        // Skip non-message entries
        if !matches!(entry.entry_type.as_str(), "user" | "assistant") {
            continue;
        }

        // Skip sidechain messages
        if entry.is_sidechain == Some(true) {
            continue;
        }

        // Parse timestamp
        let ts = entry
            .timestamp
            .as_deref()
            .and_then(|t| t.parse::<DateTime<Utc>>().ok());

        if let Some(ts) = ts {
            if first_timestamp.is_none() {
                first_timestamp = Some(ts);
            }
            last_timestamp = Some(ts);
        }

        let Some(msg) = &entry.message else {
            continue;
        };

        // Extract model and version from first assistant message
        if msg.role == "assistant" && model_name.is_none() {
            model_name = msg.model.clone();
        }

        // Accumulate token usage
        if let Some(usage) = &msg.usage {
            token_usage.input_tokens += usage.input_tokens.unwrap_or(0);
            token_usage.output_tokens += usage.output_tokens.unwrap_or(0);
            token_usage.cache_read_tokens += usage.cache_read_input_tokens.unwrap_or(0);
            token_usage.cache_write_tokens += usage.cache_creation_input_tokens.unwrap_or(0);
        }

        // Process message content
        let role = match msg.role.as_str() {
            "user" => Role::User,
            "assistant" => Role::Assistant,
            _ => continue,
        };

        // Content can be a string or array of content blocks
        match &msg.content {
            serde_json::Value::String(text) => {
                if role == Role::User && original_request.is_empty() {
                    original_request = text.clone();
                }
                if role == Role::Assistant {
                    reasoning_text.push_str(text);
                    reasoning_text.push('\n');
                    if first_assistant_text.is_none() && !text.trim().is_empty() {
                        first_assistant_text = Some(text.clone());
                    }
                }
                transcript_entries.push(TranscriptEntry {
                    timestamp: ts.unwrap_or_else(Utc::now),
                    role,
                    content: TranscriptContent::Text { text: text.clone() },
                    token_count: None,
                });
            }
            serde_json::Value::Array(blocks) => {
                for block in blocks {
                    let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("");

                    match block_type {
                        "text" => {
                            let text = block
                                .get("text")
                                .and_then(|t| t.as_str())
                                .unwrap_or("")
                                .to_string();

                            if role == Role::User && original_request.is_empty() {
                                original_request = text.clone();
                            }
                            if role == Role::Assistant {
                                reasoning_text.push_str(&text);
                                reasoning_text.push('\n');
                                if first_assistant_text.is_none() && !text.trim().is_empty() {
                                    first_assistant_text = Some(text.clone());
                                }
                            }

                            transcript_entries.push(TranscriptEntry {
                                timestamp: ts.unwrap_or_else(Utc::now),
                                role: role.clone(),
                                content: TranscriptContent::Text { text },
                                token_count: None,
                            });
                        }
                        "tool_use" => {
                            let tool_name = block
                                .get("name")
                                .and_then(|n| n.as_str())
                                .unwrap_or("unknown")
                                .to_string();
                            let tool_id = block
                                .get("id")
                                .and_then(|n| n.as_str())
                                .unwrap_or("")
                                .to_string();
                            let input = block
                                .get("input")
                                .cloned()
                                .unwrap_or(serde_json::Value::Null);

                            // Track file operations
                            if matches!(tool_name.as_str(), "Write" | "Edit" | "NotebookEdit") {
                                if let Some(path) = input.get("file_path").and_then(|p| p.as_str())
                                {
                                    let change_type = if tool_name == "Write" {
                                        FileChangeType::Created
                                    } else {
                                        FileChangeType::Modified
                                    };
                                    if !file_changes.iter().any(|fc: &FileChange| fc.path == path) {
                                        file_changes.push(FileChange {
                                            path: path.to_string(),
                                            change_type,
                                            lines_added: None,
                                            lines_removed: None,
                                        });
                                    }
                                }
                            }

                            tool_calls.push(ToolCall {
                                timestamp: ts.unwrap_or_else(Utc::now),
                                tool_name: tool_name.clone(),
                                input: input.clone(),
                                output_summary: None,
                                duration_ms: None,
                                is_error: false,
                            });

                            transcript_entries.push(TranscriptEntry {
                                timestamp: ts.unwrap_or_else(Utc::now),
                                role: role.clone(),
                                content: TranscriptContent::ToolUse {
                                    tool_name,
                                    tool_id,
                                    input,
                                },
                                token_count: None,
                            });
                        }
                        "tool_result" => {
                            let tool_id = block
                                .get("tool_use_id")
                                .and_then(|n| n.as_str())
                                .unwrap_or("")
                                .to_string();
                            let output = block
                                .get("content")
                                .and_then(|c| c.as_str())
                                .unwrap_or("")
                                .to_string();
                            let is_error = block
                                .get("is_error")
                                .and_then(|e| e.as_bool())
                                .unwrap_or(false);

                            transcript_entries.push(TranscriptEntry {
                                timestamp: ts.unwrap_or_else(Utc::now),
                                role: Role::Tool,
                                content: TranscriptContent::ToolResult {
                                    tool_id,
                                    output,
                                    is_error,
                                },
                                token_count: None,
                            });
                        }
                        "thinking" => {
                            let text = block
                                .get("thinking")
                                .and_then(|t| t.as_str())
                                .unwrap_or("")
                                .to_string();
                            if !text.is_empty() {
                                reasoning_text.push_str(&text);
                                reasoning_text.push('\n');
                                transcript_entries.push(TranscriptEntry {
                                    timestamp: ts.unwrap_or_else(Utc::now),
                                    role: role.clone(),
                                    content: TranscriptContent::Thinking { text },
                                    token_count: None,
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    // Try to get version from the first JSON line
    if let Some(v) = content
        .lines()
        .next()
        .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .and_then(|v| v.get("version").and_then(|v| v.as_str()).map(String::from))
    {
        agent_version = Some(v);
    }

    token_usage.total_tokens = token_usage.input_tokens
        + token_usage.output_tokens
        + token_usage.cache_read_tokens
        + token_usage.cache_write_tokens;

    let now = Utc::now();
    let created_at = first_timestamp.unwrap_or(now);
    let finished_at = last_timestamp.unwrap_or(now);

    let id = EngramId::new();

    let manifest = Manifest {
        id,
        version: 1,
        created_at,
        finished_at: Some(finished_at),
        agent: AgentInfo {
            name: "claude-code".into(),
            model: model_name,
            version: agent_version,
        },
        git_commits: Vec::new(),
        token_usage,
        summary: if original_request.len() > 100 {
            Some(format!("{}...", &original_request[..100]))
        } else if original_request.is_empty() {
            Some("Imported Claude Code session".into())
        } else {
            Some(original_request.clone())
        },
        tags: Vec::new(),
        capture_mode: CaptureMode::Import,
        source_hash: None,
    };

    // Extract dead ends and decisions from reasoning text
    let insights = crate::session::extractor::extract_insights(reasoning_text.as_bytes());

    // Derive interpreted_goal from the first assistant response (truncated)
    let interpreted_goal = first_assistant_text.map(|t| {
        let trimmed = t.trim();
        if trimmed.len() > 200 {
            format!("{}...", &trimmed[..200])
        } else {
            trimmed.to_string()
        }
    });

    let intent = Intent {
        original_request: if original_request.is_empty() {
            "Imported Claude Code session".into()
        } else {
            original_request
        },
        interpreted_goal,
        summary: manifest.summary.clone(),
        dead_ends: insights.dead_ends,
        decisions: insights.decisions,
    };

    let operations = Operations {
        tool_calls,
        file_changes,
        shell_commands: Vec::new(),
    };

    Ok(EngramData {
        manifest,
        intent,
        transcript: Transcript {
            entries: transcript_entries,
        },
        operations,
        lineage: Lineage::default(),
    })
}

/// Convert a filesystem path to Claude Code's project key format.
/// /Users/sjonas/myproject -> -Users-sjonas-myproject
fn path_to_claude_key(path: &Path) -> String {
    let s = path.to_string_lossy();
    let s = s.trim_end_matches('/');
    s.replace('/', "-")
}

fn dirs_for_claude_projects() -> Option<PathBuf> {
    // ~/.claude/projects/
    home_dir().map(|h| h.join(".claude").join("projects"))
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_claude_key() {
        assert_eq!(
            path_to_claude_key(Path::new("/Users/sjonas/myproject")),
            "-Users-sjonas-myproject"
        );
        // Trailing slash should be stripped
        assert_eq!(
            path_to_claude_key(Path::new("/Users/sjonas/myproject/")),
            "-Users-sjonas-myproject"
        );
    }

    #[test]
    fn test_parse_simple_session() {
        let jsonl = r#"{"type":"user","uuid":"uuid1","timestamp":"2026-01-15T10:00:00Z","message":{"role":"user","content":"Add a hello world function"},"version":"2.1.39"}
{"type":"assistant","uuid":"uuid2","parentUuid":"uuid1","timestamp":"2026-01-15T10:00:05Z","message":{"role":"assistant","content":[{"type":"text","text":"I'll add a hello world function."},{"type":"tool_use","id":"toolu_1","name":"Write","input":{"file_path":"src/main.rs","content":"fn hello() { println!(\"Hello!\"); }"}}],"model":"claude-sonnet-4-5","usage":{"input_tokens":1000,"output_tokens":200}}}"#;

        let data = parse_claude_code_session(jsonl).unwrap();
        assert_eq!(data.manifest.agent.name, "claude-code");
        assert_eq!(data.manifest.agent.model, Some("claude-sonnet-4-5".into()));
        assert_eq!(data.manifest.token_usage.input_tokens, 1000);
        assert_eq!(data.manifest.token_usage.output_tokens, 200);
        assert_eq!(data.intent.original_request, "Add a hello world function");
        assert!(!data.transcript.entries.is_empty());
        assert_eq!(data.operations.tool_calls.len(), 1);
        assert_eq!(data.operations.tool_calls[0].tool_name, "Write");
        assert_eq!(data.operations.file_changes.len(), 1);
        assert_eq!(data.operations.file_changes[0].path, "src/main.rs");
    }

    #[test]
    fn test_parse_session_with_tool_result() {
        let jsonl = r#"{"type":"user","uuid":"u1","timestamp":"2026-01-15T10:00:00Z","message":{"role":"user","content":"Run tests"}}
{"type":"assistant","uuid":"a1","parentUuid":"u1","timestamp":"2026-01-15T10:00:02Z","message":{"role":"assistant","content":[{"type":"tool_use","id":"toolu_1","name":"Bash","input":{"command":"cargo test"}}],"model":"claude-sonnet-4-5","usage":{"input_tokens":500,"output_tokens":100}}}
{"type":"user","uuid":"u2","timestamp":"2026-01-15T10:00:10Z","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"toolu_1","content":"test result: ok. 5 passed","is_error":false}]}}"#;

        let data = parse_claude_code_session(jsonl).unwrap();
        assert_eq!(data.intent.original_request, "Run tests");
        assert_eq!(data.operations.tool_calls.len(), 1);

        // Should have 3 transcript entries: user text, tool_use, tool_result
        assert_eq!(data.transcript.entries.len(), 3);
    }

    #[test]
    fn test_parse_empty_session() {
        let data = parse_claude_code_session("").unwrap();
        assert_eq!(data.manifest.agent.name, "claude-code");
        assert!(data.transcript.entries.is_empty());
    }

    #[test]
    fn test_parse_session_extracts_dead_ends() {
        // Session where the assistant mentions a dead end
        let jsonl = r#"{"type":"user","uuid":"u1","timestamp":"2026-01-15T10:00:00Z","message":{"role":"user","content":"Add authentication"},"version":"2.1.39"}
{"type":"assistant","uuid":"a1","parentUuid":"u1","timestamp":"2026-01-15T10:00:05Z","message":{"role":"assistant","content":"tried passport.js but middleware conflict with existing stack","model":"claude-sonnet-4-5","usage":{"input_tokens":100,"output_tokens":50}}}"#;

        let data = parse_claude_code_session(jsonl).unwrap();
        assert!(!data.intent.dead_ends.is_empty());
        assert_eq!(data.intent.dead_ends[0].approach, "passport.js");
        assert!(data.intent.dead_ends[0]
            .reason
            .contains("middleware conflict"));
    }

    #[test]
    fn test_parse_session_extracts_decisions() {
        let jsonl = r#"{"type":"user","uuid":"u1","timestamp":"2026-01-15T10:00:00Z","message":{"role":"user","content":"Add auth"},"version":"2.1.39"}
{"type":"assistant","uuid":"a1","parentUuid":"u1","timestamp":"2026-01-15T10:00:05Z","message":{"role":"assistant","content":"decided to use custom middleware because full control over auth flow","model":"claude-sonnet-4-5","usage":{"input_tokens":100,"output_tokens":50}}}"#;

        let data = parse_claude_code_session(jsonl).unwrap();
        assert!(!data.intent.decisions.is_empty());
        assert_eq!(
            data.intent.decisions[0].description,
            "use custom middleware"
        );
    }

    #[test]
    fn test_parse_session_extracts_interpreted_goal() {
        let jsonl = r#"{"type":"user","uuid":"u1","timestamp":"2026-01-15T10:00:00Z","message":{"role":"user","content":"Add auth"},"version":"2.1.39"}
{"type":"assistant","uuid":"a1","parentUuid":"u1","timestamp":"2026-01-15T10:00:05Z","message":{"role":"assistant","content":"I'll implement OAuth2 authentication with PKCE for the SPA.","model":"claude-sonnet-4-5","usage":{"input_tokens":100,"output_tokens":50}}}"#;

        let data = parse_claude_code_session(jsonl).unwrap();
        assert_eq!(
            data.intent.interpreted_goal,
            Some("I'll implement OAuth2 authentication with PKCE for the SPA.".into())
        );
    }

    #[test]
    fn test_parse_session_truncates_long_interpreted_goal() {
        let long_text = "A".repeat(300);
        let jsonl = format!(
            r#"{{"type":"user","uuid":"u1","timestamp":"2026-01-15T10:00:00Z","message":{{"role":"user","content":"Do something"}},"version":"2.1.39"}}
{{"type":"assistant","uuid":"a1","parentUuid":"u1","timestamp":"2026-01-15T10:00:05Z","message":{{"role":"assistant","content":"{}","model":"claude-sonnet-4-5","usage":{{"input_tokens":100,"output_tokens":50}}}}}}"#,
            long_text
        );

        let data = parse_claude_code_session(&jsonl).unwrap();
        let goal = data.intent.interpreted_goal.unwrap();
        assert!(goal.len() <= 203); // 200 chars + "..."
        assert!(goal.ends_with("..."));
    }

    #[test]
    fn test_parse_session_thinking_blocks_feed_insights() {
        // Thinking blocks should also feed into insight extraction
        let jsonl = r#"{"type":"user","uuid":"u1","timestamp":"2026-01-15T10:00:00Z","message":{"role":"user","content":"Fix the bug"},"version":"2.1.39"}
{"type":"assistant","uuid":"a1","parentUuid":"u1","timestamp":"2026-01-15T10:00:05Z","message":{"role":"assistant","content":[{"type":"thinking","thinking":"tried using regex but performance was terrible for large inputs"},{"type":"text","text":"Let me fix this bug."}],"model":"claude-sonnet-4-5","usage":{"input_tokens":100,"output_tokens":50}}}"#;

        let data = parse_claude_code_session(jsonl).unwrap();
        assert!(!data.intent.dead_ends.is_empty());
        assert_eq!(data.intent.dead_ends[0].approach, "using regex");
    }

    #[test]
    fn test_parse_session_skips_sidechain() {
        let jsonl = r#"{"type":"user","uuid":"u1","timestamp":"2026-01-15T10:00:00Z","message":{"role":"user","content":"Add auth"},"version":"2.1.39"}
{"type":"assistant","uuid":"a2","parentUuid":"u1","timestamp":"2026-01-15T10:00:05Z","isSidechain":true,"message":{"role":"assistant","content":"This is sidechain","model":"claude-sonnet-4-5","usage":{"input_tokens":100,"output_tokens":50}}}
{"type":"assistant","uuid":"a1","parentUuid":"u1","timestamp":"2026-01-15T10:00:05Z","message":{"role":"assistant","content":"Main response","model":"claude-sonnet-4-5","usage":{"input_tokens":200,"output_tokens":100}}}"#;

        let data = parse_claude_code_session(jsonl).unwrap();
        // Sidechain tokens should be excluded
        assert_eq!(data.manifest.token_usage.input_tokens, 200);
        assert_eq!(data.manifest.token_usage.output_tokens, 100);
    }

    #[test]
    fn test_parse_session_multiple_file_operations() {
        let jsonl = r#"{"type":"user","uuid":"u1","timestamp":"2026-01-15T10:00:00Z","message":{"role":"user","content":"Add files"}}
{"type":"assistant","uuid":"a1","parentUuid":"u1","timestamp":"2026-01-15T10:00:05Z","message":{"role":"assistant","content":[{"type":"tool_use","id":"t1","name":"Write","input":{"file_path":"src/new.rs","content":"// new file"}},{"type":"tool_use","id":"t2","name":"Edit","input":{"file_path":"src/main.rs","old_string":"old","new_string":"new"}},{"type":"tool_use","id":"t3","name":"Write","input":{"file_path":"src/new.rs","content":"// updated"}}],"model":"claude-sonnet-4-5","usage":{"input_tokens":500,"output_tokens":200}}}"#;

        let data = parse_claude_code_session(jsonl).unwrap();
        // src/new.rs should appear only once (deduped)
        assert_eq!(data.operations.file_changes.len(), 2);
        assert!(data
            .operations
            .file_changes
            .iter()
            .any(|fc| fc.path == "src/new.rs"));
        assert!(data
            .operations
            .file_changes
            .iter()
            .any(|fc| fc.path == "src/main.rs"));
        // Write -> Created, Edit -> Modified
        let new_rs = data
            .operations
            .file_changes
            .iter()
            .find(|fc| fc.path == "src/new.rs")
            .unwrap();
        assert!(matches!(new_rs.change_type, FileChangeType::Created));
        let main_rs = data
            .operations
            .file_changes
            .iter()
            .find(|fc| fc.path == "src/main.rs")
            .unwrap();
        assert!(matches!(main_rs.change_type, FileChangeType::Modified));
    }

    #[test]
    fn test_parse_session_source_hash() {
        let jsonl = r#"{"type":"user","uuid":"u1","timestamp":"2026-01-15T10:00:00Z","message":{"role":"user","content":"test"}}"#;
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), jsonl).unwrap();

        let data = ClaudeCodeImporter::import_session(tmp.path()).unwrap();
        assert!(data.manifest.source_hash.is_some());
        // Hash should be consistent
        let data2 = ClaudeCodeImporter::import_session(tmp.path()).unwrap();
        assert_eq!(data.manifest.source_hash, data2.manifest.source_hash);
    }
}
