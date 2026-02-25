# Importing Sessions

Import existing AI agent sessions into engram.

## Auto-Detection

The fastest way to import is auto-detection:

```bash
engram import --auto-detect
```

This scans known locations for Claude Code and Aider session files, computes content hashes for deduplication, and imports any new sessions.

### Preview First

Use `--dry-run` to see what would be imported without storing anything:

```bash
engram import --auto-detect --dry-run
```

## Claude Code

### Session File Location

Claude Code stores session data as JSONL files in:

```
~/.claude/projects/<project-hash>/
```

Each file contains one JSON object per line with messages, tool calls, and metadata.

### What Gets Extracted

| Data | Source |
|------|--------|
| Messages | `user` and `assistant` entries |
| Tool calls | `tool_use` and `tool_result` content blocks |
| File changes | Extracted from `write_to_file`, `edit_file`, etc. tool calls |
| Token usage | `usage` field on assistant messages |
| Dead ends | Heuristic extraction from "tried X but Y" patterns in assistant text |
| Decisions | Heuristic extraction from "decided to X because Y" patterns |
| Interpreted goal | Extracted from first assistant response |

### Import Specific File

```bash
engram import ~/.claude/projects/abc123/session.jsonl --import-format claude-code
```

## Aider

### Session File Location

Aider stores chat history as Markdown:

```
.aider.chat.history.md
```

### Import

```bash
engram import .aider.chat.history.md --import-format aider
```

## LLM-Powered Summarization

When an Anthropic API key is configured, engram automatically sends a condensed version of each imported session to Claude Haiku to generate structured metadata:

- **Summary** -- What was accomplished (1-2 sentences focused on outcomes)
- **Interpreted goal** -- What the AI understood the user wanted and the strategy it used
- **Dead ends** -- Approaches that were actually tried and abandoned, with reasons
- **Decisions** -- Key architectural or design choices with rationale

This produces significantly higher-quality intent fields compared to the default heuristic extraction.

### Setup

```bash
# Store API key in global config (recommended)
engram config set anthropic_api_key sk-ant-api03-...

# Or set environment variable
export ANTHROPIC_API_KEY=sk-ant-api03-...
```

### Override Model

By default, summarization uses `claude-haiku-4-5-20251001` for cost efficiency. To use a different model:

```bash
engram config set summarize_model claude-sonnet-4-20250514
```

### Skip Summarization

```bash
engram import --auto-detect --no-summarize
```

### Fallback

If no API key is set or the API call fails, engram silently falls back to heuristic extraction (pattern matching on "tried X but Y", "decided to X because Y", etc.). The import always succeeds regardless of API availability.

## Deduplication

Every imported session gets a `source_hash` -- a SHA-256 hash of the source file content stored in the manifest. Before importing, engram checks if an engram with the same `source_hash` already exists.

This means:
- Re-running `engram import --auto-detect` is always safe
- The same session file won't create duplicate engrams
- If the session file is modified (new messages appended), it gets a new hash and imports as a new engram

## See Also

- [import](../cli/import.md) -- CLI reference
- [Capture Modes](capture-modes.md) -- Comparison of all three modes
