use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// A single line of the transcript.jsonl file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TranscriptEntry {
    pub timestamp: DateTime<Utc>,
    pub role: Role,
    pub content: TranscriptContent,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum TranscriptContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "tool_use")]
    ToolUse {
        tool_name: String,
        tool_id: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_id: String,
        output: String,
        is_error: bool,
    },
    #[serde(rename = "thinking")]
    Thinking { text: String },
}

/// The full transcript, serialized as JSONL.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Transcript {
    pub entries: Vec<TranscriptEntry>,
}

impl Transcript {
    /// Serialize to JSONL bytes (one JSON object per line).
    pub fn to_jsonl(&self) -> Result<Vec<u8>, CoreError> {
        let mut buf = Vec::new();
        for entry in &self.entries {
            serde_json::to_writer(&mut buf, entry)?;
            buf.push(b'\n');
        }
        Ok(buf)
    }

    /// Deserialize from JSONL bytes.
    pub fn from_jsonl(data: &[u8]) -> Result<Self, CoreError> {
        let text = std::str::from_utf8(data).map_err(|e| CoreError::Parse(e.to_string()))?;
        let mut entries = Vec::new();
        for line in text.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let entry: TranscriptEntry =
                serde_json::from_str(line).map_err(CoreError::InvalidManifest)?;
            entries.push(entry);
        }
        Ok(Transcript { entries })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entries() -> Vec<TranscriptEntry> {
        vec![
            TranscriptEntry {
                timestamp: Utc::now(),
                role: Role::User,
                content: TranscriptContent::Text {
                    text: "Add OAuth2 authentication".into(),
                },
                token_count: None,
            },
            TranscriptEntry {
                timestamp: Utc::now(),
                role: Role::Assistant,
                content: TranscriptContent::Thinking {
                    text: "Let me think about this...".into(),
                },
                token_count: Some(50),
            },
            TranscriptEntry {
                timestamp: Utc::now(),
                role: Role::Assistant,
                content: TranscriptContent::ToolUse {
                    tool_name: "Write".into(),
                    tool_id: "toolu_123".into(),
                    input: serde_json::json!({"path": "src/auth.rs"}),
                },
                token_count: Some(100),
            },
            TranscriptEntry {
                timestamp: Utc::now(),
                role: Role::Tool,
                content: TranscriptContent::ToolResult {
                    tool_id: "toolu_123".into(),
                    output: "File written successfully".into(),
                    is_error: false,
                },
                token_count: None,
            },
        ]
    }

    #[test]
    fn test_jsonl_roundtrip() {
        let transcript = Transcript {
            entries: sample_entries(),
        };
        let jsonl = transcript.to_jsonl().unwrap();
        let parsed = Transcript::from_jsonl(&jsonl).unwrap();
        assert_eq!(transcript.entries.len(), parsed.entries.len());
        for (a, b) in transcript.entries.iter().zip(parsed.entries.iter()) {
            assert_eq!(a.role, b.role);
            assert_eq!(a.content, b.content);
            assert_eq!(a.token_count, b.token_count);
        }
    }

    #[test]
    fn test_empty_transcript() {
        let transcript = Transcript::default();
        let jsonl = transcript.to_jsonl().unwrap();
        let parsed = Transcript::from_jsonl(&jsonl).unwrap();
        assert!(parsed.entries.is_empty());
    }

    #[test]
    fn test_content_variants_serde() {
        let text = TranscriptContent::Text {
            text: "hello".into(),
        };
        let json = serde_json::to_string(&text).unwrap();
        assert!(json.contains("\"type\":\"text\""));

        let tool_use = TranscriptContent::ToolUse {
            tool_name: "Bash".into(),
            tool_id: "id1".into(),
            input: serde_json::json!({"command": "ls"}),
        };
        let json = serde_json::to_string(&tool_use).unwrap();
        assert!(json.contains("\"type\":\"tool_use\""));
    }
}
