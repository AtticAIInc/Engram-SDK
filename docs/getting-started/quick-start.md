# Quick Start

This guide walks you through setting up engram in under 5 minutes.

## 1. Initialize Engram

In any Git repository:

```bash
cd your-project
engram init
```

This enables all automation by default:
- **Auto-capture**: Claude Code sessions imported on commit
- **Auto-push**: Engram refs sync when you `git push`
- **Claude Code hook**: Sessions auto-imported when Claude Code exits
- **Git notes**: Reasoning attached to commits, viewable via `git loge`
- **Commit trailers**: `Engram-Id`, `Engram-Agent`, `Engram-Model`, `Engram-Tokens`, `Engram-Cost`

If you use Claude Code, you're done -- sessions are captured automatically. Read on for other capture methods.

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

Shows engrams with agent, model, token usage, and estimated cost.

### View reasoning on commits

```bash
git loge                     # Alias installed by engram init
git log --notes=engram       # Standard git equivalent
```

Shows commit trailers and git notes inline with reasoning metadata.

### View a specific engram

```bash
engram show HEAD             # Most recent engram
engram show HEAD --intent    # Just the intent
engram show HEAD --transcript  # Full transcript
```

### Search across all engrams

```bash
engram search "authentication"
engram search "database migration" -n 20
```

Full-text search across intent, transcript, file paths, and dead ends.

### Understand why a file exists

```bash
engram why src/auth.rs
```

Rich narrative tracing the file's full reasoning chain -- every session that touched it, what was tried, and what was rejected.

### Trace a file's history

```bash
engram trace src/auth.rs
```

Shows every engram that touched a file, in chronological order.

### Cost analytics

```bash
engram stats                    # Aggregate totals
engram stats --by-file --top 10 # Most expensive files
engram stats --trend            # Daily cost over last 30 days
```

### Dead-end detection

```bash
engram dead-ends --recurring    # Approaches rejected 2+ times
```

## 5. Review by Intent

Instead of line-by-line code review, review the chain of intents:

```bash
engram review main..feature-branch
```

See what was asked, what was done, what dead ends were explored, and what architectural decisions were made.

## 6. Sync with Remote

Engram refs auto-push when you `git push` (enabled by default). Manual commands are also available:

```bash
engram push     # Push engram refs to remote
engram pull     # Fetch engram refs and reindex
```

## Next Steps

- [Core Concepts](core-concepts.md) -- Understand engrams, components, and storage
- [CLI Reference](../cli/README.md) -- All 24 commands in detail
- [SDK Guides](../sdks/README.md) -- Integrate directly into your agent
- [MCP Integration](../mcp/README.md) -- Connect AI agents to reasoning history
