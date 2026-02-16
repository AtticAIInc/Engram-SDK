# SDK Overview

Engram provides SDKs for Rust, Python, and TypeScript. All three follow the same fluent API pattern:

```
begin() → log_*() → commit()
```

## Comparison

| Feature | Rust | Python | TypeScript |
|---------|------|--------|------------|
| Package | `engram-sdk` (crate) | `engram` (pip) | `@engram/sdk` (npm) |
| Git backend | git2 (vendored libgit2) | Git CLI (subprocess) | Git CLI (execFileSync) |
| ID generation | `uuid` crate | `uuid4().hex` | `crypto.randomUUID()` |
| Context manager | No | Yes (`with` / `async with`) | No |
| Method style | `snake_case` | `snake_case` | `camelCase` |
| Token accumulation | Yes | Yes | Yes |
| Auto-discover repo | Yes | Yes | Yes |
| Requires | Rust 1.80+ | Python 3.9+ | Node.js 18+ |

## Choosing an SDK

- **Rust** -- Use if your agent is written in Rust or you need maximum performance
- **Python** -- Use for Python agents. Supports context manager for auto-commit on exit
- **TypeScript** -- Use for Node.js/TypeScript agents

## Common API Pattern

All SDKs share the same methods:

| Operation | Rust | Python | TypeScript |
|-----------|------|--------|------------|
| Create session | `EngramSession::begin(name, model)` | `EngramSession.begin(name, model)` | `EngramSession.begin(name, model)` |
| Log message | `.log_message(role, content)` | `.log_message(role, content)` | `.logMessage(role, content)` |
| Log tool call | `.log_tool_call(name, input, output)` | `.log_tool_call(name, input, output)` | `.logToolCall(name, input, output)` |
| Log file change | `.log_file_change(path, type)` | `.log_file_change(path, type)` | `.logFileChange(path, type)` |
| Log rejection | `.log_rejection(approach, reason)` | `.log_rejection(approach, reason)` | `.logRejection(approach, reason)` |
| Log decision | `.log_decision(desc, rationale)` | `.log_decision(desc, rationale)` | `.logDecision(desc, rationale)` |
| Add tokens | `.add_tokens(in, out, cost)` | `.add_tokens(in, out, cost)` | `.addTokens(in, out, cost)` |
| Store engram | `.commit(sha, summary)` | `.commit(sha, summary)` | `.commit(sha, summary)` |

## SDK Guides

- [Rust SDK](rust.md)
- [Python SDK](python.md)
- [TypeScript SDK](typescript.md)
