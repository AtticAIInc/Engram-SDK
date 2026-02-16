# engram blame

Show reasoning blame for a file.

## Usage

```bash
engram blame <file> [-n <limit>]
```

## Description

Shows which engrams are responsible for changes to a file, similar to `git blame` but at the reasoning level. For each engram that touched the file, displays the agent, intent, and summary.

## Flags

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--limit` | `-n` | integer | 20 | Maximum number of results |

## Arguments

| Argument | Description |
|----------|-------------|
| `<file>` | File path to blame |

## Examples

```bash
# Blame a file
engram blame src/auth.rs

# Limit results
engram blame src/auth.rs -n 5

# JSON output
engram blame src/auth.rs --format json
```

## See Also

- [trace](trace.md) -- Chronological file history
- [search](search.md) -- Search by content
