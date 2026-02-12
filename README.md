# Engram

**Capture agent reasoning as first-class, versioned data in Git.**

Git commits capture *what* changed but discard *why*. When AI agents code, the reasoning trail -- 100K+ tokens of explored alternatives, architectural tradeoffs, rejected approaches -- IS the institutional knowledge. Engram makes reasoning first-class, versioned, and queryable, stored in Git itself.

Each **engram** is a discrete unit of reasoning memory: the full session transcript, human intent, agent decisions, tool calls, dead ends explored, and token economics -- linked to the Git commits it produced.

## Quick Start

```bash
# Install from source
git clone https://github.com/AtticAIInc/Engram-SDK.git
cd Engram-SDK
cargo install --path crates/engram-cli

# Initialize in your repo
engram init

# Record an agent session (wraps any agent command in a PTY)
engram record -- claude "add OAuth2 authentication"

# Import existing sessions from Claude Code or Aider
engram import --auto-detect

# Explore reasoning history
engram log --cost
engram show HEAD --intent
engram search "authentication"
engram trace src/auth.rs

# Review by intent, not by diff
engram review main..feature-branch

# Push reasoning alongside code
engram push
```

## Why Engram?

**Without Engram**, your Git history looks like:
```
abc123 Add OAuth2 with PKCE flow
def456 Fix middleware ordering bug
789abc Refactor auth to use sessions
```

**With Engram**, every commit carries its reasoning:
```
abc123 Add OAuth2 with PKCE flow [claude-code/claude-sonnet-4-5] $0.23 47832tok
  Intent: "Add OAuth2 authentication with PKCE for our SPA"
  Summary: Implemented OAuth2 with PKCE using custom middleware. Added
           token refresh, CSRF protection, and session management.
  Dead ends: Tried passport.js (middleware conflict), considered Auth0
             SDK (added 2MB to bundle, decided against).
  Files: +src/auth.rs +src/middleware/oauth.rs ~src/routes/api.rs

def456 Fix middleware ordering bug
  Intent: "The auth middleware runs after the rate limiter, causing 401s"
  Summary: Reordered middleware stack. Auth must run before rate limiting.
  Related: Follows from abc123 (the original auth implementation)
```

This is **institutional knowledge** that compounds. When the next agent (or human) touches auth, they see the full reasoning chain -- what was tried, what was rejected, and why.

## Three Capture Modes

### Mode 1: Wrapper (any agent)
```bash
engram record -- claude "add auth"
engram record -- aider --model gpt-4o
engram record -- cursor-cli "fix the bug"
```
Spawns your agent in a PTY, captures output, detects file changes via SHA256 snapshots. File change detection respects `.gitignore`, `.git/info/exclude`, and global gitignore rules.

### Mode 2: Session Import
```bash
engram import --auto-detect                              # Find and import from known agents
engram import ~/.claude/projects/.../session.jsonl --format claude-code
engram import .aider.chat.history.md --format aider
engram import --dry-run                                  # Preview what would be imported
```
Parses Claude Code JSONL sessions and Aider chat history markdown. Extracts transcripts, tool calls, token usage, and file changes. Re-importing the same file is safe -- duplicate detection via content hashing prevents double imports.

### Mode 3: SDK Integration

**Rust:**
```rust
use engram_sdk::EngramSession;

let mut session = EngramSession::begin("my-agent", Some("claude-sonnet-4-5"));
session.log_message("user", "Add OAuth2 authentication");
session.log_message("assistant", "Implementing OAuth2 with PKCE...");
session.log_tool_call("write_file", r#"{"path":"src/auth.rs"}"#, Some("Created auth module"));
session.log_file_change("src/auth.rs", "created");
session.log_rejection("passport.js", "Middleware conflict with existing stack");
session.add_tokens(1500, 800, Some(0.02));
let id = session.commit(Some("abc123"), Some("Implemented OAuth2 with PKCE")).unwrap();
```

**Python:**
```python
from engram import EngramSession

with EngramSession("my-agent", "claude-sonnet-4-5") as session:
    session.log_message("user", "Add OAuth2 authentication")
    session.log_message("assistant", "Implementing OAuth2 with PKCE...")
    session.log_tool_call("write_file", {"path": "src/auth.rs"})
    session.log_file_change("src/auth.rs", "created")
    session.log_rejection("passport.js", "Middleware conflict")
    session.add_tokens(1500, 800, 0.02)
```

**TypeScript:**
```typescript
import { EngramSession } from '@engram/sdk';

const session = EngramSession.begin('my-agent', 'claude-sonnet-4-5');
session.logMessage('user', 'Add OAuth2 authentication');
session.logToolCall('write_file', { path: 'src/auth.rs' });
session.logFileChange('src/auth.rs', 'created');
session.logRejection('passport.js', 'Middleware conflict');
session.addTokens(1500, 800, 0.02);
const id = session.commit('abc123', 'Implemented OAuth2 with PKCE');
```

## Git-Native Storage

Engrams are stored as native Git objects -- they travel with `clone`, `push`, `pull`. No sidecar database, no separate sync, no vendor lock-in.

```
.git/refs/engrams/
  ab/
    abc123...  -> commit -> tree containing:
      manifest.json        # Compact metadata for fast listing
      intent.md            # Human-readable reasoning summary
      transcript.jsonl     # Full session, one message per line
      operations.json      # Tool calls, file ops, shell commands
      lineage.json         # Relationships to other engrams
```

Engrams sync alongside code:
```bash
engram push              # Push engram refs to remote
engram pull              # Fetch engram refs and reindex
engram fetch             # Fetch only (no reindex)
```

## Git Hooks Integration

When you run `engram init`, git hooks are automatically installed:

