# CLI Overview

The `engram` CLI provides 24 commands for capturing, querying, and syncing agent reasoning.

## Installation

```bash
cargo install --path crates/engram-cli
```

## Global Flags

These flags are available on all commands:

| Flag | Short | Description |
|------|-------|-------------|
| `--verbose` | `-v` | Increase verbosity (`-v` info, `-vv` debug, `-vvv` trace) |
| `--format <format>` | | Output format: `text` (default), `json`, `markdown` |

## Commands

### Capture

| Command | Description |
|---------|-------------|
| [init](init.md) | Initialize engram with smart defaults (all automation ON) |
| [record](record.md) | Record an agent session via PTY wrapper |
| [import](import.md) | Import sessions from Claude Code or Aider |

### Query

| Command | Description |
|---------|-------------|
| [log](log.md) | List engrams (most recent first) |
| [show](show.md) | Show details of a specific engram |
| [search](search.md) | Full-text search across engrams |
| [trace](trace.md) | Show reasoning history for a file |
| [why](why.md) | Explain why a file exists through its reasoning chain |
| [diff](diff.md) | Compare two engrams |
| [graph](graph.md) | Show the context graph |
| [blame](blame.md) | Show reasoning blame for a file |
| [stats](stats.md) | Show aggregate statistics (by file, branch, trend) |
| [dead-ends](dead-ends.md) | Surface rejected approaches (supports recurring detection) |

### Annotations

| Command | Description |
|---------|-------------|
| [annotate](annotate.md) | Attach engram reasoning as git notes to commits |

### Review

| Command | Description |
|---------|-------------|
| [review](review.md) | Review intent chain for a branch range |
| [pr-summary](pr-summary.md) | Generate a PR description from engram chain |

### Sync

| Command | Description |
|---------|-------------|
| [push](push.md) | Push engram refs to a remote |
| [pull](pull.md) | Pull engram refs and reindex |
| [fetch](fetch.md) | Fetch engram refs (no reindex) |

### Maintenance

| Command | Description |
|---------|-------------|
| [gc](gc.md) | Garbage collect old engrams |
| [reindex](reindex.md) | Rebuild the search index |

### Integration

| Command | Description |
|---------|-------------|
| [mcp](mcp.md) | Start MCP server for AI agent integration |

### Info

| Command | Description |
|---------|-------------|
| [version](version.md) | Print version information |
