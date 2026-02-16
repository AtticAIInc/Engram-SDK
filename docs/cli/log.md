# engram log

List engrams, most recent first.

## Usage

```bash
engram log [--cost] [-n <limit>] [--agent <name>] [--by-agent]
```

## Description

Lists all engrams in the repository sorted by creation time (most recent first). Shows engram ID, timestamp, agent name, model, summary, and optionally token costs.

## Flags

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--cost` | | bool | false | Show token usage and cost |
| `--limit` | `-n` | integer | 20 | Maximum number of entries to display |
| `--agent` | | string | none | Filter by agent name |
| `--by-agent` | | bool | false | Group output by agent name |

## Examples

```bash
# List recent engrams
engram log

# Show with costs
engram log --cost

# Show last 5
engram log -n 5

# Filter by agent
engram log --agent claude-code

# Group by agent
engram log --by-agent

# JSON output
engram log --format json
```

## See Also

- [show](show.md) -- View details of a specific engram
- [search](search.md) -- Search by content
- [stats](stats.md) -- Aggregate statistics
