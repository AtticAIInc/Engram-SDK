# engram show

Show details of a specific engram.

## Usage

```bash
engram show <id> [--intent] [--transcript] [--operations]
```

## Description

Displays the full contents of an engram by ID. Use `HEAD` to show the most recent engram. By default shows all sections; use flags to show only a specific section.

## Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--intent` | bool | false | Show only the intent section |
| `--transcript` | bool | false | Show only the transcript (as JSONL) |
| `--operations` | bool | false | Show only operations (tool calls, file changes) |

## Arguments

| Argument | Description |
|----------|-------------|
| `<id>` | Engram ID, prefix, or `HEAD` for most recent |

## Examples

```bash
# Show the most recent engram
engram show HEAD

# Show just the intent
engram show HEAD --intent

# Show full transcript
engram show abc123 --transcript

# Show operations
engram show abc123 --operations

# Use a prefix
engram show abc1

# JSON output
engram show HEAD --format json
```

## See Also

- [log](log.md) -- List engrams to find IDs
- [diff](diff.md) -- Compare two engrams
