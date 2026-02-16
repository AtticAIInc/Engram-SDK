# Search and Query

Engram provides full-text search powered by Tantivy, plus specialized query commands.

## Full-Text Search

```bash
engram search "authentication"
engram search "database migration" -n 20
```

### What Gets Indexed

| Field | Description |
|-------|-------------|
| `summary` | One-line summary from manifest |
| `intent` | Full intent markdown (request, dead ends, decisions) |
| `transcript` | All message content from the transcript |
| `files` | File paths from operations |
| `agent` | Agent name |
| `tags` | User-defined tags |

### Search Index

The index is stored at `.git/engram-index/` and is:

- **Auto-created** on first search if it doesn't exist
- **Incrementally updated** when creating or importing engrams
- **Fully rebuildable** with `engram reindex`

The Tantivy index uses a 50MB writer heap for fast indexing.

## File Trace

See every engram that touched a specific file, in chronological order:

```bash
engram trace src/auth.rs
```

This searches all engram operations for file changes matching the given path.

## Reasoning Blame

Like `git blame` but for reasoning -- shows which engrams are responsible for changes to a file:

```bash
engram blame src/auth.rs
engram blame src/auth.rs -n 5
```

## Engram Diff

Compare two engrams side by side:

```bash
engram diff abc123 def456
```

Shows:
- Common files touched by both sessions
- Files unique to each session
- Token usage delta
- Cost delta

## See Also

- [search](../cli/search.md) -- CLI reference
- [trace](../cli/trace.md) -- CLI reference
- [blame](../cli/blame.md) -- CLI reference
- [diff](../cli/diff.md) -- CLI reference
