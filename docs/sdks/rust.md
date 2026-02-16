# Rust SDK

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
engram-sdk = "0.1"
```

## Quick Start

```rust
use engram_sdk::EngramSession;

let mut session = EngramSession::begin("my-agent", Some("claude-sonnet-4-5"));
session.log_message("user", "Add OAuth2 authentication");
session.log_message("assistant", "Implementing OAuth2 with PKCE...");
session.log_tool_call("write_file", r#"{"path": "src/auth.rs"}"#, Some("Created auth module"));
session.log_file_change("src/auth.rs", "created");
session.log_rejection("passport.js", "Middleware conflict with existing stack");
session.log_decision("Use JWT", "Stateless, works with load balancers");
session.add_tokens(1500, 800, Some(0.02));

let id = session.commit(Some("abc123"), Some("Implemented OAuth2 with PKCE")).unwrap();
println!("Stored engram: {}", id.as_str());
```

## Session Lifecycle

1. **Begin** -- Create a session with agent name and optional model
2. **Log** -- Record messages, tool calls, file changes, rejections, decisions, and tokens
3. **Commit** -- Store the engram in Git (auto-discovers the repository)

All `log_*` and `add_tokens` methods return `&mut Self` for chaining:

```rust
session
    .log_message("user", "Fix the bug")
    .log_message("assistant", "Found the issue")
    .log_file_change("src/fix.rs", "modified")
    .add_tokens(500, 200, Some(0.005));
```

## API Reference

### EngramSession

| Method | Signature | Description |
|--------|-----------|-------------|
| `begin` | `fn begin(agent_name: &str, model: Option<&str>) -> Self` | Create a new session |
| `agent_version` | `fn agent_version(&mut self, version: &str) -> &mut Self` | Set agent version |
| `parent` | `fn parent(&mut self, parent_id: EngramId) -> &mut Self` | Set parent engram for chaining |
| `set_summary` | `fn set_summary(&mut self, summary: &str) -> &mut Self` | Set session summary |
| `tag` | `fn tag(&mut self, tag: &str) -> &mut Self` | Add a tag |
| `log_message` | `fn log_message(&mut self, role: &str, content: &str) -> &mut Self` | Log a conversation message |
| `log_tool_call` | `fn log_tool_call(&mut self, tool_name: &str, input: &str, output_summary: Option<&str>) -> &mut Self` | Log a tool invocation |
| `log_file_change` | `fn log_file_change(&mut self, path: &str, change_type: &str) -> &mut Self` | Log a file change |
| `log_shell_command` | `fn log_shell_command(&mut self, command: &str, exit_code: Option<i32>, duration_ms: Option<u64>) -> &mut Self` | Log a shell command |
| `log_rejection` | `fn log_rejection(&mut self, approach: &str, reason: &str) -> &mut Self` | Log a dead end |
| `log_decision` | `fn log_decision(&mut self, description: &str, rationale: &str) -> &mut Self` | Log a decision |
| `add_tokens` | `fn add_tokens(&mut self, input: u64, output: u64, cost_usd: Option<f64>) -> &mut Self` | Add token usage (accumulates) |
| `build` | `fn build(self, git_sha: Option<&str>, summary: Option<&str>) -> EngramData` | Build without storing |
| `commit` | `fn commit(self, git_sha: Option<&str>, summary: Option<&str>) -> Result<EngramId>` | Store in Git (auto-discover repo) |
| `commit_to` | `fn commit_to(self, storage: &GitStorage, git_sha: Option<&str>, summary: Option<&str>) -> Result<EngramId>` | Store in specific repo |

### Change Type Values

The `change_type` parameter in `log_file_change` accepts:

| Value | Meaning |
|-------|---------|
| `"created"`, `"create"`, `"new"` | File was created |
| `"modified"` | File was modified |
| `"deleted"`, `"delete"`, `"removed"` | File was deleted |

### Message Roles

| Role | Description |
|------|-------------|
| `"user"` | Human message (first user message becomes the original request) |
| `"assistant"` | Agent response |
| `"system"` | System prompt |
| `"tool"` | Tool output |

### Re-exported Types

The SDK re-exports these types from `engram-core`:

- `EngramData`, `EngramId`, `Manifest`, `AgentInfo`, `TokenUsage`
- `FileChange`, `FileChangeType`, `CaptureMode`
- `GitStorage`

## See Also

- [SDK Overview](README.md) -- Cross-SDK comparison
- [Python SDK](python.md) -- Python version
- [TypeScript SDK](typescript.md) -- TypeScript version
