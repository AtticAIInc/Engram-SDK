# Design Principles

## 1. Git-Native

Engrams are Git objects (blobs, trees, commits, refs). No external database, no sidecar files, no separate sync mechanism. Everything lives inside `.git/` and travels with standard Git operations.

**Why:** Developers already have Git. Adding another storage system creates friction, sync problems, and vendor lock-in.

## 2. Smart Defaults

`engram init` enables all automation out of the box: auto-capture, auto-push, Claude Code hooks, git notes. Users opt out, not in.

**Why:** Most users want the full experience. Requiring manual configuration means most never discover the best features.

## 3. Zero Config Remotes

Engram refspecs (`refs/engrams/*`) are added to Git remotes during `engram init`. After that, standard `git push`/`fetch` includes engram data automatically.

**Why:** Reasoning data should travel alongside code without any extra steps.

## 4. Vendored Dependencies

The Rust crates use `git2` with vendored libgit2 and vendored OpenSSL. No system dependencies beyond a C compiler.

**Why:** Eliminates "works on my machine" problems and simplifies installation across platforms.

## 5. No Unsafe Code

`unsafe_code = "forbid"` is set workspace-wide. No crate in the workspace uses `unsafe`.

**Why:** Memory safety is non-negotiable for a tool that touches Git repositories.

## 6. Library-First

All functionality lives in library crates. The CLI (`engram-cli`) is a thin wrapper that parses arguments and calls library functions. The MCP server (`engram-mcp`) does the same.

**Why:** Enables the SDK, CLI, and MCP server to share the same implementation. Makes it easy to embed engram in other tools.

## 7. Cross-Platform

File locking uses `fs2` (advisory locks). Unix-specific code is guarded by `#[cfg(unix)]`. The Python and TypeScript SDKs use the git CLI rather than platform-specific bindings.

**Why:** Developers use macOS, Linux, and Windows. Engram should work on all of them.

## 8. Safe Imports

Duplicate detection via SHA-256 content hashing (`source_hash` on Manifest) prevents re-importing the same session file.

**Why:** Users should be able to run `engram import --auto-detect` freely without worrying about creating duplicates.

## 9. Error Strategy

Library crates use `thiserror` for typed errors. The CLI uses `anyhow` for convenient error chaining with context.

**Why:** Libraries should give callers precise error types to match on. CLIs just need readable error messages.

## 10. Observability

The `tracing` crate is used throughout, controlled via `-v` flags or the `ENGRAM_LOG` environment variable. Three verbosity levels: info, debug, trace.

**Why:** When something goes wrong, users need visibility into what engram is doing.

## 11. Forward Compatibility

The manifest includes a `version` field (currently `1`). Enum serialization uses `#[serde(rename_all = "snake_case")]` for canonical snake_case values across all three SDKs.

**Why:** The data model will evolve. Versioning and consistent serialization make migrations possible.

## 12. Cross-Repository Awareness

A global config at `~/.config/engram/repos.toml` tracks all initialized repositories. `engram search --global` searches across all of them, merging results by relevance.

**Why:** Teams and individuals work across multiple repositories. Reasoning should be searchable across project boundaries.

## 13. Embedded UIs

The dashboard (`engram dashboard`) embeds its HTML/JS SPA via `include_str!()`. The TUI (`engram browse`) uses ratatui for a zero-dependency terminal interface. Neither requires external files, build steps, or npm.

**Why:** A single binary should provide the full experience. External asset files create deployment complexity.

## Trade-offs

| Decision | Trade-off |
|----------|-----------|
| Git objects (not files) | Harder to inspect manually, but enables automatic sync |
| Orphan commits | No branch history for engrams, but keeps code history clean |
| Tantivy for search | Adds binary size, but provides fast full-text search |
| Vendored libgit2 | Slower builds, but zero system dependencies |
| 5-blob tree structure | More objects per engram, but enables reading individual components |
| Embedded SPA (include_str) | Binary is larger, but zero deployment files |
| axum for dashboard | Adds async runtime dep, but mature HTTP framework |
| ratatui for TUI | Adds terminal UI dep, but rich interactive experience |
