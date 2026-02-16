# Architecture Overview

Engram is organized as a Cargo workspace with 7 Rust crates, plus Python and TypeScript SDKs.

## Crate Structure

```
crates/
  engram-core/       Core library: data model, Git storage engine, config, hooks
  engram-capture/    PTY wrapper, file change detection, session importers
  engram-query/      Tantivy search index, file tracing, engram diff, context graph
  engram-protocol/   Push/pull/fetch engram refs via Git refspecs
  engram-sdk/        Fluent Rust SDK for direct agent integration
  engram-mcp/        MCP server for AI agent integration (rmcp)
  engram-cli/        CLI binary (installed as `engram`)
sdks/
  python/            Python SDK (git CLI via subprocess)
  typescript/        TypeScript SDK (git CLI via execFileSync)
```

## Dependency Graph

```
engram-cli
  ├── engram-core
  ├── engram-capture  → engram-core
  ├── engram-query    → engram-core
  ├── engram-protocol → engram-core
  ├── engram-sdk      → engram-core
  └── engram-mcp      → engram-core, engram-query
```

The CLI is a thin wrapper -- all functionality lives in library crates.

## Key Components

### engram-core

The foundation. Contains:

- **Data model** (`src/model/`) -- `EngramId`, `Manifest`, `Intent`, `Transcript`, `Operations`, `Lineage`, `EngramData`
- **Storage engine** (`src/storage/`) -- Git object creation, ref management, CRUD operations
- **Config** (`src/config/`) -- `EngramConfig` stored in `.git/config` under `[engram]`
- **Hooks** (`src/hooks/`) -- `ActiveSession` with file locking, hook installer, commit trailer injection
- **Errors** (`src/error.rs`) -- `CoreError` enum via `thiserror`

### engram-capture

Session capture layer:

- **PTY wrapper** (`src/pty/`) -- Spawns commands in pseudo-terminals, captures output
- **File detection** -- SHA-256 snapshots before/after, respects `.gitignore`
- **Session builder** (`src/session/`) -- Converts raw captured data to `EngramData`
- **Importers** (`src/import/`) -- Claude Code JSONL parser, Aider markdown parser, auto-detection

### engram-query

Query and analysis:

- **Search index** (`src/index/`) -- Tantivy schema, writer, reader, rebuild
- **Search engine** (`src/search.rs`) -- High-level search with auto-index lifecycle
- **File trace** (`src/trace.rs`) -- Chronological reasoning history per file
- **Engram diff** (`src/diff.rs`) -- Compare two engrams
- **Context graph** (`src/graph/`) -- Node/edge model, subgraph extraction, DOT output
- **Branch review** (`src/review.rs`) -- Walk git log for `Engram-Id` trailers

### engram-protocol

Remote sync:

- **Refspecs** (`src/refspec.rs`) -- Configure `refs/engrams/*` on remotes
- **Sync** (`src/sync.rs`) -- Push/pull/fetch operations with dry-run support

## Deep Dives

- [Git Object Model](git-object-model.md) -- How engrams are stored as Git objects
- [Data Model](data-model.md) -- The five components in detail
- [Design Principles](design-principles.md) -- Why it's built this way
