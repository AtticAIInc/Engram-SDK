# engram diff

Compare two engrams.

## Usage

```bash
engram diff <id_a> <id_b>
```

## Description

Compares two engrams side by side, showing:

- Common files touched by both sessions
- Files unique to each session
- Token usage delta
- Cost delta

## Arguments

| Argument | Description |
|----------|-------------|
| `<id_a>` | First engram ID (or prefix) |
| `<id_b>` | Second engram ID (or prefix) |

## Examples

```bash
# Compare two engrams
engram diff abc123 def456

# Use prefixes
engram diff abc def

# JSON output
engram diff abc123 def456 --format json
```

## See Also

- [show](show.md) -- View a single engram
- [review](review.md) -- Review a range of commits
