# engram gc

Garbage collect old engrams.

## Usage

```bash
engram gc [--older-than <duration>] [--dry-run] [-y]
```

## Description

Deletes engrams older than a specified duration. Removes the engram refs; the Git objects will be cleaned up by the next `git gc`.

## Flags

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--older-than <duration>` | | string | none | Age threshold (e.g., `30d`, `4w`, `6m`, `1y`) |
| `--dry-run` | | bool | false | Preview what would be deleted |
| `--yes` | `-y` | bool | false | Skip confirmation prompt |

### Duration Format

| Unit | Meaning | Example |
|------|---------|---------|
| `d` | days | `30d` = 30 days |
| `w` | weeks | `4w` = 4 weeks |
| `m` | months | `6m` = 6 months |
| `y` | years | `1y` = 1 year |

## Examples

```bash
# Preview what would be deleted
engram gc --older-than 90d --dry-run

# Delete engrams older than 6 months
engram gc --older-than 6m -y

# Delete all engrams older than 1 year
engram gc --older-than 1y -y
```

## See Also

- [log](log.md) -- List engrams to review before GC
- [stats](stats.md) -- See total counts before cleanup