- **prepare-commit-msg**: Injects `Engram-Id:` and `Engram-Agent:` trailers into commit messages during active recording sessions
- **post-commit**: Links new commit SHAs to the active session for automatic commit tracking

Existing hooks are preserved -- engram chains after them via `.pre-engram` backups. Hooks fail silently to never break your git workflow.

## Search and Query

Full-text search powered by Tantivy, stored at `.git/engram-index/`:

```bash
# Search across intent, transcript, file paths, dead ends
engram search "authentication"
engram search "database migration" -n 20

# Trace the full reasoning history of a file
engram trace src/auth.rs

# Compare two engrams (files, tokens, cost)
engram diff abc123 def456

# Rebuild search index from scratch
engram reindex
```

The search index is automatically updated when creating or importing engrams.

## Context Graph

Engrams form a **context graph** -- a semantic reasoning layer over your codebase:

```bash
# Explore connections between engrams, files, and agents
engram graph
engram graph file:src/auth.rs --depth 2

# Export as Graphviz DOT format
engram graph --dot | dot -Tsvg -o graph.svg
```

Nodes are engrams, files, agents, and commits. Edges are "modified by", "used agent", "follows from", "touched file", "produced by".

## Intent-Based Review

```bash
engram review main..feature-branch
```

Instead of line-by-line code review, read the chain of intents and summaries. See what was asked, what was done, what dead ends were explored, and what architectural decisions were made. Includes aggregate token usage and cost.

## PR Summary

Auto-generate structured PR descriptions from the engram chain:

```bash
engram pr-summary main..feature-branch
engram pr-summary main..feature-branch --format json
```

Outputs a markdown PR description with summary, file changes, reasoning chain, dead ends, and token economics.

## MCP Server

Expose engram data to AI agents via the [Model Context Protocol](https://modelcontextprotocol.io):

```bash
engram mcp
```

Starts an MCP server on stdio with 6 tools:

| Tool | Description |
|------|-------------|
| `engram_search` | Full-text search across engrams |
| `engram_show` | Show full details of an engram |
| `engram_log` | List recent engrams |
| `engram_trace` | Reasoning history for a file |
| `engram_diff` | Compare two engrams |
| `engram_dead_ends` | Surface rejected approaches |

Configure in Claude Desktop (`claude_desktop_config.json`):
```json
{
  "mcpServers": {
    "engram": {
      "command": "engram",
      "args": ["mcp"]
    }
  }
}
```

## CLI Reference

| Command       | Description |
|---------------|-------------|
| `init`        | Initialize engram in a Git repository (`--remote`, `--force`) |
| `record`      | Record an agent session via PTY wrapper (`--agent`, `--model`) |
| `import`      | Import sessions from Claude Code or Aider (with dedup) |
| `log`         | List engrams (most recent first) (`--cost`, `--by-agent`) |
| `show`        | Show details of a specific engram (supports `HEAD`) |
| `search`      | Full-text search across engrams |
| `trace`       | Show reasoning history for a file |
| `diff`        | Compare two engrams |
| `graph`       | Show the context graph (text or DOT) |
| `review`      | Review intent chain for a branch range |
| `pr-summary`  | Generate a PR description from the engram chain |
| `mcp`         | Start MCP server (stdio) for AI agent integration |
| `stats`       | Show aggregate statistics across all engrams |
| `blame`       | Show reasoning blame for a file |
| `gc`          | Garbage collect old engrams (`--older-than`, `--dry-run`) |
| `push`        | Push engram refs to a remote |
| `pull`        | Pull engram refs and reindex |
| `fetch`       | Fetch engram refs from a remote |
| `reindex`     | Rebuild the search index |
| `version`     | Print version information |

All commands support `--format json` for machine-readable output and `-v`/`-vv`/`-vvv` for verbosity.

## Architecture

```
crates/
  engram-core/       Data model, Git storage engine, hooks, config
  engram-capture/    PTY wrapper, file change detection, session importers
  engram-query/      Tantivy search index, context graph, branch review
  engram-protocol/   Push/pull/fetch via Git refspecs
  engram-sdk/        Fluent Rust SDK for direct agent integration
  engram-mcp/        MCP server for AI agent integration (rmcp)
  engram-cli/        CLI binary (installed as `engram`)
sdks/
  python/            Python SDK (pygit2)
  typescript/        TypeScript SDK (git CLI)
```

### Design Principles

- **Git-native**: Engrams are Git objects (blobs, trees, commits, refs). No external database.
- **Zero config remotes**: Engram refs sync with standard `git push`/`fetch` via refspecs.
- **Vendored dependencies**: git2 with vendored libgit2 + OpenSSL. No system deps beyond a C compiler.
- **No unsafe code**: `unsafe_code = "forbid"` workspace-wide.
- **Library-first**: All functionality lives in library crates; the CLI is a thin wrapper.
- **Cross-platform**: File locking via `fs2`, Unix-specific code guarded by `#[cfg(unix)]`.
- **Safe imports**: Duplicate detection via SHA-256 content hashing prevents re-importing the same session.

## Building from Source

```bash
# Clone and build
git clone https://github.com/AtticAIInc/Engram-SDK.git
cd Engram-SDK
cargo build --workspace

# Run tests (54 Rust + 10 Python + 7 TypeScript = 71 total)
cargo test --workspace
cd sdks/python && python3 -m pytest tests/
cd sdks/typescript && npx vitest run

# Lint
cargo clippy --workspace -- -D warnings

# Install the CLI
cargo install --path crates/engram-cli
```

Requires Rust 1.80+ and a C compiler (for vendored libgit2/OpenSSL).

## License

Apache-2.0 OR MIT
