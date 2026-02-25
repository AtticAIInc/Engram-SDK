# engram stats

Show aggregate statistics across all engrams.

## Usage

```bash
engram stats [--by-file] [--by-branch] [--trend] [--top N]
```

## Description

Displays aggregate statistics with optional breakdowns:

- **Default**: Total engrams, breakdown by agent/mode, total tokens, estimated cost, date range
- **`--by-file`**: Cost breakdown per file (which files cost the most to develop)
- **`--by-branch`**: Cost breakdown per feature branch
- **`--trend`**: Daily cost trend over the last 30 days
- **`--top N`**: Limit breakdown entries (default: all)

Cost estimation works even when agents don't report costs directly -- engram estimates from model name and token counts using built-in API pricing tables.

## Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--by-file` | bool | false | Show cost breakdown by file |
| `--by-branch` | bool | false | Show cost breakdown by branch |
| `--trend` | bool | false | Show daily cost trend (last 30 days) |
| `--top <N>` | number | all | Limit breakdown to top N entries |

## Examples

```bash
# Aggregate totals
engram stats

# Most expensive files
engram stats --by-file --top 10

# Cost per feature branch
engram stats --by-branch

# Daily cost trend
engram stats --trend

# JSON output
engram stats --format json
```

## See Also

- [log](log.md) -- List individual engrams
- [why](why.md) -- Explain why a specific file exists
