# Engram: Build Document

**Capture agent reasoning as first-class, versioned data in Git.**

Version 0.1 · February 2026 · The Attic AI

---

## 1. Thesis

In the last three months, the fundamental role of the software developer has been refactored. Coding agents have become so good that in many situations it's easier to prompt than to write code yourself. The terminal is the new center of gravity. The best engineers run a dozen agents at once.

Yet we still depend on a software development lifecycle that makes code in files and folders the central artifact — in repositories and in pull requests. The concept of understanding and reviewing code is a dying paradigm. It's going to be replaced by a workflow that starts with **intent** and ends with **outcomes** expressed in natural language, product and business metrics, and assertions to validate correctness.

**The core insight:** Today's git commit is lossy. It captures the *what* (a diff) but discards the *why* (an entire reasoning session that might have consumed 100K+ tokens, explored dead ends, made architectural tradeoffs). When agents do the coding, that reasoning trail IS the institutional knowledge. Throwing it away is like burning your design docs after every sprint.

**The strategic bet:** Code is becoming commodity output. Intent, reasoning, and coordination metadata is where value accumulates. Whoever captures the reasoning layer of AI-assisted development owns the developer graph for the next decade. Engram seeds that data layer in every repo.

---

## 2. What Is an Engram?

An **engram** is a discrete unit of reasoning memory — a neuroscience term for a memory trace stored as a biophysical change in neural tissue. In our system, an engram is the complete record of an AI agent session linked to one or more git commits.

Each engram captures:

- **Intent** — What the human asked for, in their words
- **Reasoning** — The full agent transcript: every message, every decision point
- **Operations** — What the agent actually did: tool calls, file changes, shell commands
- **Dead Ends** — Things the agent tried and rejected, and why (institutional knowledge that prevents future agents from retreading abandoned paths)
- **Economics** — Token usage, cost, duration
- **Lineage** — Relationships to other engrams, forming the proto-context-graph

An engram answers the question that a git diff cannot: **Why does this code look like this?**

---

## 3. Platform Vision

Engram is the foundational data capture layer for a larger platform vision centered on three core components:

**Component 1: Git-Compatible Semantic Database**
A version-controlled system that unifies code, intent, constraints, and reasoning. Engrams are stored as native git objects — they travel with clone, push, pull. No sidecar database, no separate sync, no vendor lock-in.

**Component 2: Universal Semantic Reasoning Layer**
A context graph built from engram lineage data. Nodes are engrams, files, functions, agents, and concepts. Edges are "modified by", "motivated by", "depends on", "rejected in favor of". This enables multi-agent coordination, reasoning chain tracing, and semantic code understanding.

**Component 3: AI-Native User Interface**
A reinvention of the SDLC for agent-human collaboration. Intent-based review replaces diff-based review. Navigation by purpose replaces navigation by directory structure. The UI shows a graph of intents, features, and reasoning trails.

Engram (the CLI + SDK) is the open-source wedge that seeds component 1 in every repo, making components 2 and 3 possible.

---

## 4. Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                          engram-cli                              │
│  init · record · attach · import · log · show · search · trace  │
│  blame · stats · push · pull · review · diff · graph · serve    │
└────────────┬───────────────────┬─────────────────┬──────────────┘
             │                   │                 │
     ┌───────▼───────┐  ┌───────▼──────┐  ┌───────▼───────┐
     │ engram-capture │  │ engram-query │  │  engram-sdk   │
     │               │  │              │  │               │
     │ ┌───────────┐ │  │ ┌──────────┐ │  │  Rust SDK for │
     │ │  wrapper   │ │  │ │ Tantivy  │ │  │  direct agent │
     │ │  (PTY)     │ │  │ │ index    │ │  │  integration  │
     │ ├───────────┤ │  │ ├──────────┤ │  │               │
     │ │  import    │ │  │ │ search   │ │  └───────────────┘
     │ │  (agents)  │ │  │ │ engine   │ │
     │ ├───────────┤ │  │ ├──────────┤ │  ┌───────────────┐
     │ │  server    │ │  │ │ context  │ │  │engram-protocol│
     │ │  (HTTP)    │ │  │ │ graph    │ │  │  wire format   │
     │ └───────────┘ │  │ └──────────┘ │  └───────────────┘
     └───────┬───────┘  └──────┬───────┘
             │                 │
     ┌───────▼─────────────────▼──────┐     ┌─────────────┐
     │          engram-core           │     │ SDKs        │
     │                                │     │ ┌─────────┐ │
     │ ┌─────────┐  ┌──────────────┐ │     │ │ Python  │ │
     │ │  model   │  │ git storage  │ │     │ ├─────────┤ │
     │ │  Engram  │  │ EngramStore  │ │     │ │ TypeSc. │ │
     │ │  Intent  │  │ refs/engrams │ │     │ ├─────────┤ │
     │ │  Session │  │ git objects  │ │     │ │ Go      │ │
     │ │  Ops     │  ├──────────────┤ │     │ └─────────┘ │
     │ │  Lineage │  │ hooks        │ │     └─────────────┘
     │ └─────────┘  │ config       │ │
     │              │ trailers     │ │
     │              └──────────────┘ │
     └────────────────────────────────┘
