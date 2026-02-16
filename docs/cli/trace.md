# engram trace

Show reasoning history for a file.

## Usage

```bash
engram trace <file>
```

## Description

Shows every engram that touched a given file, in chronological order. For each match, displays the engram summary, agent, timestamp, and what changed.

## Arguments

| Argument | Description |
|----------|-------------|
| `<file>` | File path to trace |

## Examples

```bash
# Trace reasoning history for a file
engram trace src/auth.rs

# JSON output
engram trace src/middleware/oauth.rs --format json
```

## See Also

- [blame](blame.md) -- Reasoning blame for a file
- [search](search.md) -- Search by content
- [Search and Query Guide](../guides/search-and-query.md)
