use engram_core::model::*;
use engram_core::storage::GitStorage;

use super::extractor::extract_insights;
use crate::error::CaptureError;
use crate::pty::CapturedSession;

/// Builds an EngramData from a CapturedSession.
pub struct SessionBuilder {
    agent_info: AgentInfo,
    captured: CapturedSession,
    git_commits: Vec<String>,
    parent_engram: Option<EngramId>,
}

impl SessionBuilder {
    pub fn new(agent_info: AgentInfo, captured: CapturedSession) -> Self {
        Self {
            agent_info,
            captured,
            git_commits: Vec::new(),
            parent_engram: None,
        }
    }

    /// Set the git commits produced during this session.
    pub fn with_commits(mut self, commits: Vec<String>) -> Self {
        self.git_commits = commits;
        self
    }

    /// Set the parent engram (for chaining).
    pub fn with_parent(mut self, parent: EngramId) -> Self {
        self.parent_engram = Some(parent);
        self
    }

    /// Build the EngramData.
    pub fn build(self) -> EngramData {
        let id = EngramId::new();

        // Extract intent from the command + args
        let original_request = if self.captured.args.is_empty() {
            self.captured.command.clone()
        } else {
            format!("{} {}", self.captured.command, self.captured.args.join(" "))
        };

        let summary = if self.captured.file_changes.is_empty() {
            Some(format!(
                "Ran {} (exit code: {})",
                self.captured.command,
                self.captured
                    .exit_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "unknown".into())
            ))
        } else {
            Some(format!(
                "{} file(s) changed by {}",
                self.captured.file_changes.len(),
                self.captured.command
            ))
        };

        let manifest = Manifest {
            id,
            version: 1,
            created_at: self.captured.start_time,
            finished_at: Some(self.captured.end_time),
            agent: self.agent_info,
            git_commits: self.git_commits.clone(),
            token_usage: TokenUsage::default(), // PTY capture doesn't know token usage
            summary,
            tags: Vec::new(),
            capture_mode: CaptureMode::Wrapper,
            source_hash: None,
        };

        // Best-effort extraction of dead ends and decisions from raw output
        let insights = extract_insights(&self.captured.raw_output);

        let intent = Intent {
            original_request,
            interpreted_goal: None,
            summary: manifest.summary.clone(),
            dead_ends: insights.dead_ends,
            decisions: insights.decisions,
        };

        // Build transcript from raw output
        let transcript = Transcript {
            entries: vec![TranscriptEntry {
                timestamp: self.captured.start_time,
                role: Role::System,
                content: TranscriptContent::Text {
                    text: format!(
                        "PTY session: {} {}",
                        self.captured.command,
                        self.captured.args.join(" ")
                    ),
                },
                token_count: None,
            }],
        };

        let operations = Operations {
            tool_calls: Vec::new(),
            file_changes: self.captured.file_changes.clone(),
            shell_commands: vec![ShellCommand {
                timestamp: self.captured.start_time,
                command: format!("{} {}", self.captured.command, self.captured.args.join(" ")),
                exit_code: self.captured.exit_code.map(|c| c as i32),
                duration_ms: Some(
                    (self.captured.end_time - self.captured.start_time).num_milliseconds() as u64,
                ),
            }],
        };

        let lineage = Lineage {
            parent_engram: self.parent_engram,
            git_commits: self.git_commits,
            branch: None, // Could detect from git HEAD
            ..Default::default()
        };

        EngramData {
            manifest,
            intent,
            transcript,
            operations,
            lineage,
        }
    }

    /// Build and immediately store in Git.
    pub fn build_and_store(self, storage: &GitStorage) -> Result<EngramId, CaptureError> {
        let data = self.build();
        let id = storage.create(&data)?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn mock_captured_session() -> CapturedSession {
        CapturedSession {
            raw_output: b"hello world\n".to_vec(),
            start_time: Utc::now(),
            end_time: Utc::now(),
            exit_code: Some(0),
            file_changes: vec![FileChange {
                path: "src/main.rs".into(),
                change_type: FileChangeType::Modified,
                lines_added: None,
                lines_removed: None,
            }],
            command: "claude".into(),
            args: vec!["add auth".into()],
        }
    }

    #[test]
    fn test_session_builder() {
        let agent = AgentInfo {
            name: "claude-code".into(),
            model: Some("claude-sonnet-4-5".into()),
            version: None,
        };
        let captured = mock_captured_session();

        let data = SessionBuilder::new(agent, captured)
            .with_commits(vec!["abc123".into()])
            .build();

        assert_eq!(data.manifest.agent.name, "claude-code");
        assert_eq!(data.manifest.capture_mode, CaptureMode::Wrapper);
        assert_eq!(data.intent.original_request, "claude add auth");
        assert_eq!(data.operations.file_changes.len(), 1);
        assert_eq!(data.operations.shell_commands.len(), 1);
        assert_eq!(data.lineage.git_commits, vec!["abc123".to_string()]);
    }
}
