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

    let intent = Intent {
        original_request: if original_request.is_empty() {
            "Imported Claude Code session".into()
        } else {
            original_request
        },
        interpreted_goal: None,
        summary: manifest.summary.clone(),
        dead_ends: Vec::new(),
        decisions: Vec::new(),
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
    path.to_string_lossy().replace('/', "-")
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
}
