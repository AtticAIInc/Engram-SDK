# engram push

Push engram refs to a remote.

## Usage

```bash
engram push [remote] [--dry-run]
```

## Description

Pushes all engram refs (`refs/engrams/*`) to the specified remote. This is how reasoning data syncs alongside code -- teammates can pull engrams to see the reasoning behind commits.

## Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--dry-run` | bool | false | Preview what would be pushed |

## Arguments

| Argument | Default | Description |
|----------|---------|-------------|
| `[remote]` | `origin` | Remote name |

## Examples

```bash
# Push to origin
engram push

# Push to a specific remote
engram push upstream

# Preview what would be pushed
engram push --dry-run
```

## See Also

- [pull](pull.md) -- Pull engram refs
- [fetch](fetch.md) -- Fetch without reindexing
- [Remote Sync Guide](../guides/remote-sync.md)
