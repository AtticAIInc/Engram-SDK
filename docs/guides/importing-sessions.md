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

## Deduplication

Every imported session gets a `source_hash` -- a SHA-256 hash of the source file content stored in the manifest. Before importing, engram checks if an engram with the same `source_hash` already exists.

This means:
- Re-running `engram import --auto-detect` is always safe
- The same session file won't create duplicate engrams
- If the session file is modified (new messages appended), it gets a new hash and imports as a new engram

## See Also

- [import](../cli/import.md) -- CLI reference
- [Capture Modes](capture-modes.md) -- Comparison of all three modes
