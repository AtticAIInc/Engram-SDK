# Engram

**Capture agent reasoning as first-class, versioned data in Git.**

Git commits capture *what* changed but discard *why*. When AI agents code, the reasoning trail -- 100K+ tokens of explored alternatives, architectural tradeoffs, rejected approaches -- IS the institutional knowledge. Engram makes reasoning first-class, versioned, and queryable, stored in Git itself.

Each **engram** is a discrete unit of reasoning memory: the full session transcript, human intent, agent decisions, tool calls, dead ends explored, and token economics -- linked to the Git commits it produced.

For full documentation and guides, visit: https://the-attic-ai.gitbook.io/untitled/

## Quick Start

```bash
# Install via npm (recommended)
npm install -g engram

# Or via curl
curl -fsSL https://raw.githubusercontent.com/AtticAIInc/Engram-SDK/main/install.sh | sh

# Or from source (requires Rust toolchain)
cargo install --git https://github.com/AtticAIInc/Engram-SDK.git engram-cli

# Initialize in your repo (all automation enabled by default)
engram init

# That's it. Now use Claude Code normally — sessions are auto-captured,
# commits are auto-annotated, and engram refs auto-push with your code.

# Explore your reasoning history
engram log --cost
engram show HEAD --intent
engram search "authentication"
engram why src/auth.rs
engram stats --trend
git loge                             # View reasoning inline on commits
```

## Why Engram?

**Without Engram**, your Git history looks like:
```
abc123 Add OAuth2 with PKCE flow
def456 Fix middleware ordering bug
789abc Refactor auth to use sessions
```

**With Engram**, every commit carries its reasoning (`git loge`):
```
abc123 Add OAuth2 with PKCE flow

    Engram-Id: a1b2c3d4...
    Engram-Agent: claude-code
    Engram-Model: claude-sonnet-4-5
    Engram-Tokens: 47832
    Engram-Cost: $0.23

Notes (engram):
    [claude-code/claude-sonnet-4-5] $0.23 47832tok
    Intent: "Add OAuth2 authentication with PKCE for our SPA"
    Summary: Implemented OAuth2 with PKCE using custom middleware
    Dead ends:
      - passport.js: Middleware conflict with existing stack
    Decisions:
      - Custom middleware over Auth0 SDK: Auth0 added 2MB to bundle
    Files: +auth.rs +oauth.rs ~api.rs
```

This is **institutional knowledge** that compounds. When the next agent (or human) touches auth, they see the full reasoning chain -- what was tried, what was rejected, and why.

## What `engram init` Enables

Running `engram init` sets up everything automatically:

| Feature | Default | Opt-out flag |
|---------|---------|--------------|
| **Auto-capture**: Claude Code sessions imported on commit | ON | `--no-auto-capture` |
| **Auto-push**: Engram refs sync when you `git push` | ON | `--no-auto-push` |
| **Claude Code hook**: Sessions auto-imported on exit | ON | `--no-claude-code` |
| **Git notes**: Reasoning attached to commits, viewable via `git loge` | ON | — |
| **Commit trailers**: `Engram-Id`, `Engram-Agent`, `Engram-Model`, `Engram-Tokens`, `Engram-Cost` | ON | — |

Existing git hooks are preserved -- engram chains after them via `.pre-engram` backups. All hooks fail silently to never break your workflow.

## Three Capture Modes

### Mode 1: Automatic (Claude Code)

Just run `engram init` and use Claude Code normally. Sessions are captured automatically via the `SessionEnd` hook when Claude Code exits, and via the `prepare-commit-msg` hook when you commit. No ongoing effort.

### Mode 2: Wrapper (any agent)
```bash
engram record -- claude "add auth"
engram record -- aider --model gpt-4o
engram record -- cursor-cli "fix the bug"
```
Spawns your agent in a PTY, captures output, detects file changes via SHA256 snapshots. Respects `.gitignore`, `.git/info/exclude`, and global gitignore rules.

### Mode 3: Session Import
```bash
engram import --auto-detect                              # Find and import from known agents
engram import ~/.claude/projects/.../session.jsonl --format claude-code
engram import .aider.chat.history.md --format aider
engram import --dry-run                                  # Preview what would be imported
```
Parses Claude Code JSONL sessions and Aider chat history markdown. Re-importing the same file is safe -- duplicate detection via content hashing prevents double imports.

## LLM-Powered Summarization

When an Anthropic API key is configured, engram sends a condensed transcript to Claude during import to generate high-quality summaries, interpreted goals, dead ends, and decisions. Without an API key, engram falls back to heuristic pattern extraction (still useful, but less nuanced).

