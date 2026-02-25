# engram search

Full-text search across engrams, with optional cross-repository search.

## Usage

```bash
engram search <query> [-n <limit>] [--global] [--repos <paths>]
```

## Description

Searches across intent, transcript content, file paths, dead ends, and tags using the Tantivy search index. The index is automatically created on first search and incrementally updated when creating or importing engrams.

Supports cross-repository search via `--global` (searches all registered repos) or `--repos` (searches specific paths).

## Flags

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--limit` | `-n` | integer | 10 | Maximum number of results |
| `--global` | | boolean | false | Search across all registered repositories |
| `--repos` | | comma-separated paths | | Search specific repositories |

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

# Search across all registered repositories
engram search "database" --global

# Search specific repositories
engram search "auth" --repos /path/to/repo1,/path/to/repo2
```

## Cross-Repository Search

### Global search (`--global`)

Searches all repositories registered in `~/.config/engram/repos.toml`. Repositories are automatically registered when you run `engram init`. Results are merged and sorted by relevance score.

### Ad-hoc multi-repo search (`--repos`)

Provide a comma-separated list of repository paths to search without needing global registration.

### Output

Cross-repo results include a `[repo-name]` prefix to identify which repository each result came from.

## See Also

- [trace](trace.md) -- Trace a specific file's history
- [blame](blame.md) -- Reasoning blame for a file
- [Search and Query Guide](../guides/search-and-query.md)
