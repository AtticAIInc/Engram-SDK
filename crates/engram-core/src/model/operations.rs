use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Structured list of all tool calls and file operations.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Operations {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolCall>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_changes: Vec<FileChange>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shell_commands: Vec<ShellCommand>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolCall {
    pub timestamp: DateTime<Utc>,
    pub tool_name: String,
    pub input: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub is_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FileChange {
    pub path: String,
    pub change_type: FileChangeType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines_added: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines_removed: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FileChangeType {
    Created,
    Modified,
    Deleted,
    Renamed { from: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ShellCommand {
    pub timestamp: DateTime<Utc>,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operations_serde_roundtrip() {
        let ops = Operations {
            tool_calls: vec![ToolCall {
                timestamp: Utc::now(),
                tool_name: "Write".into(),
                input: serde_json::json!({"path": "src/auth.rs"}),
                output_summary: Some("File created".into()),
                duration_ms: Some(150),
                is_error: false,
            }],
            file_changes: vec![FileChange {
                path: "src/auth.rs".into(),
                change_type: FileChangeType::Created,
                lines_added: Some(50),
                lines_removed: None,
            }],
            shell_commands: vec![ShellCommand {
                timestamp: Utc::now(),
                command: "cargo test".into(),
                exit_code: Some(0),
                duration_ms: Some(3000),
            }],
        };
        let json = serde_json::to_string_pretty(&ops).unwrap();
        let parsed: Operations = serde_json::from_str(&json).unwrap();
        assert_eq!(ops, parsed);
    }

    #[test]
    fn test_rename_variant() {
        let change = FileChange {
            path: "src/new_auth.rs".into(),
            change_type: FileChangeType::Renamed {
                from: "src/auth.rs".into(),
            },
            lines_added: None,
            lines_removed: None,
        };
        let json = serde_json::to_string(&change).unwrap();
        assert!(json.contains("renamed"));
        let parsed: FileChange = serde_json::from_str(&json).unwrap();
        assert_eq!(change, parsed);
    }
}
