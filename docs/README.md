# Engram

**Capture agent reasoning as first-class, versioned data in Git.**

Every day, AI agents produce thousands of lines of code — but the reasoning behind those changes vanishes the moment the session ends. The explored alternatives, the architectural tradeoffs, the rejected approaches, the "we tried X but it broke Y" moments — all gone. Git commits tell you *what* changed. Engram captures *why*.

## Why Engram?

AI-assisted development creates a new class of institutional knowledge that traditional tools can't capture:

- **A developer leaves the team.** Their AI sessions contained deep context about why the auth system uses custom middleware instead of Auth0. Without Engram, that reasoning is lost forever.
- **A bug appears in production.** `git blame` shows the line was written by Claude, but why? What alternatives were considered? What constraints led to this approach? Engram links every line to the full reasoning chain.
- **The same dead end gets explored three times.** Different team members (or the same AI agent in different sessions) try passport.js, hit the same middleware conflict, and waste tokens rediscovering the same failure. Engram surfaces recurring dead ends across all sessions.
- **A PR needs review.** Instead of reading 500 lines of diff, the reviewer sees the intent chain: what was requested, what strategy the AI chose, what it tried and rejected, and what architectural decisions were made.

Engram turns ephemeral AI reasoning into permanent, searchable, version-controlled knowledge — stored in Git itself.

## What Is an Engram?

An **engram** is a discrete unit of reasoning memory: the full session transcript, human intent, agent decisions, tool calls, dead ends explored, and token economics — linked to the Git commits it produced.

Each engram captures five components:

| Component | Format | Purpose |
|-----------|--------|---------|
| **Manifest** | `manifest.json` | Compact metadata: agent, tokens, cost, tags, timestamps |
| **Intent** | `intent.md` | Human-readable: original request, interpreted goal, dead ends, decisions |
| **Transcript** | `transcript.jsonl` | Full session: every message, tool call, and thinking block |
| **Operations** | `operations.json` | Tool calls, file changes, shell commands |
| **Lineage** | `lineage.json` | Relationships to commits, branches, other engrams |

## What You Can Do

### Query the reasoning behind any file or line

```bash
engram why src/auth.rs           # Why does this file exist?
engram why src/auth.rs:42        # Why does this specific line exist?
engram trace src/auth.rs         # Full chronological reasoning history
engram blame src/auth.rs         # Which AI sessions changed which parts
```

### Search across all AI reasoning ever captured

```bash
engram search "authentication"           # Full-text search across all sessions
engram search "OAuth" --global           # Search across all your repositories
engram dead-ends --recurring             # Find approaches rejected 2+ times
```

### Understand cost and effort

```bash
engram stats                             # Total tokens, cost, session count
engram stats --by-file --top 10          # Which files cost the most AI effort
engram stats --trend                     # Daily cost trend over 30 days
engram dashboard --serve --open          # Interactive web dashboard
```

### Review by intent, not just by diff

```bash
engram review main..feature-branch       # Intent chain for a branch
engram pr-summary main..HEAD --format md # Auto-generate PR descriptions
engram audit v1.0..v2.0 --format csv     # Compliance: map commits to reasoning
```

### Let AI agents learn from their own history

```bash
engram mcp                               # MCP server for AI agent integration
```

With the MCP server, AI agents can query past reasoning *during their own sessions* — checking what was tried before, what failed, and why. This prevents repeated mistakes and builds on prior work.

## Before and After

**Without Engram**, your Git history looks like:
```
abc123 Add OAuth2 with PKCE flow
def456 Fix middleware ordering bug
789abc Refactor auth to use sessions
```

**With Engram** (`git loge`), every commit carries its reasoning:
```
abc123 Add OAuth2 with PKCE flow

    Engram-Id: a1b2c3d4...
    Engram-Agent: claude-code
    Engram-Model: claude-sonnet-4-5
    Engram-Tokens: 47832
    Engram-Cost: $0.23

Notes (engram):
    [claude-code/claude-sonnet-4-5] $0.23 47832tok
    Intent: "Add OAuth2 authentication with PKCE for our SPA"
    Summary: Implemented OAuth2 with PKCE using custom middleware
    Dead ends:
      - passport.js: Middleware conflict with existing stack
    Decisions:
      - Custom middleware over Auth0 SDK: Auth0 added 2MB to bundle
    Files: +auth.rs +oauth.rs ~api.rs
```

## Four Ways to Capture

1. **Automatic** -- Just run `engram init` and use Claude Code. Sessions are auto-captured on exit and on commit. Zero configuration required.

2. **Wrapper** -- Wrap any agent command in a PTY. Zero agent cooperation needed.
   ```bash
   engram record -- claude "add OAuth2 authentication"
   ```

3. **Import** -- Parse existing sessions from Claude Code or Aider. Retroactively capture past work.
   ```bash
   engram import --auto-detect
   ```

4. **SDK** -- Integrate directly into your agent with Rust, Python, or TypeScript.
   ```python
   with EngramSession("my-agent", "claude-sonnet-4-5") as session:
       session.log_message("user", "Add OAuth2 authentication")
       session.log_file_change("src/auth.rs", "created")
   ```

## LLM-Powered Summarization

When you set an API key, engram uses Claude to generate high-quality structured summaries of each session at import time — extracting the real intent, key decisions, and rejected approaches from the full transcript. This dramatically improves the quality of search results, dashboard insights, and `engram why` explanations.

```bash
engram config set anthropic_api_key sk-ant-...
```

Without an API key, engram falls back to heuristic extraction. With it, every imported session gets rich, accurate metadata.

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

## Interactive Exploration

Browse your reasoning history through multiple interfaces:

- **Web dashboard** (`engram dashboard --serve`) -- 6-tab SPA with engram browser, cost trends, file analytics, git notes viewer, transcript browser, and force-directed context graph
- **Terminal UI** (`engram browse`) -- Split-panel TUI with search, keyboard navigation, and inline detail view
- **MCP server** (`engram mcp`) -- AI agents query reasoning history during their own sessions
- **CLI** -- 29 commands for every query and operation

## Get Started

* [**Installation**](getting-started/README.md) -- Install from source in under a minute
* [**Quick Start**](getting-started/quick-start.md) -- Set up in 5 minutes
* [**Core Concepts**](getting-started/core-concepts.md) -- Understand the mental model
* [**CLI Reference**](cli/README.md) -- All 29 commands
* [**SDK Guides**](sdks/README.md) -- Rust, Python, TypeScript
* [**MCP Integration**](mcp/README.md) -- Connect AI agents to reasoning history