```

### Crate Responsibilities

| Crate | Purpose | Key Types |
|-------|---------|-----------|
| `engram-core` | Data model, git storage, hooks, config | `Engram`, `EngramStore`, `Intent`, `Session`, `Operations`, `Lineage` |
| `engram-capture` | Session capture via wrapper, import, or HTTP server | `WrapperCapture`, `SessionImporter`, `CaptureServer` |
| `engram-query` | Search index, context graph, analytics | `EngramIndex`, `SearchEngine`, `ContextGraph` |
| `engram-protocol` | Wire format for agent ↔ server communication | `BeginSession`, `SessionEvent`, `EndSession` |
| `engram-sdk` | Rust SDK for direct agent integration | `EngramSession`, `CommitResult` |
| `engram-cli` | CLI binary — the user-facing interface | 18 subcommands |

---

## 5. Data Model

### 5.1 The Engram Object

The `Engram` is the central data structure. It represents a complete record of an agent session linked to one or more git commits.

```
Engram {
  id: String                    // Content-addressed ID (BLAKE3 hash)
  schema_version: {1, 0}       // Forward compatibility
  created_at: DateTime<Utc>
  commit_shas: Vec<String>      // Git commit(s) this engram attaches to

  agent: AgentInfo              // Who did the work
  intent: Intent                // Why the work was done
  session: Session              // The full reasoning record
  operations: Operations        // What was actually done
  lineage: Lineage              // Relationships to other engrams
  metadata: HashMap<K,V>        // Extensible key-value pairs
}
```

**Design decisions:**

- **Content-addressed ID**: BLAKE3 hash of the canonical serialized form. Fast, deterministic, verifiable. Enables deduplication and integrity checking.
- **Multi-commit**: An engram can attach to multiple commits (e.g., a rebase produces new SHAs for the same session). `commit_shas` is a vec, not a single field.
- **Schema version**: Enables forward compatibility. Older readers skip fields they don't understand. Migration is additive only.
- **Extensible metadata**: Agents can store custom data (MCP server configs, environment info, custom metrics) without schema changes.

### 5.2 AgentInfo

```
AgentInfo {
  name: String          // "claude-code", "aider", "cursor", "custom-agent"
  model: String         // "claude-sonnet-4-5-20250929", "gpt-4o"
  version: Option       // Agent software version
  provider: Option      // "anthropic", "openai", "google"
}
```

Intentionally flexible — must support agents and models that don't exist yet. Known agent identifiers are provided for auto-detection: `claude-code`, `aider`, `cursor`, `copilot`, `windsurf`, `continue`, `cline`.

### 5.3 Intent

The bridge between "what the human wanted" and "what the code does."

```
Intent {
  prompt: String                // Raw human instruction
  summary: String               // Agent-generated summary (past tense)
  tags: Vec<String>             // Semantic tags for search
  constraints: Vec<String>      // Rules applied ("no new deps", "must be backwards compat")
  rejections: Vec<Rejection>    // Things tried and abandoned
}

Rejection {
  approach: String              // What was considered
  reason: String                // Why it was rejected
}
```

**The `rejections` field is critical institutional knowledge.** When a future agent (or human) works on the same area, they see what was already tried and why it failed. This prevents wasted cycles retreading abandoned paths. In a multi-agent environment, this is how agents learn from each other's experiences without direct communication.

Intent renders to a human-readable `intent.md` file in the git tree for code review workflows.

### 5.4 Session

The full reasoning record: transcript, timing, and token economics.

```
Session {
  transcript: Vec<Message>      // Complete conversation
  tokens: TokenUsage            // Breakdown by category
  cost_usd: Option<f64>         // Total session cost
  duration_s: Option<f64>       // Wall-clock time
  rounds: u32                   // User→assistant turn count
}

TokenUsage {
  input: u64
  output: u64
  cache_read: u64               // Anthropic prompt caching
  cache_write: u64              // Anthropic prompt caching
  reasoning: u64                // o1/R1-style reasoning tokens
}
```

**Token accounting** must support different provider models. Anthropic has cache_read/cache_write; OpenAI has completion_tokens_details; reasoning models have separate reasoning token counts. The schema accommodates all of these.

### 5.5 Message

Messages follow a superset of the OpenAI/Anthropic message format:

```
Message {
  role: Role                    // user, assistant, system, tool
  content: Content              // Text or structured blocks
  timestamp: Option<DateTime>
  tokens: Option<u64>
  tool_call_id: Option<String>
}

