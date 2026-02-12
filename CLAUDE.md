# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**Repository:** https://github.com/AtticAIInc/Engram-SDK

Engram captures AI agent reasoning as first-class, versioned data in Git. Each "engram" is a discrete unit of reasoning memory (intent, transcript, tool calls, rejected approaches, token economics) linked to the Git commits it produced. Engrams are stored as native Git objects under `.git/refs/engrams/` — no sidecar database.

## Build Commands

```bash
source "$HOME/.cargo/env"             # Ensure cargo is on PATH
cargo build --workspace               # Build all crates
cargo test --workspace                # Run all Rust tests (54 currently)
cargo test -p engram-core             # Test a single crate
cargo clippy --workspace -- -D warnings  # Lint (zero warnings policy)
cargo fmt --all -- --check            # Format check
cargo run -p engram-cli -- <cmd>      # Run the CLI
```

### SDK Tests

```bash
# Python SDK (requires pygit2)
cd sdks/python && pip install -e ".[dev]" && python3 -m pytest tests/ -v

# TypeScript SDK (requires Node.js 18+)
cd sdks/typescript && npm install && npx vitest run
```

**Total test count: 54 Rust + 10 Python + 7 TypeScript = 71 tests.**

## Architecture

Cargo workspace with 7 crates under `crates/`:

```
crates/engram-core/      Core library: data model, Git storage engine, config, error types, hooks
crates/engram-capture/   PTY wrapper, file change detection, session builder, importers (Claude Code, Aider)
crates/engram-query/     Tantivy full-text search index, file tracing, engram diff, context graph, branch review
crates/engram-protocol/  Push/pull/fetch engram refs between repos via Git refspecs
crates/engram-sdk/       Fluent Rust SDK: EngramSession::begin() -> log_*() -> commit()
crates/engram-mcp/       MCP server for AI agent integration (rmcp crate, stdio transport)
crates/engram-cli/       CLI binary (installed as `engram`) — 20 public subcommands + 1 hidden
sdks/python/             Python SDK (pygit2), install with pip
sdks/typescript/         TypeScript SDK (git CLI via execFileSync), install with npm
```

### CLI Commands (21 total)

`init`, `record`, `import`, `log`, `show`, `search`, `trace`, `diff`, `graph`, `review`, `pr-summary`, `mcp`, `stats`, `blame`, `gc`, `push`, `pull`, `fetch`, `reindex`, `version` (+ hidden `hook-handler`)

### engram-core structure

- `src/model/` — Data types: `EngramId`, `Manifest`, `AgentInfo`, `TokenUsage`, `Intent`, `Transcript`, `Operations`, `Lineage`, `EngramData`
  - `EngramId`: UUID v4 hex (no dashes, 32 chars). `parse()` validates >= 2 chars. `fanout_prefix()` returns first 2 chars (safe fallback to "00" for short IDs).
  - `Manifest`: includes `source_hash: Option<String>` for import deduplication (SHA-256 of source file)
  - Enum serialization: `#[serde(rename_all = "snake_case")]` — canonical format is **snake_case** (e.g. `"wrapper"`, `"created"`)
- `src/storage/` — Git storage engine:
  - `git_backend.rs` — `GitStorage`: main CRUD facade (discover/open/create/read/list/delete/find_by_source_hash). Maintains `.git/engram-head` pointer file for O(1) HEAD resolution.
  - `objects.rs` — Creates Git blobs, trees, and commits for an engram
  - `refs.rs` — Manages refs under `refs/engrams/<ab>/<full-id>` with fanout
  - `read.rs` — Reads engram data back from Git objects
- `src/config/` — `EngramConfig` in `.git/config` under `[engram]`
- `src/error.rs` — `CoreError` enum (thiserror), includes `InvalidId` variant
- `src/hooks/` — Git hook system:
  - `session.rs` — `ActiveSession` (`.git/engram-session`) with `fs2` advisory file locking for concurrent commit safety
  - `installer.rs` — Installs prepare-commit-msg/post-commit hooks, chains with existing hooks, `#[cfg(unix)]` guarded permissions
  - `handlers.rs` — Hook callbacks: commit trailer injection (`Engram-Id:` and `Engram-Agent:`)

### engram-capture structure

- `src/pty/` — PTY wrapper (portable-pty), file change detector (SHA256 snapshots via `ignore` crate — respects `.gitignore`, `.git/info/exclude`, global gitignore)
- `src/session/` — SessionBuilder: CapturedSession -> EngramData. Includes `extractor.rs` for heuristic dead-end/decision extraction from raw output.
- `src/import/` — Claude Code JSONL parser, Aider markdown parser, auto-detection. Importers compute `source_hash` for deduplication.

### engram-query structure

- `src/index/` — Tantivy schema, writer, reader, rebuild
- `src/search.rs` — High-level SearchEngine with auto-index lifecycle
- `src/trace.rs` — File reasoning trace
- `src/diff.rs` — Compare two engrams (files, tokens, cost)
- `src/graph/` — Context graph (nodes: engram/file/agent/commit; DOT output)
- `src/review.rs` — Branch review: walks git log for Engram-Id trailers

## Key Design Decisions

- **Git object model**: ref -> commit -> tree -> 5 blobs (manifest.json, intent.md, transcript.jsonl, operations.json, lineage.json)
- **EngramId**: UUID v4 hex (no dashes, 32 chars). Fanout via first 2 chars in ref path.
- **git2 crate** (v0.20, vendored-libgit2 + vendored-openssl) for all Git operations
- **Error strategy**: `thiserror` in libraries, `anyhow` in CLI
- **Tracing**: `tracing` crate, controlled via `-v` flags or `ENGRAM_LOG` env var
- **Search index**: Tantivy at `.git/engram-index/`, auto-created on first search, auto-updated on create/import
- **Sync**: engram refspecs (`refs/engrams/*`) added to Git remotes for push/fetch
- **unsafe_code = "forbid"** workspace-wide
- **Cross-SDK serialization**: Rust is canonical. Python and TypeScript SDKs must match snake_case enum values.
- **File locking**: `fs2` crate for advisory locks on `ActiveSession` (MSRV 1.80 compatible — use `fs2::FileExt::` fully-qualified calls to avoid name collision with Rust 1.89+ std methods)
- **Import dedup**: SHA-256 `source_hash` on Manifest prevents re-importing the same session file
- **MCP server**: `engram-mcp` crate uses `rmcp` (v0.15) with stdio transport. Server stores `PathBuf` not `GitStorage` because `git2::Repository` is `!Send` and rmcp requires `ServerHandler: Send + Sync + 'static`. Each tool opens the repo fresh per request. Uses `schemars` v1 (matching rmcp's dependency).

### engram-mcp structure

- `src/lib.rs` — `EngramMcpServer` struct with 6 tools: `engram_search`, `engram_show`, `engram_log`, `engram_trace`, `engram_diff`, `engram_dead_ends`. Uses rmcp `#[tool_router]`, `#[tool]`, `#[tool_handler]` macros. `run_stdio()` function starts the server.

## License

Apache-2.0 OR MIT (dual-licensed).
