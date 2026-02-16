# engram pull

Pull engram refs from a remote and reindex.

## Usage

```bash
engram pull [remote]
```

## Description

Fetches engram refs from the specified remote and rebuilds the search index to include the new engrams. This is equivalent to `engram fetch` + `engram reindex`.

## Arguments

| Argument | Default | Description |
|----------|---------|-------------|
| `[remote]` | `origin` | Remote name |

## Examples

```bash
# Pull from origin
engram pull

# Pull from a specific remote
engram pull upstream
```

## See Also

- [push](push.md) -- Push engram refs
- [fetch](fetch.md) -- Fetch without reindexing
- [reindex](reindex.md) -- Rebuild search index manually
- [Remote Sync Guide](../guides/remote-sync.md)
