# MCP Best Practices

Guidelines for AI agents using engram MCP tools effectively.

## Before Starting a Task

### Search for related work

```
engram_search: "authentication"
```

Check if there's existing reasoning about the area you're about to work on. This prevents duplicating effort and surfaces context that might not be in the code.

### Check dead ends

```
engram_dead_ends: {"query": "OAuth"}
```

Before implementing a solution, check if previous sessions already tried and rejected certain approaches. This prevents retreading abandoned paths.

## Before Modifying a File

### Trace the file's history

```
engram_trace: {"file_path": "src/auth.rs"}
```

Understand the reasoning behind the file's current state. This is especially valuable when:
- The code has non-obvious structure
- You're about to refactor
- There are comments referencing past decisions

## During a Session

### Check recent context

```
engram_log: {"limit": 5}
engram_show: {"id": "HEAD"}
```

See what the most recent sessions accomplished. This helps maintain continuity across sessions.

### Compare approaches

```
engram_diff: {"id_a": "abc123", "id_b": "def456"}
```

Compare two sessions to understand how approaches diverged. Useful when evaluating different implementations.

## Proactive Usage

The most valuable MCP usage is **proactive** -- querying reasoning history before the agent is asked to. An agent that checks dead ends before starting, traces file history before modifying, and searches for related work before implementing will produce better results than one that doesn't.

### Recommended Workflow

1. Receive task
2. `engram_search` for related prior work
3. `engram_dead_ends` for rejected approaches in this area
4. For each file to modify: `engram_trace` for reasoning history
5. Implement with full context
6. Log session via SDK or wrapper

## See Also

- [Tools Reference](tools-reference.md) -- Parameter details for all 6 tools
- [Setup](setup.md) -- Configure MCP for your agent
