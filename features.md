# Engram Features

Engram captures AI agent reasoning as first-class, versioned data in Git. This document catalogs every feature across the CLI, Rust library crates, MCP server, and language SDKs.

---

## Table of Contents

- [Git-Native Storage Engine](#git-native-storage-engine)
- [Reasoning Data Model](#reasoning-data-model)
- [Session Capture](#session-capture)
- [Session Import](#session-import)
- [Search and Indexing](#search-and-indexing)
- [File Tracing](#file-tracing)
- [Engram Diff](#engram-diff)
- [Context Graph](#context-graph)
- [Branch Review](#branch-review)
- [PR Summary Generation](#pr-summary-generation)
- [Reasoning Blame](#reasoning-blame)
- [Statistics](#statistics)
- [Garbage Collection](#garbage-collection)
- [Remote Sync (Push / Pull / Fetch)](#remote-sync-push--pull--fetch)
- [Git Hooks Integration](#git-hooks-integration)
- [MCP Server](#mcp-server)
- [SDK Integration](#sdk-integration)
  - [Rust SDK](#rust-sdk)
  - [Python SDK](#python-sdk)
  - [TypeScript SDK](#typescript-sdk)
- [CLI Reference](#cli-reference)
- [Architecture and Design](#architecture-and-design)

---

## Git-Native Storage Engine

Engrams are stored as native Git objects -- no sidecar database, no external dependencies, no vendor lock-in. They travel with `clone`, `push`, and `pull`.

**Object model:** Each engram is a standalone Git commit pointing to a tree of five blobs:

```
refs/engrams/<ab>/<full-id>  ->  commit ("engram: <id>")
                                    └── tree
                                        ├── manifest.json      Compact metadata
                                        ├── intent.md          Human-readable reasoning
                                        ├── transcript.jsonl   Full session (one message per line)
                                        ├── operations.json    Tool calls, file ops, shell commands
                                        └── lineage.json       Relationships to other engrams
```

**Key properties:**

| Property | Detail |
|----------|--------|
| Ref layout | `refs/engrams/<fanout>/<full-id>` with 2-char fanout prefix |
| ID format | UUID v4 hex, 32 characters (no dashes) |
| ID resolution | Exact match, prefix match, or `HEAD` alias (via `.git/engram-head` pointer) |
| HEAD pointer | `.git/engram-head` file updated on every create for O(1) resolution |
| Config | `[engram]` section in `.git/config` (`enabled`, `auto_capture`, `default_agent`, `push_on_push`) |
| Import dedup | SHA-256 `source_hash` on Manifest prevents re-importing the same session file |
| Commits | Orphan commits (no parents) -- engrams form their own DAG via lineage, separate from code history |

**CRUD operations** (via `GitStorage`):

- `create(data)` -- serialize and store as Git objects, create ref, update head pointer
- `read(id_or_prefix)` -- resolve ID, load full `EngramData` from Git objects
- `read_manifest(id_or_prefix)` -- fast path, loads only the manifest blob
- `list(opts)` -- enumerate all engram refs, sorted by `created_at` descending, with optional agent filter and limit
- `delete(id_or_prefix)` -- remove the engram ref
- `find_by_source_hash(hash)` -- lookup for import deduplication

---

## Reasoning Data Model

Every engram captures a complete reasoning unit with five components:

### Manifest (`manifest.json`)

Core metadata for fast listing without reading the full engram.

| Field | Type | Description |
|-------|------|-------------|
| `id` | EngramId | 32-char UUID v4 hex |
| `version` | u32 | Schema version |
| `created_at` | DateTime | Session start timestamp |
| `finished_at` | DateTime? | Session end timestamp |
| `agent` | AgentInfo | Agent name, model, and version |
| `git_commits` | Vec\<String\> | Associated code commit SHAs |
| `token_usage` | TokenUsage | Input/output/cache tokens and cost |
| `summary` | String? | Brief summary of the session |
| `tags` | Vec\<String\> | User-defined tags |
| `capture_mode` | CaptureMode | `wrapper`, `import`, or `sdk` |
| `source_hash` | String? | SHA-256 for import deduplication |

### Intent (`intent.md`)

Human-readable reasoning summary, stored as Markdown with structured sections.

| Field | Description |
|-------|-------------|
| `original_request` | The exact human instruction that started the session |
| `interpreted_goal` | Agent's interpretation of what was being asked |
| `summary` | Brief description of what was accomplished |
| `dead_ends` | Approaches tried and rejected (approach + reason) |
| `decisions` | Architectural decisions made (description + rationale) |

### Transcript (`transcript.jsonl`)

Full conversation log, one JSON entry per line.

| Entry type | Content |
|------------|---------|
| Text | Role (user/assistant/system/tool) + text content |
| ToolUse | Tool name, tool ID, JSON input (Claude format) |
| ToolResult | Tool ID, output text, error flag |
| Thinking | Extended thinking / chain-of-thought text |

### Operations (`operations.json`)

Concrete actions taken during the session.

| Section | Fields |
|---------|--------|
| Tool calls | timestamp, tool_name, input (JSON), output_summary, duration_ms, is_error |
| File changes | path, change_type (created/modified/deleted/renamed), lines added/removed |
| Shell commands | timestamp, command, exit_code, duration_ms |

### Lineage (`lineage.json`)

Relationships between engrams and code.

| Field | Description |
|-------|-------------|
| `parent_engram` | ID of preceding engram in a reasoning chain |
| `child_engrams` | IDs of engrams that follow from this one |
| `related_engrams` | Typed relationships (follows_from, motivates, depends_on, supersedes, conflicts_with) |
| `git_commits` | Code commits produced during this session |
| `branch` | Git branch name |

### Token Economics

| Field | Description |
|-------|-------------|
| `input_tokens` | Tokens sent to the model |
| `output_tokens` | Tokens received from the model |
| `cache_read_tokens` | Tokens served from prompt cache |
| `cache_write_tokens` | Tokens written to prompt cache |
| `total_tokens` | Sum of all token fields |
| `cost_usd` | Estimated cost in USD |

---

## Session Capture

**Command:** `engram record -- <command> [args...]`

Wraps any AI agent command in a PTY (pseudo-terminal) to capture its full session.

**How it works:**

1. Snapshots the working tree (SHA-256 hash of every tracked file, respecting `.gitignore`, `.git/info/exclude`, and global gitignore via the `ignore` crate)
2. Creates an `ActiveSession` (`.git/engram-session` with `fs2` advisory file locking)
3. Spawns the command in a PTY with separate reader/writer threads
4. Captures all stdout/stderr output
5. After exit, re-snapshots the working tree and diffs to detect file changes
6. Runs heuristic dead-end/decision extraction on the raw output
7. Builds and stores the engram, incrementally updates the search index

**Dead-end / decision extraction heuristics** (case-insensitive patterns on raw output):

| Pattern | Extracted as |
|---------|-------------|
| "tried X but Y" | Dead end |
| "rejected X because Y" | Dead end |
| "X didn't work because Y" | Dead end |
| "instead of X" | Dead end |
| "decided to X because Y" | Decision |
| "chose X over Y" | Decision |

**Options:**

| Flag | Description |
|------|-------------|
| `--agent NAME` | Agent name (auto-detected from command if omitted) |
| `--model MODEL` | Model name (e.g., `claude-sonnet-4-5`, `gpt-4o`) |

---

## Session Import

**Command:** `engram import [path] [--format FORMAT] [--auto-detect] [--dry-run]`

Import reasoning sessions from existing AI coding tools.

### Claude Code Import

Parses Claude Code JSONL session files from `~/.claude/projects/`.

**Capabilities:**
- Auto-discovers session files by converting the repo path to Claude's project key format (e.g., `/Users/name/project` -> `-Users-name-project`)
- Parses JSONL entries with flexible content blocks: text, tool_use, tool_result, thinking
- Extracts model name from the first assistant message
- Accumulates token usage including cache tokens (cache_creation_input_tokens, cache_read_input_tokens)
- Tracks file operations from Write/Edit/NotebookEdit tool calls
- Captures agent version from session metadata
- Skips sidechain messages

### Aider Import

Parses Aider's `.aider.chat.history.md` markdown format.

**Capabilities:**
- Discovers `.aider.chat.history.md` in the repository root
- Splits multi-session files by `# aider chat started at` headers
- Parses user messages (`#### ` prefix), assistant responses (unprefixed), and system output (`> ` prefix)
- Extracts token counts from `> Tokens: 3.2k sent, 245 received. Cost: $0.01` lines
- Supports k/K/m/M suffixes for token numbers (e.g., `3.2k` = 3200)
- Each session in a file gets a unique source hash: `SHA256(file_hash:index)`

### Deduplication

Both importers compute a SHA-256 `source_hash` from the source file content. Before storing, `find_by_source_hash()` checks if the hash already exists. Re-importing the same file is always safe.

---

## Search and Indexing

**Command:** `engram search <query> [-n LIMIT]`

Full-text search powered by Tantivy, stored at `.git/engram-index/`.

**Indexed fields:**

| Field | Searchable | Stored | Type |
|-------|-----------|--------|------|
| `id` | exact | yes | STRING |
| `intent_request` | full-text | yes | TEXT |
| `intent_summary` | full-text | yes | TEXT |
| `transcript_text` | full-text | no | TEXT |
| `agent_name` | exact | yes | STRING |
| `agent_model` | exact | yes | STRING |
| `created_at` | range | yes | DATE |
| `file_paths` | full-text | yes | TEXT |
| `dead_ends` | full-text | yes | TEXT |
| `cost_usd` | range | yes | F64 |
| `total_tokens` | range | yes | U64 |
| `manifest_json` | no | yes | stored-only |

**Search behavior:**
- Queries are parsed by Tantivy's QueryParser across 5 fields: intent_request, intent_summary, transcript_text, dead_ends, file_paths
- Results ranked by relevance score
- Specialized `search_by_file(path, limit)` for file-specific lookups

**Index lifecycle:**
- Lazy creation on first search (`ensure_index`)
- Incremental updates when creating or importing engrams (`index_engram`)
- Full rebuild on demand (`engram reindex`)
- Automatic rebuild after `engram pull` fetches new refs
- 50MB writer heap allocation

---

## File Tracing

**Command:** `engram trace <file>`

Shows the complete reasoning history for a specific file across all engrams.

- Lists every engram that created, modified, or deleted the file
- Results sorted chronologically (oldest first)
- Displays: short ID, date, agent name, summary, and change type
- Powered by `search_by_file()` under the hood (limit: 100 results)

---

## Engram Diff

**Command:** `engram diff <id_a> <id_b>`

Compares two engrams side-by-side.

**Comparison output:**

| Section | Description |
|---------|-------------|
| Common files | Files modified by both engrams |
| Only in A | Files unique to the first engram |
| Only in B | Files unique to the second engram |
| Token delta | Difference in total tokens (B - A) |
| Cost delta | Difference in USD cost (B - A) |

---

## Context Graph

**Command:** `engram graph [node] [--depth N] [--dot]`

Builds a semantic reasoning graph over the codebase.

**Node types:**
- Engram (box shape in DOT) -- labeled with summary or short ID
- File (note shape) -- source files touched by engrams
- Agent (diamond shape) -- AI agents that created engrams
- Commit (ellipse shape) -- Git commits linked to engrams

**Edge types:**
- Engram --UsedAgent--> Agent
- Engram --TouchedFile--> File
- File --ModifiedBy--> Engram
- Engram --ProducedBy--> Commit
- Engram --FollowsFrom--> Parent Engram (lineage)

**Features:**
- Full graph construction from all engrams
- Subgraph extraction via BFS around a center node (e.g., `file:src/auth.rs`)
- Configurable traversal depth (default: 2)
- Node deduplication via HashSet

**Output formats:**
- Text -- node and edge listing with counts
- JSON -- full node/edge data
- Graphviz DOT -- with shapes, edge labels, left-to-right layout (`rankdir=LR`)

---

## Branch Review

**Command:** `engram review <base>..<head>`

Intent-based code review: read the chain of reasoning instead of line-by-line diffs.

**How it works:**
1. Resolves base and head refs
2. Walks the git log (revwalk from head, hiding base)
3. For each commit, parses the message for `Engram-Id:` trailers
4. Deduplicates engram IDs
5. Reads full engram data and aggregates

**Output:**
- Total commits in range
- Number of engrams found
- Aggregate token usage and cost
- All files changed across engrams
- Per-engram: short ID, commit SHA, summary

---

## PR Summary Generation

**Command:** `engram pr-summary <base>..<head> [--format json|markdown]`

Auto-generates structured PR descriptions from the engram chain.

**Markdown output includes:**
- Summary section with per-engram summaries
- Files changed across all engrams
- Dead ends explored (rejected approaches and reasons)
- Token economics (total tokens, total cost)
- Commit list
- Bot attribution footer

---

## Reasoning Blame

**Command:** `engram blame <file> [-n LIMIT]`

Shows reasoning blame for a file -- who changed it, when, and why.

**Output per engram:**
- Short ID and date
- Change type (created / modified / deleted / touched)
- Agent name
- Summary
- Intent (if different from summary)
- Dead ends encountered

---

## Statistics

**Command:** `engram stats`

Aggregate statistics across all engrams.

**Output:**
- Total engram count
- Total tokens consumed
- Total cost (USD)
- Date range (earliest to latest)
- Breakdown by agent name
- Breakdown by capture mode (wrapper / import / sdk)

---

## Garbage Collection

**Command:** `engram gc --older-than <duration> [--dry-run] [-y]`

Removes engrams older than a specified duration.

**Duration units:**
- `d` -- days (e.g., `30d`)
- `w` -- weeks (e.g., `4w`)
- `m` -- months (30 days, e.g., `6m`)
- `y` -- years (365 days, e.g., `1y`)

**Safety:**
- `--dry-run` previews what would be deleted
- Interactive confirmation prompt (skip with `-y`)
- Lists each engram to be deleted before proceeding

---

## Remote Sync (Push / Pull / Fetch)

Engram refs sync alongside code via standard Git refspecs.

### Push

**Command:** `engram push [remote] [--dry-run]`

- Pushes `refs/engrams/*` to the remote
- Auto-configures refspecs on the remote if missing
- Push refspec: `refs/engrams/*:refs/engrams/*`

### Pull

**Command:** `engram pull [remote]`

- Fetches engram refs from the remote
- Automatically rebuilds the search index if new refs were fetched

### Fetch

**Command:** `engram fetch [remote] [--dry-run]`

- Fetches engram refs without rebuilding the index
- Fetch refspec: `+refs/engrams/*:refs/engrams/*` (force flag)

### Refspec Management

- `ensure_refspecs(repo, remote)` -- idempotent, adds engram refspecs to a specific remote
- `ensure_all_refspecs(repo)` -- configures all remotes in the repository
- Additive only (never removes existing refspecs)
- Configured automatically during `engram init`

---

## Git Hooks Integration

Installed automatically by `engram init`. Existing hooks are preserved via `.pre-engram` backups and chained.

### prepare-commit-msg

During an active recording session, injects trailers into commit messages:

```
Engram-Id: <32-char-id>
Engram-Agent: <agent-name>/<model>
Engram-Tokens: <total-tokens>
Engram-Cost: $<cost>
```

Idempotent: skips injection if `Engram-Id:` is already present.

### post-commit

Records the new commit SHA to the active session file (`.git/engram-session`).

### Session Locking

- Active sessions stored as JSON in `.git/engram-session`
- `fs2` advisory file locking for concurrent commit safety
- Exclusive locks for writes, shared locks for reads

### Hook Safety

- Hooks fail silently to never break the git workflow
- Unknown hook names are ignored with a debug log
- Installation backs up existing hooks; uninstallation restores originals

---

## MCP Server

**Command:** `engram mcp`

Starts a Model Context Protocol server on stdio transport, exposing engram data to AI agents (e.g., Claude Desktop, Claude Code).

**Implementation:** `engram-mcp` crate using `rmcp` v0.15 with `schemars` v1 for JSON schema generation. Stores `PathBuf` instead of `git2::Repository` because `Repository` is `!Send` and rmcp requires `Send + Sync + 'static`. Each tool opens the repo fresh per request.

### Tools

| Tool | Parameters | Description |
|------|-----------|-------------|
| `engram_search` | `query`, `limit?` (default: 10) | Full-text search across intent, transcript, files, dead ends |
| `engram_show` | `id` (or "HEAD") | Full engram details: manifest, intent, file changes, dead ends, decisions |
| `engram_log` | `limit?` (default: 10), `by_agent?` | List recent engrams with tokens and cost |
| `engram_trace` | `file_path` | Chronological reasoning history for a file |
| `engram_diff` | `id_a`, `id_b` | Compare two engrams (files, tokens, cost) |
| `engram_dead_ends` | `id?`, `query?` | Surface rejected approaches from one or all engrams |

### Configuration (Claude Desktop)

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

---

## SDK Integration

Three SDKs with identical fluent APIs for programmatic engram creation.

### Rust SDK

**Crate:** `engram-sdk`

```rust
use engram_sdk::EngramSession;

let mut session = EngramSession::begin("my-agent", Some("claude-sonnet-4-5"));
```

**Session lifecycle methods** (all return `&mut Self` for chaining):

| Method | Description |
|--------|-------------|
| `log_message(role, content)` | Log conversation message (first user message auto-captured as original_request) |
| `log_tool_call(name, input, output?)` | Log tool invocation (parses JSON input) |
| `log_file_change(path, change_type)` | Track file modification (created/modified/deleted) |
| `log_shell_command(cmd, exit_code?, duration?)` | Log shell command execution |
| `log_rejection(approach, reason)` | Record rejected approach (dead end) |
| `log_decision(description, rationale)` | Record architectural decision |
| `add_tokens(input, output, cost?)` | Accumulate token usage (additive across calls) |
| `set_summary(summary)` | Set session summary |
| `tag(tag)` | Add a tag |
| `parent(parent_id)` | Link to parent engram |
| `agent_version(version)` | Set agent version |

**Finalization:**

| Method | Description |
|--------|-------------|
| `commit(git_sha?, summary?)` | Auto-discovers repo, stores engram, returns `EngramId` |
| `commit_to(storage, git_sha?, summary?)` | Stores to a specific `GitStorage` instance |
| `build(git_sha?, summary?)` | Builds `EngramData` without storing (for testing) |

**Re-exports:** `AgentInfo`, `CaptureMode`, `EngramData`, `EngramId`, `FileChange`, `FileChangeType`, `GitStorage`, `Manifest`, `TokenUsage`

### Python SDK

**Package:** `engram` (requires pygit2 >= 1.14, Python >= 3.9)

```python
from engram import EngramSession

with EngramSession("my-agent", "claude-sonnet-4-5") as session:
    session.log_message("user", "Add OAuth2 authentication")
    # Auto-commits on context manager exit
```

**API parity with Rust SDK**, plus:
- Context manager support (sync and async via `__aenter__` / `__aexit__`)
- Auto-commit on successful context manager exit
- `build()` and `commit()` methods
- `GitStorage` with `open()`, `discover()`, `create()`, `read()`, `read_manifest()`, `list()`, `delete()`
- Full data model classes matching Rust types (`Manifest`, `Intent`, `Transcript`, `Operations`, `Lineage`, `EngramData`)

**Tests:** 10 tests covering model serialization, session lifecycle, token accumulation, and Git round-trips.

### TypeScript SDK

**Package:** `@engram/sdk` (requires Node.js >= 18, uses `git` CLI via `execFileSync`)

```typescript
import { EngramSession } from '@engram/sdk';

const session = EngramSession.begin('my-agent', 'claude-sonnet-4-5');
session.logMessage('user', 'Add OAuth2 authentication');
const id = session.commit('abc123', 'Implemented OAuth2');
```

**API parity with Rust SDK**, with camelCase naming convention:
- `logMessage()`, `logToolCall()`, `logFileChange()`, `logShellCommand()`
- `logRejection()`, `logDecision()`, `addTokens()`, `setSummary()`, `tag()`, `parent()`
- `build()` and `commit()` methods
- `GitStorage` with `open()`, `discover()`, `create()`, `read()`, `readManifest()`, `list()`, `delete()`
- Dual ESM/CJS build via tsup with TypeScript declarations

**Tests:** 7 tests covering model helpers, session lifecycle, token accumulation, and Git round-trips.

### Cross-SDK Compatibility

| Aspect | Rust | Python | TypeScript |
|--------|------|--------|------------|
| Git library | git2 (vendored libgit2) | pygit2 (direct binding) | Git CLI (execFileSync) |
| ID generation | uuid crate | uuid4().hex | crypto.randomUUID() |
| Ref layout | `refs/engrams/<ab>/<id>` | Same | Same |
| Object model | 5 blobs + tree + orphan commit | Same | Same |
| Serialization | serde | dataclasses + dict | TypeScript interfaces |
| Enum format | snake_case (canonical) | snake_case | snake_case |

---

## CLI Reference

All commands support `--format json` for machine-readable output and `-v` / `-vv` / `-vvv` for verbosity.

| Command | Description | Key flags |
|---------|-------------|-----------|
| `init` | Initialize engram in a Git repository | `--force`, `--remote` |
| `record` | Record an agent session via PTY wrapper | `--agent`, `--model`, `-- <cmd>` |
| `import` | Import from Claude Code or Aider | `--format`, `--auto-detect`, `--dry-run` |
| `log` | List engrams (most recent first) | `--cost`, `-n`, `--agent`, `--by-agent` |
| `show` | Show details of a specific engram | `--intent`, `--transcript`, `--operations` |
| `search` | Full-text search across engrams | `-n` |
| `trace` | Reasoning history for a file | |
| `diff` | Compare two engrams | |
| `graph` | Context graph (text or DOT) | `--depth`, `--dot` |
| `review` | Intent chain for a branch range | |
| `pr-summary` | Generate PR description from engram chain | `--format` |
| `mcp` | Start MCP server (stdio) | |
| `stats` | Aggregate statistics | |
| `blame` | Reasoning blame for a file | `-n` |
| `gc` | Garbage collect old engrams | `--older-than`, `--dry-run`, `-y` |
| `push` | Push engram refs to remote | `--dry-run` |
| `pull` | Pull engram refs and reindex | |
| `fetch` | Fetch engram refs (no reindex) | `--dry-run` |
| `reindex` | Rebuild the search index | |
| `version` | Print version | |
| `hook-handler` | *(hidden)* Internal git hook callback | |

**Output formats:** `text` (default), `json`, `markdown`

---

## Architecture and Design

### Workspace Layout

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

| Principle | Implementation |
|-----------|---------------|
| Git-native | Engrams are Git objects (blobs, trees, commits, refs). No external database. |
| Zero config remotes | Engram refs sync via standard refspecs, configured automatically on `init`. |
| Vendored dependencies | git2 with vendored libgit2 + OpenSSL. No system deps beyond a C compiler. |
| No unsafe code | `unsafe_code = "forbid"` workspace-wide. |
| Library-first | All functionality lives in library crates; the CLI is a thin wrapper. |
| Cross-platform | File locking via `fs2`, Unix-specific code guarded by `#[cfg(unix)]`. |
| Safe imports | SHA-256 content hashing prevents duplicate imports. |
| Error strategy | `thiserror` in libraries, `anyhow` in the CLI. |
| Observability | `tracing` crate, controlled via `-v` flags or `ENGRAM_LOG` env var. |

### Dependencies

| Category | Crate | Version |
|----------|-------|---------|
| Git | git2 (vendored) | 0.20 |
| Serialization | serde, serde_json, chrono, uuid | 1.x, 1.x, 0.4, 1.x |
| Search | tantivy | 0.22 |
| CLI | clap (derive) | 4.5 |
| PTY | portable-pty | 0.9 |
| Hashing | sha2 | 0.10 |
| File walking | ignore | 0.4 |
| File locking | fs2 | 0.4 |
| MCP | rmcp, tokio, schemars | 0.15, 1.x, 1.x |
| Errors | thiserror, anyhow | 2.x, 1.x |
| Tracing | tracing, tracing-subscriber | 0.1, 0.3 |

### Test Coverage

| Suite | Count |
|-------|-------|
| Rust (cargo test --workspace) | 54 |
| Python (pytest) | 10 |
| TypeScript (vitest) | 7 |
| **Total** | **71** |

### Requirements

- Rust 1.80+ and a C compiler (for vendored libgit2/OpenSSL)
- Python 3.9+ with pygit2 (for Python SDK)
- Node.js 18+ (for TypeScript SDK)

### License

Apache-2.0 OR MIT (dual-licensed)
