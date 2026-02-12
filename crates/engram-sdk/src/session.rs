use chrono::Utc;

use engram_core::model::*;
use engram_core::storage::GitStorage;

/// A fluent session builder for creating engrams programmatically.
///
/// Use this from agent code or wrappers to capture reasoning, tool calls,
/// file changes, dead ends, and token economics, then store as a Git engram.
pub struct EngramSession {
    agent: AgentInfo,
    transcript: Vec<TranscriptEntry>,
    tool_calls: Vec<ToolCall>,
    file_changes: Vec<FileChange>,
    shell_commands: Vec<ShellCommand>,
    dead_ends: Vec<DeadEnd>,
    decisions: Vec<Decision>,
    token_usage: TokenUsage,
    original_request: Option<String>,
    summary: Option<String>,
    tags: Vec<String>,
    parent: Option<EngramId>,
    started_at: chrono::DateTime<Utc>,
}

impl EngramSession {
    /// Begin a new session for a given agent and optional model name.
    pub fn begin(agent_name: &str, model: Option<&str>) -> Self {
        Self {
            agent: AgentInfo {
                name: agent_name.to_string(),
                model: model.map(String::from),
                version: None,
            },
            transcript: Vec::new(),
            tool_calls: Vec::new(),
            file_changes: Vec::new(),
            shell_commands: Vec::new(),
            dead_ends: Vec::new(),
            decisions: Vec::new(),
            token_usage: TokenUsage::default(),
            original_request: None,
            summary: None,
            tags: Vec::new(),
            parent: None,
            started_at: Utc::now(),
        }
    }

    /// Set the agent version.
    pub fn agent_version(&mut self, version: &str) -> &mut Self {
        self.agent.version = Some(version.to_string());
        self
    }

    /// Set the parent engram (for chaining sessions).
    pub fn parent(&mut self, parent_id: EngramId) -> &mut Self {
        self.parent = Some(parent_id);
        self
    }

    /// Set a summary for this session.
    pub fn set_summary(&mut self, summary: &str) -> &mut Self {
        self.summary = Some(summary.to_string());
        self
    }

    /// Add a tag.
    pub fn tag(&mut self, tag: &str) -> &mut Self {
        self.tags.push(tag.to_string());
        self
    }

    /// Log a message (user, assistant, system, or tool).
    pub fn log_message(&mut self, role: &str, content: &str) -> &mut Self {
        let role = match role {
            "user" => Role::User,
            "assistant" => Role::Assistant,
            "system" => Role::System,
            "tool" => Role::Tool,
            _ => Role::System,
        };

        // First user message becomes the original request
        if role == Role::User && self.original_request.is_none() {
            self.original_request = Some(content.to_string());
        }

        self.transcript.push(TranscriptEntry {
            timestamp: Utc::now(),
            role,
            content: TranscriptContent::Text {
                text: content.to_string(),
            },
            token_count: None,
        });
        self
    }

    /// Log a tool call with its name, input, and optional output summary.
    pub fn log_tool_call(
        &mut self,
        tool_name: &str,
        input: &str,
        output_summary: Option<&str>,
    ) -> &mut Self {
        let input_value: serde_json::Value =
            serde_json::from_str(input).unwrap_or(serde_json::Value::String(input.to_string()));

        self.tool_calls.push(ToolCall {
            timestamp: Utc::now(),
            tool_name: tool_name.to_string(),
            input: input_value,
            output_summary: output_summary.map(String::from),
            duration_ms: None,
            is_error: false,
        });
        self
    }

    /// Log a file change.
    pub fn log_file_change(&mut self, path: &str, change_type: &str) -> &mut Self {
        let ct = match change_type {
            "created" | "create" | "new" => FileChangeType::Created,
            "deleted" | "delete" | "removed" => FileChangeType::Deleted,
            _ => FileChangeType::Modified,
        };
        self.file_changes.push(FileChange {
            path: path.to_string(),
            change_type: ct,
            lines_added: None,
            lines_removed: None,
        });
        self
    }

    /// Log a shell command execution.
    pub fn log_shell_command(
        &mut self,
        command: &str,
        exit_code: Option<i32>,
        duration_ms: Option<u64>,
    ) -> &mut Self {
        self.shell_commands.push(ShellCommand {
            timestamp: Utc::now(),
            command: command.to_string(),
            exit_code,
            duration_ms,
        });
        self
    }

    /// Log a rejected approach (dead end).
    pub fn log_rejection(&mut self, approach: &str, reason: &str) -> &mut Self {
        self.dead_ends.push(DeadEnd {
            approach: approach.to_string(),
            reason: reason.to_string(),
        });
        self
    }

    /// Log a decision made during the session.
    pub fn log_decision(&mut self, description: &str, rationale: &str) -> &mut Self {
        self.decisions.push(Decision {
            description: description.to_string(),
            rationale: rationale.to_string(),
        });
        self
    }

    /// Add token usage. Accumulates across multiple calls.
    pub fn add_tokens(
        &mut self,
        input_tokens: u64,
        output_tokens: u64,
        cost_usd: Option<f64>,
    ) -> &mut Self {
        self.token_usage.input_tokens += input_tokens;
        self.token_usage.output_tokens += output_tokens;
        self.token_usage.total_tokens += input_tokens + output_tokens;
        if let Some(cost) = cost_usd {
            *self.token_usage.cost_usd.get_or_insert(0.0) += cost;
        }
        self
    }