Content = Text(String) | Blocks(Vec<ContentBlock>)

ContentBlock =
  | Text { text }
  | ToolUse { id, name, input }
  | ToolResult { tool_use_id, content, is_error }
  | Unknown                     // Catch-all for future provider types
```

The `Unknown` variant ensures we never lose data from provider-specific message types we haven't seen yet.

### 5.6 Operations

Everything the agent did during the session — the audit trail.

```
Operations {
  files_read: Vec<String>
  files_written: Vec<FileChange>
  tool_calls: Vec<ToolCall>
  shell_commands: Vec<ShellCommand>
  web_requests: Vec<WebRequest>
}

FileChange {
  path: String
  action: Created | Modified | Deleted | Renamed
  lines_added: Option<u32>
  lines_removed: Option<u32>
}

ToolCall {
  name: String
  input: JSON
  output: Option<JSON>
  is_error: bool
  duration_ms: Option<u64>
  timestamp: Option<DateTime>
}

ShellCommand {
  command: String
  exit_code: Option<i32>
  stdout_preview: Option<String>    // Truncated
  stderr_preview: Option<String>    // Truncated
  duration_ms: Option<u64>
}
```

Shell output is truncated to configurable limits (default 4KB) to keep engram size reasonable. Full output is captured in the transcript.

### 5.7 Lineage

The proto-context-graph. This is how engrams form a connected reasoning layer.

```
Lineage {
  parent: Option<String>            // Direct predecessor engram ID
  related: Vec<Relation>            // Cross-references
  dependencies: Vec<Dependency>     // Context consumed
  edges: Vec<Edge>                  // Semantic graph edges
}

Relation {
  engram_id: String
  kind: ContinuationOf | RefactorOf | FixFor | ConsultedFrom
        | ConflictsWith | Supersedes | Custom(String)
  description: Option<String>
}

Dependency {
  source: Engram { id } | File { path, commit_sha }
          | External { url } | Untracked { description }
  description: Option<String>
}

Edge {
  from: Node
  to: Node
  relation: String
  confidence: Option<f32>           // For auto-generated edges
}

Node = Engram { id } | File { path } | Function { file, name }
     | Concept { name } | Agent { name } | Intent { summary }
```

**Lineage grows into the full context graph over time.** The `edges` field is where auto-generated semantic relationships accumulate — built by the query engine from analyzing engram content. The `confidence` score allows distinguishing human-declared relationships (1.0) from inferred ones.

---

## 6. Git-Native Storage

### 6.1 Design Decision: Inside Git, Not Adjacent

This is the most important architectural decision in the project. Engram data lives **inside git**, not in a sidecar database, because:

1. **It travels with clone/push/pull** — no separate sync infrastructure needed
2. **It's content-addressed and immutable** — same integrity guarantees as code
3. **It participates in branching** — engram history follows code history
4. **No hosted service required** — works offline, works with any git host
5. **No vendor lock-in** — data is portable standard git objects

### 6.2 Storage Layout

```
.git/
  refs/
    engrams/
      <commit-sha[0..2]>/           # Two-char prefix for filesystem scaling
        <commit-sha>  → tree SHA    # Points to engram tree object
  objects/
    # Engram data stored as standard git blobs and trees
```

Each engram is a **git tree object** containing:

```
engram-tree/
├── manifest.json        # Compact metadata — fast reads for listing
├── intent.md            # Human-readable summary for code review
├── transcript.jsonl     # Full session — one message per line
├── operations.json      # Tool calls, file ops, shell commands
├── lineage.json         # Parent/related engram refs
└── metadata.json        # Optional extensible key-value data
```

**Why this file structure:**

- `manifest.json` is small (~1KB) and contains everything needed for `engram log` without loading full data
- `transcript.jsonl` can be large (100K+ tokens of conversation) but git's delta compression handles it well since agent sessions have repetitive structure. JSONL format allows streaming reads.
- `intent.md` is human-readable markdown for code review workflows — reviewers read the intent, not the code
- Separation means you can fetch manifests without downloading full transcripts (important for large repos)

### 6.3 Git Commit Trailers

Every commit with an attached engram gets trailer lines for discoverability in standard `git log`:

```
feat: add OAuth2 authentication flow

