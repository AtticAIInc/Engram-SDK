# engram record

Record an agent session by wrapping a command in a PTY.

## Usage

```bash
engram record [--agent <name>] [--model <model>] -- <command> [args...]
```

## Description

Spawns the given command in a pseudo-terminal (PTY) to capture its full session. The workflow:

1. Snapshots the working tree (SHA-256 hashes, respects `.gitignore`)
2. Creates an `ActiveSession` with a file lock
3. Spawns the command in a PTY and captures all output
4. Re-snapshots the working tree to detect file changes
5. Extracts dead ends and decisions via heuristic pattern matching
6. Stores the engram and updates the search index

The agent name is auto-detected from the command if not specified (recognizes `claude`, `aider`, `cursor`, `copilot`, etc.).

## Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--agent <name>` | string | auto-detected | Agent name |
| `--model <model>` | string | none | Model name (e.g., `claude-sonnet-4-5`, `gpt-4o`) |

## Arguments

| Argument | Description |
|----------|-------------|
| `<command> [args...]` | Command and arguments to run (after `--`) |

## Examples

```bash
# Record a Claude Code session
engram record -- claude "add OAuth2 authentication"

# Record an Aider session
engram record -- aider --model gpt-4o

# Specify agent and model explicitly
engram record --agent my-agent --model custom-model -- ./run-agent.sh
```

## See Also

- [Capture Modes](../guides/capture-modes.md)
- [import](import.md) -- Import existing sessions instead of recording
