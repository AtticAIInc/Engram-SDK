# engram search

Full-text search across engrams.

## Usage

```bash
engram search <query> [-n <limit>]
```

## Description

Searches across intent, transcript content, file paths, dead ends, and tags using the Tantivy search index. The index is automatically created on first search and incrementally updated when creating or importing engrams.

## Flags

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--limit` | `-n` | integer | 10 | Maximum number of results |

## Arguments

| Argument | Description |
|----------|-------------|
| `<query>` | Search query (free-text) |

## Examples

```bash
# Search for authentication-related sessions
engram search "authentication"

# Search with more results
engram search "database migration" -n 20

# Search for a specific file
engram search "src/auth.rs"

# JSON output
engram search "OAuth" --format json
```

## See Also

- [trace](trace.md) -- Trace a specific file's history
- [blame](blame.md) -- Reasoning blame for a file
- [Search and Query Guide](../guides/search-and-query.md)