    /// Finalize and store the engram in Git.
    ///
    /// - `git_sha`: Optional commit SHA to associate with this engram.
    /// - `summary`: Optional summary (overrides auto-generated one).
    ///
    /// Returns the EngramId on success.
    pub fn commit(
        self,
        git_sha: Option<&str>,
        summary: Option<&str>,
    ) -> Result<EngramId, engram_core::error::CoreError> {
        let storage = GitStorage::discover()?;
        self.commit_to(&storage, git_sha, summary)
    }

    /// Finalize and store in a specific GitStorage instance.
    pub fn commit_to(
        self,
        storage: &GitStorage,
        git_sha: Option<&str>,
        summary: Option<&str>,
    ) -> Result<EngramId, engram_core::error::CoreError> {
        let data = self.build(git_sha, summary);
        storage.create(&data)
    }

    /// Build the EngramData without storing it.
    pub fn build(self, git_sha: Option<&str>, summary: Option<&str>) -> EngramData {
        let id = EngramId::new();
        let finished_at = Utc::now();

        let final_summary = summary
            .map(String::from)
            .or(self.summary)
            .or(self.original_request.clone());

        let git_commits = git_sha.map(|s| vec![s.to_string()]).unwrap_or_default();

        let manifest = Manifest {
            id,
            version: 1,
            created_at: self.started_at,
            finished_at: Some(finished_at),
            agent: self.agent,
            git_commits: git_commits.clone(),
            token_usage: self.token_usage,
            summary: final_summary,
            tags: self.tags,
            capture_mode: CaptureMode::Sdk,
            source_hash: None,
        };

        let intent = Intent {
            original_request: self
                .original_request
                .unwrap_or_else(|| "SDK session".to_string()),
            interpreted_goal: None,
            summary: manifest.summary.clone(),
            dead_ends: self.dead_ends,
            decisions: self.decisions,
        };

        let transcript = Transcript {
            entries: self.transcript,
        };

        let operations = Operations {
            tool_calls: self.tool_calls,
            file_changes: self.file_changes,
            shell_commands: self.shell_commands,
        };

        let lineage = Lineage {
            parent_engram: self.parent,
            git_commits,
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_build() {
        let mut session = EngramSession::begin("test-agent", Some("gpt-4"));
        session
            .log_message("user", "Add auth to the API")
            .log_message("assistant", "I'll add JWT auth.")
            .log_tool_call(
                "write_file",
                r#"{"path":"src/auth.rs"}"#,
                Some("Created auth module"),
            )
            .log_file_change("src/auth.rs", "created")
            .log_rejection("Session auth", "Too stateful")
            .log_decision("Use JWT", "Stateless, works with load balancers")
            .add_tokens(1500, 800, Some(0.02))
            .tag("auth");

        let data = session.build(Some("abc123"), Some("Add JWT authentication"));

        assert_eq!(data.manifest.agent.name, "test-agent");
        assert_eq!(data.manifest.agent.model, Some("gpt-4".into()));
        assert_eq!(data.manifest.capture_mode, CaptureMode::Sdk);
        assert_eq!(data.manifest.summary, Some("Add JWT authentication".into()));
        assert_eq!(data.manifest.token_usage.input_tokens, 1500);
        assert_eq!(data.manifest.token_usage.output_tokens, 800);
        assert_eq!(data.manifest.token_usage.total_tokens, 2300);
        assert_eq!(data.manifest.token_usage.cost_usd, Some(0.02));
        assert_eq!(data.manifest.tags, vec!["auth"]);

        assert_eq!(data.intent.original_request, "Add auth to the API");
        assert_eq!(data.intent.dead_ends.len(), 1);
        assert_eq!(data.intent.decisions.len(), 1);

        assert_eq!(data.transcript.entries.len(), 2);
        assert_eq!(data.operations.tool_calls.len(), 1);
        assert_eq!(data.operations.file_changes.len(), 1);
        assert_eq!(
            data.operations.file_changes[0].change_type,
            FileChangeType::Created
        );

        assert_eq!(data.lineage.git_commits, vec!["abc123"]);
    }

    #[test]
    fn test_session_store() {
        // Create a temp git repo and test storage round-trip
        let dir = tempfile::tempdir().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();

        // Configure git user for the test repo
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test User").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();

        // Create initial commit so the repo is valid
        let sig = repo.signature().unwrap();
        let tree_oid = repo.treebuilder(None).unwrap().write().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
            .unwrap();

        // Initialize engram
        let storage = GitStorage::open(dir.path()).unwrap();
        storage.init().unwrap();

        // Create session via SDK
        let mut session = EngramSession::begin("test-agent", Some("claude-sonnet"));
        session
            .log_message("user", "Fix the login bug")
            .log_message("assistant", "I found the issue in auth.rs")
            .add_tokens(500, 200, Some(0.005));

        let id = session
            .commit_to(&storage, None, Some("Fixed login bug"))
            .unwrap();

        // Read back
        let data = storage.read(id.as_str()).unwrap();
        assert_eq!(data.manifest.agent.name, "test-agent");
        assert_eq!(data.manifest.summary, Some("Fixed login bug".into()));
        assert_eq!(data.intent.original_request, "Fix the login bug");
        assert_eq!(data.transcript.entries.len(), 2);
    }

    #[test]
    fn test_accumulate_tokens() {
        let mut session = EngramSession::begin("test", None);
        session
            .add_tokens(100, 50, Some(0.01))
            .add_tokens(200, 100, Some(0.02));

        let data = session.build(None, None);
        assert_eq!(data.manifest.token_usage.input_tokens, 300);
        assert_eq!(data.manifest.token_usage.output_tokens, 150);
        assert_eq!(data.manifest.token_usage.total_tokens, 450);
        let cost = data.manifest.token_usage.cost_usd.unwrap();
        assert!((cost - 0.03).abs() < 1e-10);
    }
}
