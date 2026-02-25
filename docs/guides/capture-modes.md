# Capture Modes

Engram supports four ways to capture agent reasoning, each suited to different workflows.

## Mode 1: Automatic (Claude Code)

The simplest mode -- just run `engram init` and use Claude Code normally.

```bash
engram init   # Enables all automation by default
```

### How It Works

1. When Claude Code exits, the `SessionEnd` hook fires and auto-imports the session
2. When you commit, the `prepare-commit-msg` hook auto-captures the most recent session and injects trailers
3. The `post-commit` hook links the commit SHA to the engram
4. Git notes with reasoning metadata are auto-attached to commits
5. When you `git push`, engram refs auto-sync to the remote

### Strengths

- **Zero ongoing effort** -- just use Claude Code normally
- Structured data (token usage, tool calls) from Claude Code JSONL
- Full deduplication via SHA-256 content hashing
- Commits automatically carry trailers and git notes with reasoning

### Limitations

- Only works with Claude Code (other agents need Wrapper or Import mode)
- Dead-end/decision extraction is heuristic

### When to Use

Use automatic mode whenever you use Claude Code. This is the recommended default.

## Mode 2: Wrapper (PTY)

Wrap any agent command in a pseudo-terminal to capture its session.

```bash
engram record -- claude "add OAuth2 authentication"
engram record -- aider --model gpt-4o
engram record -- cursor-cli "fix the bug"
```

### How It Works

1. Snapshots the working tree (SHA-256 hashes of all tracked files, respects `.gitignore`)
2. Spawns the command in a PTY
3. Captures all terminal output in real time
4. Re-snapshots the working tree when the command exits
5. Computes file diffs (created, modified, deleted)
6. Extracts dead ends and decisions via heuristic pattern matching on the output
7. Stores the engram and updates the search index

### Strengths

- Works with **any** agent -- zero integration effort
- Captures the raw terminal experience
- Auto-detects agent name from command (`claude`, `aider`, `cursor`, `copilot`)

### Limitations

- Token usage and cost not available (agent doesn't expose this to PTY)
- Tool calls are inferred from output, not structured
- Dead-end extraction is heuristic, not perfect

### When to Use

Use wrapper mode when you want to capture sessions from agents you don't control and can't modify.

## Mode 3: Import

Parse existing session files from Claude Code or Aider.

```bash
engram import --auto-detect
engram import ~/.claude/projects/.../session.jsonl --format claude-code
engram import .aider.chat.history.md --format aider
```

### How It Works

1. Discovers session files in known locations (or reads a specified file)
2. Parses the format-specific session data
3. Extracts messages, tool calls, file changes, token usage, and cost
4. Heuristically extracts dead ends and decisions from assistant messages
5. Computes SHA-256 hash of the source file for dedup
6. Stores the engram (skips if already imported)

### Supported Formats

| Format | File Type | Location |
|--------|-----------|----------|
| Claude Code | JSONL | `~/.claude/projects/<hash>/` |
| Aider | Markdown | `.aider.chat.history.md` |

### Strengths

- Captures **past** sessions retroactively
- Structured data (token usage, tool calls) from Claude Code
- Safe to re-run -- deduplication prevents doubles

### Limitations

- Only supports Claude Code and Aider formats currently
- Dead-end/decision extraction is heuristic

### When to Use

Use import mode to capture reasoning from past sessions, especially when you first install engram in an existing project.

## Mode 4: SDK

Integrate engram directly into your agent code.

```python
from engram import EngramSession

with EngramSession("my-agent", "claude-sonnet-4-5") as session:
    session.log_message("user", "Add OAuth2 authentication")
    session.log_tool_call("write_file", {"path": "src/auth.rs"})
    session.log_file_change("src/auth.rs", "created")
    session.log_rejection("passport.js", "Middleware conflict")
    session.add_tokens(1500, 800, 0.02)
```

### Strengths

- **Maximum data quality** -- structured tool calls, explicit dead ends, accurate token counts
- Full control over what gets recorded
- Available in Rust, Python, and TypeScript

### Limitations

- Requires modifying the agent code
- More integration effort than wrapper or import

### When to Use

Use SDK mode when you control the agent code and want the richest possible reasoning data.

## Comparison

| | Automatic | Wrapper | Import | SDK |
|---|-----------|---------|--------|-----|
| Integration effort | None | None | None | Moderate |
| Token usage | Yes | No | Yes (Claude Code) | Yes |
| Structured tool calls | Yes | No | Yes (Claude Code) | Yes |
| Explicit dead ends | Heuristic | Heuristic | Heuristic | Explicit |
| Git notes/trailers | Automatic | Via session | Manual annotate | Via session |
| Works with | Claude Code | Any agent | Claude Code, Aider | Your agent |
| Past sessions | No | No | Yes | No |

## Migration Path

A common progression:

1. **Start with Automatic** -- Just `engram init` if you use Claude Code
2. **Add Import** -- Get value from past sessions
3. **Add Wrapper** -- Capture sessions from other agents
4. **Move to SDK** -- When you want maximum data quality, integrate the SDK

## See Also

- [record](../cli/record.md) -- CLI command for wrapper mode
- [import](../cli/import.md) -- CLI command for import mode
- [SDK Guides](../sdks/README.md) -- SDK integration
