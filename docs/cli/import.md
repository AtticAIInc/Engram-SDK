# engram import

Import sessions from Claude Code or Aider.

## Usage

```bash
engram import [path] [--import-format <format>] [--auto-detect] [--dry-run] [--no-summarize]
```

## Description

Parses existing AI agent session files and stores them as engrams. Supports:

- **Claude Code** -- JSONL session files (typically at `~/.claude/projects/<hash>/`)
- **Aider** -- Markdown chat history files (`.aider.chat.history.md`)

Duplicate detection via SHA-256 content hashing ensures re-importing the same file is safe.

When an Anthropic API key is configured (via `engram config set anthropic_api_key` or `ANTHROPIC_API_KEY` env var), import automatically enriches each session with LLM-powered summarization — generating high-quality summaries, interpreted goals, dead ends, and architectural decisions from the full transcript.

## Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--import-format <format>` | enum | auto | Format hint: `claude-code` or `aider` |
| `--auto-detect` | bool | false | Auto-discover and import all sessions from known locations |
| `--dry-run` | bool | false | Preview what would be imported without storing |
| `--no-summarize` | bool | false | Skip LLM-powered summarization (use heuristic extraction only) |

## Arguments

| Argument | Description |
|----------|-------------|
| `[path]` | Path to a specific session file or directory (optional with `--auto-detect`) |

## Examples

```bash
# Auto-detect all sessions
engram import --auto-detect

# Import a specific Claude Code session
engram import ~/.claude/projects/.../session.jsonl --import-format claude-code

# Import Aider history
engram import .aider.chat.history.md --import-format aider

# Preview without importing
engram import --auto-detect --dry-run
```

## See Also

- [config](config.md) -- Set API key for LLM summarization
- [Importing Sessions](../guides/importing-sessions.md)
- [Capture Modes](../guides/capture-modes.md)
- [record](record.md) -- Record new sessions instead of importing
