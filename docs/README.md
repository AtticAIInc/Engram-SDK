# Engram

**Capture agent reasoning as first-class, versioned data in Git.**

Git commits capture *what* changed but discard *why*. When AI agents code, the reasoning trail -- 100K+ tokens of explored alternatives, architectural tradeoffs, rejected approaches -- IS the institutional knowledge. Engram makes reasoning first-class, versioned, and queryable, stored in Git itself.

## What Is an Engram?

An **engram** is a discrete unit of reasoning memory: the full session transcript, human intent, agent decisions, tool calls, dead ends explored, and token economics -- linked to the Git commits it produced.

Each engram captures five components:

| Component | Format | Purpose |
|-----------|--------|---------|
| **Manifest** | `manifest.json` | Compact metadata: agent, tokens, cost, tags, timestamps |
| **Intent** | `intent.md` | Human-readable: original request, dead ends, decisions |
| **Transcript** | `transcript.jsonl` | Full session: every message, one per line |
| **Operations** | `operations.json` | Tool calls, file changes, shell commands |
| **Lineage** | `lineage.json` | Relationships to commits, branches, other engrams |

## Before and After

**Without Engram**, your Git history looks like:
```
abc123 Add OAuth2 with PKCE flow
def456 Fix middleware ordering bug
789abc Refactor auth to use sessions
```

**With Engram**, every commit carries its reasoning:
```
abc123 Add OAuth2 with PKCE flow [claude-code/claude-sonnet-4-5] $0.23 47832tok
  Intent: "Add OAuth2 authentication with PKCE for our SPA"
  Dead ends: Tried passport.js (middleware conflict), considered Auth0
             SDK (added 2MB to bundle, decided against).

def456 Fix middleware ordering bug
  Intent: "The auth middleware runs after the rate limiter, causing 401s"
  Related: Follows from abc123 (the original auth implementation)
```

## Three Ways to Capture

1. **Wrapper** -- Wrap any agent command in a PTY. Zero agent cooperation needed.
   ```bash
   engram record -- claude "add OAuth2 authentication"
   ```

2. **Import** -- Parse existing sessions from Claude Code or Aider.
   ```bash
   engram import --auto-detect
   ```

3. **SDK** -- Integrate directly into your agent with Rust, Python, or TypeScript.
   ```python
   with EngramSession("my-agent", "claude-sonnet-4-5") as session:
       session.log_message("user", "Add OAuth2 authentication")
       session.log_file_change("src/auth.rs", "created")
   ```

## Git-Native Storage

Engrams are stored as native Git objects -- they travel with `clone`, `push`, `pull`. No sidecar database, no separate sync, no vendor lock-in.

```
.git/refs/engrams/
  ab/
    abc123...  -> commit -> tree containing:
      manifest.json
      intent.md
      transcript.jsonl
      operations.json
      lineage.json
```

## Get Started

* [**Installation**](getting-started/README.md) -- Install from source in under a minute
* [**Quick Start**](getting-started/quick-start.md) -- Record your first engram in 5 minutes
* [**Core Concepts**](getting-started/core-concepts.md) -- Understand the mental model
* [**CLI Reference**](cli/README.md) -- All 21 commands
* [**SDK Guides**](sdks/README.md) -- Rust, Python, TypeScript
* [**MCP Integration**](mcp/README.md) -- Connect AI agents to reasoning history
