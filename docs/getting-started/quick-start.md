# Quick Start

This guide walks you through recording your first engram in under 5 minutes.

## 1. Initialize Engram

In any Git repository:

```bash
cd your-project
engram init
```

This installs git hooks and configures refspecs for remote sync.

## 2. Record an Agent Session

Wrap any AI agent command with `engram record`:

```bash
engram record -- claude "add OAuth2 authentication"
```

This spawns the agent in a PTY, captures its output, detects file changes, and stores the full reasoning session as an engram.

Other agents work too:

```bash
engram record -- aider --model gpt-4o
engram record -- cursor-cli "fix the login bug"
```

## 3. Import Existing Sessions

Already have Claude Code or Aider sessions? Import them:

```bash
# Auto-detect and import from known agent locations
engram import --auto-detect

# Import a specific session file
engram import ~/.claude/projects/.../session.jsonl --format claude-code

# Preview what would be imported (no changes)
engram import --dry-run
```

Re-importing the same file is safe -- duplicate detection via content hashing prevents double imports.

## 4. Explore Your Reasoning History

### List recent engrams

```bash
engram log --cost
```

Shows engrams with agent, model, token usage, and cost.

### View a specific engram

```bash
# Show the most recent engram
engram show HEAD

# Show just the intent
engram show HEAD --intent

# Show full transcript
engram show HEAD --transcript
```

### Search across all engrams

```bash
engram search "authentication"
engram search "database migration" -n 20
```

Full-text search across intent, transcript, file paths, and dead ends.

### Trace a file's reasoning history

```bash
engram trace src/auth.rs
```

Shows every engram that touched a file, in chronological order.

## 5. Review by Intent

Instead of line-by-line code review, review the chain of intents:

```bash
engram review main..feature-branch
```

See what was asked, what was done, what dead ends were explored, and what architectural decisions were made.

## 6. Sync with Remote

Push reasoning alongside code:

```bash
engram push     # Push engram refs to remote
engram pull     # Fetch engram refs and reindex
```

## Next Steps

- [Core Concepts](core-concepts.md) -- Understand engrams, components, and storage
- [CLI Reference](../cli/README.md) -- All 21 commands in detail
- [SDK Guides](../sdks/README.md) -- Integrate directly into your agent
- [MCP Integration](../mcp/README.md) -- Connect AI agents to reasoning history
