# engram fetch

Fetch engram refs from a remote without reindexing.

## Usage

```bash
engram fetch [remote] [--dry-run]
```

## Description

Fetches engram refs (`refs/engrams/*`) from the specified remote without rebuilding the search index. Use this when you want to fetch refs but don't need search yet. Run `engram reindex` later to update the index.

## Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--dry-run` | bool | false | Preview what would be fetched |

## Arguments

| Argument | Default | Description |
|----------|---------|-------------|
| `[remote]` | `origin` | Remote name |

## Examples

```bash
# Fetch from origin
engram fetch

# Fetch from a specific remote
engram fetch upstream

# Preview what would be fetched
engram fetch --dry-run
```

## See Also

- [pull](pull.md) -- Fetch and reindex in one step
- [push](push.md) -- Push engram refs
- [reindex](reindex.md) -- Rebuild search index
