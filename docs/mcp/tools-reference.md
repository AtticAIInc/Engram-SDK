# MCP Tools Reference

## engram_search

Full-text search across intent, transcript, file paths, and dead ends.

### Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `query` | string | Yes | | Free-text search query |
| `limit` | number | No | 10 | Maximum number of results |

### Example

```json
{
  "query": "authentication",
  "limit": 5
}
```

### Returns

Formatted search results with ID (short), agent, model, date, and summary for each match.

---

## engram_show

Show full details of a specific engram.

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | Yes | Engram ID (full or prefix), or `"HEAD"` for most recent |

### Example

```json
{
  "id": "HEAD"
}
```

### Returns

Complete engram details:
- Manifest (ID, agent, model, date, summary, tokens, cost, commits)
- Intent (original request, interpreted goal, summary)
- File changes (with +/~/- symbols for created/modified/deleted)
- Dead ends
- Decisions
- Transcript entry count

---

## engram_log

List recent engrams, most recent first.

### Parameters

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `limit` | number | No | 10 | Maximum number of entries |
| `by_agent` | string | No | | Filter by agent name |

### Example

```json
{
  "limit": 5,
  "by_agent": "claude-code"
}
```

### Returns

List of engrams with short ID, agent, model, date, token count, cost, and summary.

---

## engram_trace

Trace the full reasoning history of a file.

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `file_path` | string | Yes | File path to trace |

### Example

```json
{
  "file_path": "src/auth.rs"
}
```

### Returns

Chronological list of engrams that created, modified, or deleted the file, with ID, agent, date, and summary.

---

## engram_diff

Compare two engrams.

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id_a` | string | Yes | First engram ID (or prefix) |
| `id_b` | string | Yes | Second engram ID (or prefix) |

### Example

```json
{
  "id_a": "abc123",
  "id_b": "def456"
}
```

### Returns

- Common files (in both engrams)
- Files only in first engram
- Files only in second engram
- Token delta
- Cost delta

---

## engram_dead_ends

Surface rejected approaches and architectural decisions.

### Parameters

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `id` | string | No | Specific engram ID to get dead ends from |
| `query` | string | No | Search for dead ends matching this text |

Both parameters are optional. If neither is provided, returns dead ends from the 50 most recent engrams.

### Examples

```json
{
  "id": "abc123"
}
```

```json
{
  "query": "authentication"
}
```

### Returns

List of dead ends (approach + reason) and decisions (description + rationale), grouped by engram.