```bash
# Set your API key (stored in ~/.config/engram/repos.toml, 0600 permissions)
engram config set anthropic_api_key sk-ant-api03-...

# Or use an environment variable (takes precedence)
export ANTHROPIC_API_KEY=sk-ant-api03-...

# Optionally override the model (default: claude-haiku-4-5-20251001)
engram config set summarize_model claude-sonnet-4-20250514
```

LLM insights are merged with heuristic-extracted ones, so no extracted insight is lost. The API key is masked in output (`sk-a...03-x`) and the config file is owner-readable only.

### Mode 4: SDK Integration

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

Engrams sync alongside code (automatically if auto-push is enabled, or manually):
```bash
engram push              # Push engram refs to remote
engram pull              # Fetch engram refs and reindex
engram fetch             # Fetch only (no reindex)
```

## Git Notes: Reasoning on Commits

Engram attaches rich reasoning metadata as [git notes](https://git-scm.com/docs/git-notes) to your commits. Notes are automatically attached when Claude Code's SessionEnd hook fires, and during `engram init` for any existing linked commits.

```bash
# Retroactively annotate commits (also runs automatically during init)
engram annotate                    # All commits linked to engrams
engram annotate main..HEAD         # Only commits in a range
engram annotate --dry-run          # Preview what would be annotated
engram annotate --force            # Overwrite existing notes

# View annotated commits
git loge                           # Alias installed by `engram init`
git log --notes=engram             # Standard git equivalent
```

Notes sync alongside engram refs during push/pull via `refs/notes/engram` refspecs.

## Search and Query

Full-text search powered by Tantivy, stored at `.git/engram-index/`:

```bash
engram search "authentication"
engram search "database migration" -n 20
engram trace src/auth.rs           # Reasoning history of a file
engram diff abc123 def456          # Compare two engrams
engram reindex                     # Rebuild search index
```

The search index is automatically updated when creating or importing engrams.

## Why Does This File Exist?

Go beyond `git blame` (which shows *who* changed a file) to understand *why* it exists:

```bash
engram why src/auth.rs
```

Produces a rich narrative tracing the file's full reasoning chain -- every session that touched it, what was requested, what was tried and rejected, and what decisions were made.

## Cost Analytics

Understand where your AI agent spend is going:

```bash
engram stats                    # Aggregate totals
engram stats --by-file --top 10 # Most expensive files
engram stats --by-branch        # Cost per feature branch
engram stats --trend            # Daily cost over last 30 days
```

Even when your agent doesn't report costs directly (e.g. Claude Code imports), engram estimates costs from the model name and token counts using built-in API pricing tables. Supports Claude (Opus, Sonnet, Haiku), GPT-4o/4-turbo/4/3.5, o1/o1-mini, o3/o3-mini, and o4-mini models with cache-aware pricing. Explicit cost data takes priority when available.

## Recurring Dead-End Detection

Find approaches that keep getting tried and rejected across sessions:

```bash
engram dead-ends                # List all dead ends
engram dead-ends --recurring    # Approaches rejected 2+ times
engram dead-ends --query "auth" # Filter by text
```

When `--recurring` finds that the same approach has been tried and rejected multiple times, it tells you: stop trying this, here's what worked instead.

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

## Context Graph

Engrams form a **context graph** -- a semantic reasoning layer over your codebase:

```bash
engram graph
engram graph file:src/auth.rs --depth 2
engram graph --dot | dot -Tsvg -o graph.svg
```

Nodes are engrams, files, agents, and commits. Edges are "modified by", "used agent", "follows from", "touched file", "produced by".

## GitHub Action

Post AI reasoning summaries on every pull request:

```yaml
name: Engram PR Summary
on:
  pull_request:
    types: [opened, synchronize]
permissions:
  pull-requests: write
  contents: read
jobs:
  engram:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: AtticAIInc/Engram-SDK@v1
        with:
          github-token: ${{ secrets.GITHUB_TOKEN }}
```

The action fetches engram refs, runs `engram pr-summary`, and posts a sticky comment with the reasoning chain, files changed, dead ends explored, and token economics.

## MCP Server

Expose engram data to AI agents via the [Model Context Protocol](https://modelcontextprotocol.io):

```bash
engram mcp
```

Starts an MCP server on stdio with 8 tools:

| Tool | Description |
|------|-------------|
| `engram_search` | Full-text search across intent, transcript, file paths, and dead ends |
| `engram_show` | Show full engram details (supports `"HEAD"` for most recent) |
| `engram_log` | List recent engrams with token usage and cost |
| `engram_trace` | Chronological reasoning history for a specific file |
| `engram_diff` | Compare two engrams: common/unique files, token and cost deltas |
| `engram_dead_ends` | Surface rejected approaches; find recurring dead ends |
| `engram_why` | Explain why a file exists through its full reasoning chain |
| `engram_stats` | Aggregate statistics by file, branch, or daily trend |

### Claude Code

Repos with a `.mcp.json` file are auto-configured -- Claude Code discovers and starts the engram MCP server automatically. To add to any repo, create `.mcp.json` at the root:

```json
{
  "mcpServers": {
    "engram": {
      "command": "engram",
      "args": ["mcp"],
      "env": {
        "PATH": "${HOME}/.cargo/bin:${PATH}"
      }
    }
  }
}
```

For global access across all projects, register at user scope: `claude mcp add --transport stdio --scope user engram -- engram mcp`

### Claude Desktop

Add to `~/.config/Claude/claude_desktop_config.json` (macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`):

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
| `init`        | Initialize engram with smart defaults (`--no-auto-capture`, `--no-auto-push`, `--no-claude-code` to opt out) |
| `config`      | Manage global configuration (`set`, `get`, `list`, `path`) |
| `record`      | Record an agent session via PTY wrapper (`--agent`, `--model`) |
| `import`      | Import sessions from Claude Code or Aider (`--auto-detect`, `--dry-run`) |
| `log`         | List engrams (`--cost`, `--by-agent`, `--limit N`) |
| `show`        | Show engram details (`HEAD`, `--intent`, `--transcript`, `--operations`) |
| `search`      | Full-text search across engrams (`-n` limit) |
| `trace`       | Show reasoning history for a file |
| `why`         | Explain why a file exists through its reasoning chain |
| `diff`        | Compare two engrams (files, tokens, cost) |
| `graph`       | Show the context graph (`--dot` for Graphviz) |
| `review`      | Review intent chain for a branch range |
| `pr-summary`  | Generate a PR description from the engram chain |
| `mcp`         | Start MCP server (stdio) for AI agent integration |
| `stats`       | Aggregate statistics (`--by-file`, `--by-branch`, `--trend`, `--top N`) |
| `dead-ends`   | Surface dead ends (`--recurring`, `--query`, `--id`) |
| `annotate`    | Attach engram reasoning as git notes (`--dry-run`, `--force`, range) |
| `blame`       | Show reasoning blame for a file |
| `audit`       | Generate audit trail report for compliance (`--format csv`) |
| `gc`          | Garbage collect old engrams (`--older-than`, `--dry-run`) |
| `push`        | Push engram refs to a remote |
| `pull`        | Pull engram refs and reindex |
| `fetch`       | Fetch engram refs from a remote |
| `reindex`     | Rebuild the search index |
| `browse`      | Interactive terminal UI for browsing engrams |
| `dashboard`   | Start the web dashboard for visualizing engram data (`--port`) |
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
  engram-dashboard/  Web dashboard (axum, embedded SPA)
  engram-tui/        Interactive terminal UI (ratatui)
  engram-cli/        CLI binary (installed as `engram`)
sdks/
  python/            Python SDK (git CLI)
  typescript/        TypeScript SDK (git CLI)
```

### Design Principles

- **Git-native**: Engrams are Git objects (blobs, trees, commits, refs). No external database.
- **Smart defaults**: `engram init` enables all automation. Opt out, not in.
- **Zero config remotes**: Engram refs sync with standard `git push`/`fetch` via refspecs.
- **Vendored dependencies**: git2 with vendored libgit2 + OpenSSL. No system deps beyond a C compiler.
- **No unsafe code**: `unsafe_code = "forbid"` workspace-wide.
- **Library-first**: All functionality lives in library crates; the CLI is a thin wrapper.
- **Cross-platform**: File locking via `fs2`, Unix-specific code guarded by `#[cfg(unix)]`.
- **Safe imports**: Duplicate detection via SHA-256 content hashing prevents re-importing the same session.

## Building from Source

```bash
git clone https://github.com/AtticAIInc/Engram-SDK.git
cd Engram-SDK
cargo build --workspace

# Run tests (179 Rust + 10 Python + 7 TypeScript = 196 total)
cargo test --workspace
cd sdks/python && python3 -m pytest tests/
cd sdks/typescript && npx vitest run

# Lint
cargo clippy --workspace -- -D warnings

# Install
cargo install --path crates/engram-cli
```

Requires Rust 1.80+ and a C compiler (for vendored libgit2/OpenSSL).

## License

Apache-2.0 OR MIT
