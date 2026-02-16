# Developer Guide

Welcome! This guide covers everything you need to contribute to Engram.

## Repository Structure

```
crates/
  engram-core/       Core: data model, Git storage, config, hooks
  engram-capture/    PTY wrapper, file detection, importers
  engram-query/      Tantivy search, file trace, diff, graph, review
  engram-protocol/   Push/pull/fetch via Git refspecs
  engram-sdk/        Fluent Rust SDK
  engram-mcp/        MCP server (rmcp, stdio)
  engram-cli/        CLI binary
sdks/
  python/            Python SDK
  typescript/        TypeScript SDK
docs/                Documentation (this site)
```

## Quick Setup

```bash
git clone https://github.com/AtticAIInc/Engram-SDK.git
cd Engram-SDK
source "$HOME/.cargo/env"    # Ensure cargo is on PATH
cargo build --workspace
cargo test --workspace
```

## Key Conventions

- **No unsafe code** -- `unsafe_code = "forbid"` workspace-wide
- **Zero warnings** -- `cargo clippy --workspace -- -D warnings`
- **Library-first** -- All logic in library crates; CLI is a thin wrapper
- **Error strategy** -- `thiserror` in libraries, `anyhow` in CLI
- **Snake case enums** -- `#[serde(rename_all = "snake_case")]` for cross-SDK compatibility
- **Tracing** -- Use `tracing::{debug, info, warn, error}` not `println!`

## Topics

- [Building from Source](building.md) -- Build, lint, format
- [Testing](testing.md) -- Test strategy and running tests