Engram-Id: a3f8c2d...
Engram-Agent: claude-code/claude-sonnet-4-5
Engram-Tokens: 47832
Engram-Cost: $0.23
```

Trailers are injected via the `prepare-commit-msg` git hook. They make engrams visible without any special tooling — any developer looking at `git log` sees that a commit has associated reasoning.

### 6.4 Ref Syncing

Engram refs sync alongside code using standard git refspec configuration:

```ini
[remote "origin"]
  fetch = +refs/engrams/*:refs/engrams/*
  push = refs/engrams/*:refs/engrams/*
```

This is configured automatically by `engram init`. Explicit push/pull commands are also available:

```bash
engram push [remote]    # git push origin 'refs/engrams/*:refs/engrams/*'
engram pull [remote]    # git fetch origin '+refs/engrams/*:refs/engrams/*'
```

### 6.5 Git Hooks

Two hooks are installed by `engram init`:

**`prepare-commit-msg`**: Checks for a `.engram-pending` file in `.git/`. If present, appends engram trailers to the commit message. This file is written by `engram record` when a session completes.

**`post-commit`**: After a successful commit, calls `engram attach <HEAD> --finalize` to link the pending engram data to the new commit SHA, write it to the git object store, and create the ref.

Both hooks are wrapped in marker comments (`# >>> engram` / `# <<< engram`) so they can be updated or removed without affecting other hooks.

---

## 7. CLI Specification

The CLI binary is named `engram`. All commands operate on the git repository found by walking up from the current directory.

### 7.1 Setup

```bash
engram init [--remote <name>]
```

Initializes engram support for the repository:
- Creates `refs/engrams/` namespace
- Installs git hooks (prepare-commit-msg, post-commit)
- Writes default config to `.engramrc`
- Configures the remote for automatic ref syncing
- Adds `.engram-index/` to `.gitignore`

### 7.2 Capture

```bash
engram record [--agent <name>] [--model <name>] -- <command...>
```

Wraps an agent command in a PTY for session capture (Mode 1). Auto-detects agent name from the command binary. On exit, writes pending engram data for the next commit.

```bash
engram attach <commit> [--session <path>] [--finalize]
```

Manually attaches an engram to a commit from session data. The `--finalize` flag is used internally by the post-commit hook.

```bash
engram import [<path>] [--format auto|claude-code|aider|jsonl] [--auto-detect]
```

Imports sessions from known agent formats. With `--auto-detect`, discovers importable sessions from well-known agent locations (`~/.claude/sessions/`, project-local `.aider.chat.history.md`, etc.).

### 7.3 Explore

```bash
engram log [-n <count>] [--by-agent] [--cost] [--agent <filter>]
```

Engram-aware git log. Commits with engrams are marked with `◆` and show agent name, token count, and cost. Commits without engrams show with `○`. The `--by-agent` flag groups by agent instead of chronological order.

```bash
engram show <id> [--intent] [--transcript] [--operations] [--json]
```

Full engram detail view. Flags select which sections to display. The `<id>` can be an engram ID prefix, commit SHA, or `HEAD`.

### 7.4 Query

```bash
engram search <query> [-n <limit>] [--tag]
```

Full-text search across intents, summaries, transcripts, and tags using the Tantivy index. With `--tag`, searches only tags.

```bash
engram trace <file>
```

Shows the chain of engrams that touched a file, in chronological order. This answers "why does this file look like this?" by showing the sequence of reasoning sessions that shaped it.

```bash
engram blame <file>
```

Enhanced git blame that shows intent and agent information alongside line attributions, not just commit hashes.

### 7.5 Analytics

```bash
engram stats [--weekly] [--agent <name>]
```

Aggregate statistics: total engrams, token usage, costs, per-agent breakdown. With `--weekly`, shows time-series cost tracking.

### 7.6 Sync

```bash
engram push [<remote>]
engram pull [<remote>]
```

Push/pull engram refs to/from a remote. Uses the configured default remote (default: `origin`).

```bash
engram gc
```

Garbage collect orphaned engram objects (engrams whose referenced commits no longer exist).

### 7.7 Review

```bash
engram review <commit-range> [--interactive]
```

**The paradigm shift.** Instead of reviewing diffs line-by-line, reads the chain of intents and summaries for a commit range. For each commit with an engram:

- Shows the human intent (what was asked)
- Shows the agent summary (what was done)
- Shows rejected approaches (dead ends explored)
- Shows files changed, cost, token usage
- Commits without engrams appear dimmed with "(no engram)"

In interactive mode, prompts to continue, view transcript, or approve at each step.

```bash
engram diff <engram1> <engram2>
```

Compares reasoning between two engrams. Shows intent differences, overlapping files, unique files, and cost comparison.

### 7.8 Graph

```bash
engram graph <node-id> [--depth <n>]
```

Explores the context graph from a starting node. Node IDs can be engram IDs, `file:<path>`, `agent:<name>`, or `concept:<name>`. Shows neighbors grouped by relationship type.

### 7.9 Server

```bash
engram serve [--port <port>] [--host <addr>]
```

Starts the HTTP capture server for SDK integration (default: `localhost:3271`). This is the Mode 3 target-state endpoint that agents connect to via the SDK.

```bash
engram reindex
```

Rebuilds the local Tantivy search index from all engram data.

---

## 8. Agent Capture Modes

Three modes of capturing agent sessions, progressively better in fidelity:

### 8.1 Mode 1: Wrapper (works today, any agent)

```bash
engram record -- claude "add OAuth2 authentication"
engram record -- aider --model gpt-4o
engram record -- cursor-cli "fix the bug"
```

**How it works:**
1. Snapshots all file modification times in the working directory
2. Spawns the agent command in a pseudo-terminal (PTY)
3. Captures all stdin/stdout/stderr
4. On process exit, diffs filesystem to find created/modified/deleted files
5. Packages everything into an `Engram` and writes pending data

**Limitations:**
- Cannot capture internal API calls or structured tool usage
- Cannot distinguish agent output from tool output in the raw stream
- Transcript is raw terminal output, not structured messages
- Token usage and cost are not available

**Strengths:**
- Zero agent cooperation required — works with any command-line agent today
- Filesystem diffing reliably captures file changes
- Duration tracking is accurate

### 8.2 Mode 2: Session Import

```bash
engram import --auto-detect
engram import ~/.claude/sessions/latest/ --format claude-code
engram import .aider.chat.history.md --format aider
engram import transcript.jsonl --format jsonl
```

Reads session data from known agent export formats and converts to engrams.

**Supported formats:**

| Agent | Location | Format | Fidelity |
|-------|----------|--------|----------|
| Claude Code | `~/.claude/sessions/<id>/` | JSON/JSONL | High — structured messages, tool calls |
| Aider | `.aider.chat.history.md` | Markdown | Medium — message content, no tool calls |
| Generic | Any `.jsonl` file | JSONL | Variable — depends on what's provided |

**Auto-detection:** `engram import --auto-detect` walks well-known session directories and offers to import any unprocessed sessions. The importer tracks which sessions have already been imported to avoid duplicates.

### 8.3 Mode 3: SDK Integration (target state)

Agents import a lightweight SDK and emit structured session data in real-time to the engram capture server.

**Architecture:**

```
  Agent Process                    Engram Server (localhost:3271)
  ┌───────────────┐               ┌─────────────────────────────┐
  │ Agent runtime  │  HTTP/JSON   │ POST /v1/sessions           │
  │ + engram SDK   │─────────────▶│ POST /v1/sessions/:id/msgs  │
  │                │              │ POST /v1/sessions/:id/tools │
  │                │              │ POST /v1/sessions/:id/files │
  │                │              │ POST /v1/sessions/:id/commit│
  └───────────────┘               └──────────────┬──────────────┘
                                                  │
                                          ┌───────▼───────┐
                                          │  EngramStore   │
                                          │  (git objects) │
                                          └───────────────┘
```

**Protocol lifecycle:**
1. Agent calls `POST /v1/sessions` with agent info → receives `session_id`
2. During session, agent streams events: messages, tool calls, file changes
3. On completion, agent calls `POST /v1/sessions/:id/commit` with commit SHA, summary, tags, token usage, cost, rejections
4. Server assembles the full `Engram`, writes to git object store, creates ref

**Why HTTP, not gRPC:** Simplicity wins for v1. Every language has an HTTP client. The protocol is simple enough that developers can integrate without an SDK — just POST JSON. gRPC can be added later for streaming-heavy use cases.

**Server port:** 3271 (CKPT on a phone keypad — holdover from the pre-rename days, but it's distinctive and available).

---

## 9. SDK Specification

SDKs are thin HTTP clients over the capture server protocol. Available in Rust, Python, and TypeScript.

### 9.1 Python SDK

```python
from engram import EngramSession

async with EngramSession("my-agent", "claude-sonnet-4-5") as session:
    # Log conversation
    await session.log_message("user", "Add OAuth2 authentication")
    await session.log_message("assistant", "Implementing OAuth2 with PKCE...")

    # Log tool calls
    await session.log_tool_call("write_file", {"path": "src/auth.rs"})
    await session.log_tool_call("run_tests", {"suite": "auth"}, output={"passed": 12})

    # Log file changes
    await session.log_file_change("src/auth.rs", "created", lines_added=245)

    # Record rejected approaches
    session.log_rejection("passport.js", "Middleware conflict with existing stack")

    # Finalize and attach to commit
    result = await session.commit(
        "abc123def",
        summary="Implemented OAuth2 with PKCE flow",
        tags=["auth", "security"],
        token_usage={"input": 32000, "output": 15832},
        cost_usd=0.23,
    )
    print(f"Engram {result.engram_id} attached")
```

### 9.2 TypeScript SDK

```typescript
import { EngramSession } from '@engram/sdk';

const session = await EngramSession.begin('my-agent', 'claude-sonnet-4-5');

await session.logMessage('user', 'Add OAuth2 authentication');
await session.logMessage('assistant', 'Implementing OAuth2 with PKCE...');
await session.logToolCall('write_file', { path: 'src/auth.rs' });
await session.logFileChange('src/auth.rs', 'created', { linesAdded: 245 });
session.logRejection('passport.js', 'Middleware conflict');

const result = await session.commit('abc123', {
  summary: 'Implemented OAuth2 with PKCE',
  tags: ['auth', 'security'],
  tokenUsage: { input: 32000, output: 15832 },
  costUsd: 0.23,
});
```

### 9.3 Rust SDK

```rust
use engram_sdk::EngramSession;

let mut session = EngramSession::begin("my-agent", "claude-sonnet-4-5").await?;

session.log_message("user", "Add OAuth2 authentication").await?;
session.log_message("assistant", "Implementing...").await?;
session.log_tool_call("write_file", json!({"path": "auth.rs"}), None).await?;
session.log_file_change("src/auth.rs", "created").await?;
session.log_rejection("passport.js", "Middleware conflict");

let result = session.commit("abc123", Some("Implemented OAuth2 with PKCE"), None).await?;
```

### 9.4 Integration Without an SDK

The protocol is simple enough to use directly:

```bash
# Start session
SESSION_ID=$(curl -s localhost:3271/v1/sessions \
  -d '{"agent_name":"my-agent","model":"gpt-4o"}' | jq -r .session_id)

# Log events
curl -s localhost:3271/v1/sessions/$SESSION_ID/messages \
  -d '{"role":"user","content":"Add auth"}'

# Commit
curl -s localhost:3271/v1/sessions/$SESSION_ID/commit \
  -d '{"commit_sha":"abc123","summary":"Added auth"}'
```

---

## 10. Query Engine & Context Graph

### 10.1 Local Index

The query engine maintains a local index at `.engram-index/` (gitignored) for fast queries without loading all engram data from git objects.

**Tantivy full-text index** over:
- Intent prompts and summaries
- Transcripts
- Tags
- Agent names

**Fields:** `engram_id` (stored), `commit_sha` (stored), `agent` (text + stored), `intent` (text + stored), `summary` (text + stored), `tags` (text + stored), `transcript` (text only), `created_at` (indexed + stored).

The index is rebuilt on-demand via `engram reindex` and updated incrementally when new engrams are created.

### 10.2 Search

```bash
engram search "authentication middleware"
engram search --tag security
```

Uses Tantivy's query parser with multi-field search across intent, summary, tags, and transcript. Results are ranked by relevance score.

### 10.3 Trace

```bash
engram trace src/auth.rs
```

Returns all engrams that touched a file (read or wrote), in chronological order. This is the "reasoning history" of a file — the chain of intent-driven sessions that shaped it.

### 10.4 Context Graph

The `ContextGraph` is built from engram lineage data and operations. It creates an in-memory graph where:

**Nodes:**
- Engrams (with summary labels)
- Files (with paths)
- Functions (file + name)
- Agents (name + model)
- Intents (summary text)
- Concepts (named abstractions)

**Edges:**
- `produced_by` — engram → agent
- `modified` — engram → file
- `continues` — engram → parent engram
- `consulted_from`, `refactor_of`, `fix_for`, `conflicts_with`, `supersedes` — engram → engram
- Custom semantic edges from lineage data

**Queries:**
- `neighbors(node_id)` — all 1-hop connections from a node
- `find_paths(from, to, max_depth)` — BFS path finding between nodes
- `file_history(path)` — all engrams connected to a file

The graph is currently built in-memory from loaded engrams. Future versions will use a persistent graph store (likely SQLite with adjacency list or embedded graph DB) for large repositories.

---

## 11. Configuration

### 11.1 Repository Config (`.engramrc`)

```toml
[general]
enabled = true
default_remote = "origin"

[capture]
max_transcript_bytes = 10485760   # 10MB
capture_shell_output = true
shell_preview_bytes = 4096
exclude_paths = ["node_modules/**", ".git/**", "target/**", "*.lock"]
auto_summarize = false            # Requires API access

[storage]
include_transcripts = true
auto_gc = true

[agents]
default_agent = ""

[agents.session_paths]
# Custom agent session locations
# my-agent = "/path/to/sessions"
```

### 11.2 Global Config

Located at `~/.config/engram/config.toml`. Same schema as repository config. Repository config takes precedence.

---

## 12. Tech Stack

| Component | Choice | Rationale |
|-----------|--------|-----------|
| Language | Rust | Fast, single binary, no runtime deps. Same playbook as ripgrep, delta, bat. Cross-compiles cleanly. |
| Git operations | `git` CLI plumbing + `gix` crate | Plumbing commands for writes (hash-object, mktree, update-ref). `gix` for higher-level reads. Avoids libgit2 dependency issues. |
| Full-text search | Tantivy | Rust-native Lucene equivalent. Fast indexing, good relevance ranking. |
| Structured queries | SQLite (via rusqlite) | Future use for metadata queries, analytics aggregation. |
| CLI framework | Clap (derive) | Standard Rust CLI framework. Derive macros for clean command definitions. |
| Terminal UI | console + dialoguer + indicatif + tabled | Color, prompts, progress bars, table formatting. |
| HTTP server | Axum | Modern async Rust web framework. Lightweight, tower-based. |
| PTY capture | portable-pty | Cross-platform pseudo-terminal for wrapper mode. |
| Serialization | serde + serde_json + toml | Standard Rust serialization. JSON for data, TOML for config. |
| Hashing | BLAKE3 | Fast content-addressed hashing for engram IDs. |
| Error handling | thiserror + anyhow + color-eyre | thiserror for library errors, anyhow for application code, color-eyre for CLI display. |
| Logging | tracing + tracing-subscriber | Structured logging with env-filter support. |

### Build & Distribution

| Channel | Command |
|---------|---------|
| Cargo | `cargo install engram` |
| Homebrew | `brew install engram` |
| GitHub Releases | Pre-built binaries for linux-x64, linux-arm64, macos-x64, macos-arm64 |
| GitHub Action | `uses: theatticai/engram-action@v1` for CI integration |
| npm | `npm install -g @engram/cli` (via binary wrapper) |

---

## 13. Platform Evolution Roadmap

Engram is the data capture layer. Once structured reasoning data is versioned in git, subsequent platform layers emerge:

### Layer 1: Engram CLI (v0.1 — this build)

Open-source CLI + SDK. Captures agent sessions as git-native data. Search, trace, review commands. Python/TypeScript/Rust SDKs.

**Milestone:** Any developer can run `engram init` in their repo and start capturing agent reasoning alongside commits.

### Layer 2: Context Graph Service (v0.2)

Persistent graph database index over engrams. Powered by SQLite or embedded graph store. Exposes a query API for:
- "What reasoning led to this architecture?"
- "Which sessions touched the auth module?"
- "What dead ends have been explored in this area?"

**Milestone:** `engram graph` provides actionable answers from reasoning history. CI integration annotates PRs with reasoning summaries.

### Layer 3: Multi-Agent Coordination (v0.3)

Agents read each other's engrams before starting work. Agent B sees that Agent A chose a specific middleware and why, avoiding conflicts before they happen. The context graph becomes shared memory for agent swarms.

**Key features:**
- Agent-facing query API: "What do I need to know about this area before starting?"
- Conflict detection: "Another session is modifying the same files"
- Dependency graph: "This file was last modified by engram X with intent Y"

**Milestone:** Coordinated multi-agent workflows where agents avoid redundant work and build on each other's decisions.

### Layer 4: Intent-Based Review (v0.4)

PRs are no longer diffs with descriptions. They're structured intent documents with code as evidence that intent was fulfilled. Assertions validate correctness.

**Key features:**
- `engram review` as a full PR review workflow
- Intent-to-outcome tracing
- Automated assertion generation from intent
- GitHub/GitLab integration for in-platform review

**Milestone:** Teams adopt intent-based review as their primary PR workflow, spending less time reading diffs and more time evaluating decisions.

### Layer 5: AI-Native IDE (v1.0)

The UI doesn't show files and folders as primary navigation. It shows a graph of intents, features, and reasoning trails. You navigate by what you're trying to accomplish, not by directory structure.

**Milestone:** A new developer platform where agents and humans collaborate, learn, and ship together. Open, scalable, and independent for every developer, no matter which agent or model they use.

---

## 14. Implementation Plan

### Phase 1: Core Foundation (weeks 1-3)

**Goal:** `engram init`, `engram record`, and `engram log` working end-to-end.

| Task | Crate | Priority |
|------|-------|----------|
| Engram data model with serialization | engram-core | P0 |
| EngramStore git object read/write | engram-core | P0 |
| Git hook installer | engram-core | P0 |
| Config system (.engramrc) | engram-core | P0 |
| Git trailer parse/inject | engram-core | P0 |
| PTY wrapper capture | engram-capture | P0 |
| `engram init` command | engram-cli | P0 |
| `engram record` command | engram-cli | P0 |
| `engram log` command | engram-cli | P0 |
| `engram show` command | engram-cli | P0 |
| `engram attach` command | engram-cli | P1 |
| Integration tests with real git repos | tests/ | P0 |

**Exit criteria:** A developer can `engram init`, `engram record -- claude "do something"`, commit, and see the engram in `engram log` and `engram show HEAD`.

### Phase 2: Import & Search (weeks 4-5)

**Goal:** Import existing sessions, search across reasoning history.

| Task | Crate | Priority |
|------|-------|----------|
| Claude Code session importer | engram-capture | P0 |
| Aider session importer | engram-capture | P1 |
| Generic JSONL importer | engram-capture | P0 |
| Tantivy index build/query | engram-query | P0 |
| `engram import` command | engram-cli | P0 |
| `engram search` command | engram-cli | P0 |
| `engram trace` command | engram-cli | P0 |
| `engram blame` command | engram-cli | P1 |
| `engram reindex` command | engram-cli | P0 |

**Exit criteria:** A developer can import their existing Claude Code sessions and search across them.

### Phase 3: Sync & Review (weeks 6-7)

**Goal:** Push/pull engrams between remotes, intent-based review.

| Task | Crate | Priority |
|------|-------|----------|
| Ref push/pull commands | engram-core | P0 |
| Remote auto-configuration | engram-core | P0 |
| `engram push` / `engram pull` | engram-cli | P0 |
| `engram review` command | engram-cli | P0 |
| `engram diff` command | engram-cli | P1 |
| `engram stats` command | engram-cli | P1 |
| `engram gc` command | engram-cli | P1 |

**Exit criteria:** Teams can share engrams via git remotes and use `engram review` for intent-based code review.

### Phase 4: SDK & Server (weeks 8-10)

**Goal:** Target-state agent integration via SDK and capture server.

| Task | Crate | Priority |
|------|-------|----------|
| HTTP capture server | engram-capture | P0 |
| Protocol definitions | engram-protocol | P0 |
| Rust SDK | engram-sdk | P0 |
| Python SDK | sdks/python | P0 |
| TypeScript SDK | sdks/typescript | P0 |
| `engram serve` command | engram-cli | P0 |
| Context graph (basic) | engram-query | P1 |
| `engram graph` command | engram-cli | P1 |

**Exit criteria:** An agent using the Python or TypeScript SDK can emit structured engrams through the capture server.

### Phase 5: Polish & Ship (weeks 11-12)

**Goal:** Public release.

| Task | Priority |
|------|----------|
| Cross-platform release builds (CI) | P0 |
| Homebrew formula | P0 |
| npm binary wrapper | P1 |
| Documentation site | P0 |
| README and getting-started guide | P0 |
| Demo video / walkthrough | P0 |
| GitHub Action for CI integration | P1 |
| Python SDK packaging (PyPI) | P0 |
| TypeScript SDK packaging (npm) | P0 |

**Exit criteria:** `cargo install engram` works. Docs are live. At least one demo repo with real engrams.

---

## 15. Open Questions

1. **Transcript size management.** Large agent sessions can produce transcripts exceeding 1MB. What's the right default truncation strategy? Store full, truncate on display? Or truncate at capture time with a configurable limit?

2. **Auto-summarization.** Should `engram record` optionally call an LLM to generate intent summaries automatically? This requires API keys and adds cost, but dramatically improves the review workflow.

3. **Engram deduplication.** If a developer runs the same prompt twice, should the system detect and link duplicate engrams? Content-addressed IDs handle identical sessions, but near-duplicates need fuzzy matching.

4. **GitHub/GitLab integration.** At what point do we build native integrations vs. relying on CI actions? PR annotations with engram summaries could be transformative for adoption.

5. **Graph persistence.** The current context graph is built in-memory. At what repo size does this become impractical, and what's the right persistent store (SQLite, embedded graph DB, or a graph-native format in git)?

6. **Privacy and filtering.** Agent transcripts may contain sensitive information (API keys in prompts, internal URLs in tool calls). What filtering/redaction should happen at capture time vs. display time?

7. **Naming the port.** The capture server uses port 3271 (legacy from the "checkpoints" name). Should we find a more engram-specific port number, or does it matter?

---

## 16. Success Metrics

**Adoption:**
- GitHub stars and forks
- `cargo install` / `brew install` download counts
- Number of repos with `refs/engrams/` data
- SDK adoption (PyPI / npm downloads)

**Usage:**
- Engrams captured per week across the user base
- `engram review` usage vs. traditional diff review
- Multi-agent coordination sessions
- Average engram size and transcript length

**Ecosystem:**
- Number of agents with native SDK integration
- Third-party tools built on engram data
- CI integrations deployed

**Business:**
- Conversion from open-source to hosted platform
- Enterprise interest for multi-agent coordination layer
- Developer mindshare as "the reasoning layer" for AI development
