# Python SDK

## Installation

```bash
pip install engram
```

No compiled dependencies -- uses the git CLI via subprocess.

## Quick Start

```python
from engram import EngramSession

session = EngramSession.begin("my-agent", "claude-sonnet-4-5")
session.log_message("user", "Add OAuth2 authentication")
session.log_message("assistant", "Implementing OAuth2 with PKCE...")
session.log_tool_call("write_file", '{"path": "src/auth.rs"}', "Created auth module")
session.log_file_change("src/auth.rs", "created")
session.log_rejection("passport.js", "Middleware conflict with existing stack")
session.log_decision("Use JWT", "Stateless, works with load balancers")
session.add_tokens(1500, 800, 0.02)

engram_id = session.commit("abc123", "Implemented OAuth2 with PKCE")
print(f"Stored engram: {engram_id}")
```

## Context Manager

The Python SDK supports context managers for automatic commit on exit:

```python
from engram import EngramSession

with EngramSession("my-agent", "claude-sonnet-4-5") as session:
    session.log_message("user", "Add OAuth2 authentication")
    session.log_message("assistant", "Implementing...")
    session.log_file_change("src/auth.rs", "created")
    session.add_tokens(1500, 800, 0.02)
    # Automatically commits on successful exit
```

Async context manager is also supported:

```python
async with EngramSession("my-agent", "claude-sonnet-4-5") as session:
    session.log_message("user", "Fix the bug")
    # Auto-commits on exit
```

## Session Lifecycle

1. **Begin** -- `EngramSession.begin(agent, model)` or `EngramSession(agent, model)`
2. **Log** -- Chain `log_*` methods to record the session
3. **Commit** -- `session.commit()` stores the engram, or use a context manager

All methods return `self` for chaining:

```python
session.log_message("user", "Fix bug") \
       .log_message("assistant", "Found it") \
       .log_file_change("src/fix.py", "modified") \
       .add_tokens(500, 200, 0.005)
```

## API Reference

### EngramSession

| Method | Signature | Description |
|--------|-----------|-------------|
| `begin` | `begin(agent_name: str, model: str \| None = None) -> EngramSession` | Create a new session (classmethod) |
| `log_message` | `log_message(role: str, content: str) -> self` | Log a conversation message |
| `log_tool_call` | `log_tool_call(tool_name: str, input_data: str \| dict, output_summary: str \| None = None) -> self` | Log a tool invocation |
| `log_file_change` | `log_file_change(path: str, change_type: str) -> self` | Log a file change |
| `log_shell_command` | `log_shell_command(command: str, exit_code: int \| None = None, duration_ms: int \| None = None) -> self` | Log a shell command |
| `log_rejection` | `log_rejection(approach: str, reason: str) -> self` | Log a dead end |
| `log_decision` | `log_decision(description: str, rationale: str) -> self` | Log a decision |
| `add_tokens` | `add_tokens(input_tokens: int, output_tokens: int, cost_usd: float \| None = None) -> self` | Add token usage (accumulates) |
| `set_summary` | `set_summary(summary: str) -> self` | Set session summary |
| `tag` | `tag(tag: str) -> self` | Add a tag |
| `parent` | `parent(parent_id: str) -> self` | Set parent engram ID |
| `build` | `build(git_sha: str \| None = None, summary: str \| None = None) -> EngramData` | Build without storing |
| `commit` | `commit(git_sha: str \| None = None, summary: str \| None = None, storage: GitStorage \| None = None) -> str` | Store in Git, returns engram ID |

### GitStorage

| Method | Signature | Description |
|--------|-----------|-------------|
| `open` | `open(path: str \| Path) -> GitStorage` | Open a Git repo at path (classmethod) |
| `discover` | `discover(path: str \| Path = ".") -> GitStorage` | Discover repo from path (classmethod) |
| `create` | `create(data: EngramData) -> str` | Store engram, returns ID |
| `read` | `read(id_or_prefix: str) -> EngramData` | Read engram by ID or prefix |
| `read_manifest` | `read_manifest(id_or_prefix: str) -> Manifest` | Read only manifest (fast) |
| `list` | `list() -> list[Manifest]` | List all engrams, newest first |
| `delete` | `delete(id_or_prefix: str) -> None` | Delete engram by ID or prefix |

### Data Model

All types are Python dataclasses in `engram.model`:

| Class | Key Fields |
|-------|------------|
| `AgentInfo` | `name: str`, `model: str \| None`, `version: str \| None` |
| `TokenUsage` | `input_tokens: int`, `output_tokens: int`, `total_tokens: int`, `cost_usd: float \| None` |
| `DeadEnd` | `approach: str`, `reason: str` |
| `Decision` | `description: str`, `rationale: str` |
| `Intent` | `original_request: str`, `dead_ends: list[DeadEnd]`, `decisions: list[Decision]` |
| `TranscriptEntry` | `timestamp: datetime`, `role: str`, `content: dict` |
| `FileChange` | `path: str`, `change_type: str`, `lines_added: int \| None` |
| `ToolCall` | `tool_name: str`, `input: Any`, `output_summary: str \| None` |
| `Manifest` | `id: str`, `agent: AgentInfo`, `token_usage: TokenUsage`, `summary: str \| None` |
| `EngramData` | `manifest`, `intent`, `transcript`, `operations`, `lineage` |

### Enums

```python
class CaptureMode(str, Enum):
    WRAPPER = "wrapper"
    IMPORT = "import"
    SDK = "sdk"

class FileChangeType(str, Enum):
    CREATED = "created"
    MODIFIED = "modified"
    DELETED = "deleted"
```

## See Also

- [SDK Overview](README.md) -- Cross-SDK comparison
- [Rust SDK](rust.md)
- [TypeScript SDK](typescript.md)
