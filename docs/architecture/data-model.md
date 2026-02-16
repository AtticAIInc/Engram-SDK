# Data Model

Every engram contains five components stored as Git blobs.

## Manifest (`manifest.json`)

Compact metadata for fast listing and filtering. This is the only component read during `engram log`.

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | UUID v4 hex (32 chars, no dashes) |
| `version` | integer | Schema version (currently `1`) |
| `created_at` | ISO 8601 | Session start time |
| `finished_at` | ISO 8601 \| null | Session end time |
| `agent.name` | string | Agent identifier (e.g., `"claude-code"`) |
| `agent.model` | string \| null | Model used (e.g., `"claude-sonnet-4-5"`) |
| `agent.version` | string \| null | Agent version |
| `git_commits` | string[] | Commit SHAs produced during this session |
| `token_usage.input_tokens` | integer | Tokens sent to the model |
| `token_usage.output_tokens` | integer | Tokens generated |
| `token_usage.cache_read_tokens` | integer | Tokens read from cache |
| `token_usage.cache_write_tokens` | integer | Tokens written to cache |
| `token_usage.total_tokens` | integer | Sum of all token fields |
| `token_usage.cost_usd` | float \| null | Estimated cost in USD |
| `summary` | string \| null | One-line summary |
| `tags` | string[] | User-defined tags |
| `capture_mode` | enum | `"wrapper"`, `"import"`, or `"sdk"` |
| `source_hash` | string \| null | SHA-256 of imported source file (for deduplication) |

## Intent (`intent.md`)

Human-readable Markdown. The "why" behind the session.

```markdown
# Original Request

Add OAuth2 authentication with PKCE for our SPA

## Interpreted Goal

Implement OAuth2 authorization code flow with PKCE, including token refresh and CSRF protection.

## Dead Ends

- **passport.js**: Middleware conflict with existing Express stack
- **Auth0 SDK**: Added 2MB to bundle size, decided against

## Decisions

- **Use JWT tokens**: Stateless, works with load balancers
- **Store refresh token in httpOnly cookie**: More secure than localStorage
```

### Fields

| Field | Description |
|-------|-------------|
| Original Request | What the human asked for, in their words |
| Interpreted Goal | How the agent understood the request (optional) |
| Summary | Brief summary of what was accomplished (optional) |
| Dead Ends | Approaches tried and rejected, with reasons |
| Decisions | Architectural choices made, with rationale |

## Transcript (`transcript.jsonl`)

Full session transcript, one JSON message per line:

```jsonl
{"timestamp":"2026-02-16T10:30:00Z","role":"user","content":{"type":"text","text":"Add OAuth2 auth"},"token_count":null}
{"timestamp":"2026-02-16T10:30:05Z","role":"assistant","content":{"type":"text","text":"I'll implement OAuth2 with PKCE..."},"token_count":250}
```

### Fields per Entry

| Field | Type | Description |
|-------|------|-------------|
| `timestamp` | ISO 8601 | When the message was sent |
| `role` | string | `"user"`, `"assistant"`, `"system"`, or `"tool"` |
| `content` | object | Message content (text, tool_use, tool_result, etc.) |
| `token_count` | integer \| null | Token count for this message |

## Operations (`operations.json`)

Structured record of what the agent actually did.

```json
{
  "tool_calls": [
    {
      "timestamp": "2026-02-16T10:30:10Z",
      "tool_name": "write_file",
      "input": {"path": "src/auth.rs"},
      "output_summary": "Created auth module",
      "duration_ms": 150,
      "is_error": false
    }
  ],
  "file_changes": [
    {
      "path": "src/auth.rs",
      "change_type": "created",
      "lines_added": 45,
      "lines_removed": 0
    }
  ],
  "shell_commands": [
    {
      "timestamp": "2026-02-16T10:31:00Z",
      "command": "cargo test",
      "exit_code": 0,
      "duration_ms": 3200
    }
  ]
}
```

### Change Types

| Value | Meaning |
|-------|---------|
| `"created"` | New file |
| `"modified"` | Existing file changed |
| `"deleted"` | File removed |

## Lineage (`lineage.json`)

Relationships to other entities, forming the context graph.

```json
{
  "parent_engram": null,
  "child_engrams": [],
  "related_engrams": [],
  "git_commits": ["abc123def456"],
  "branch": "feature/oauth"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `parent_engram` | string \| null | ID of the engram this one continues from |
| `child_engrams` | string[] | IDs of engrams that follow from this one |
| `related_engrams` | object[] | Related engrams with relationship type |
| `git_commits` | string[] | Commit SHAs produced during this session |
| `branch` | string \| null | Git branch this session was on |

## EngramId

A UUID v4 in hex format (32 characters, no dashes). Example: `a1b2c3d4e5f6a7b8c9d0e1f2a3b4c5d6`

The first two characters serve as the fanout prefix for ref storage: `refs/engrams/a1/a1b2c3d4e5f6...`

IDs can be referenced by prefix (minimum 2 characters) in all CLI commands and SDK methods.
