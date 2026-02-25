# engram dead-ends

Surface rejected approaches and find recurring dead ends across sessions.

## Usage

```bash
engram dead-ends [--recurring] [--query <text>] [--id <id>] [--limit N]
```

## Description

Shows approaches that were tried and rejected during agent sessions. With `--recurring`, identifies approaches rejected multiple times across different sessions -- a signal to stop trying that approach.

## Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--recurring` | bool | false | Only show approaches rejected 2+ times |
| `--query <text>` | string | | Filter dead ends by text |
| `--id <id>` | string | | Show dead ends from a specific engram |
| `--limit <N>` | number | 50 | Maximum number of engrams to scan |

## Examples

```bash
# List all dead ends
engram dead-ends

# Find approaches rejected multiple times
engram dead-ends --recurring

# Filter by text
engram dead-ends --query "authentication"

# Dead ends from a specific engram
engram dead-ends --id abc123

# JSON output
engram dead-ends --format json
```

## See Also

- [search](search.md) -- Full-text search across engrams
- [why](why.md) -- Explain why a file exists
