# Context Graph

Engrams form a **context graph** -- a semantic reasoning layer over your codebase.

## Nodes and Edges

### Node Types

| Type | Description | Example |
|------|-------------|---------|
| Engram | A reasoning session | `engram:abc123...` |
| File | A source code file | `file:src/auth.rs` |
| Agent | An AI agent | `agent:claude-code` |
| Commit | A Git commit | `commit:abc123` |

### Edge Types

| Edge | From | To | Meaning |
|------|------|----|---------|
| `modified_by` | File | Engram | This file was changed in this session |
| `used_agent` | Engram | Agent | This session used this agent |
| `touched_file` | Engram | File | This session changed this file |
| `produced_by` | Commit | Engram | This commit was produced during this session |
| `follows_from` | Engram | Engram | This session continues from a previous one |

## Viewing the Graph

### Full Graph

```bash
engram graph
```

### Centered on a Node

```bash
engram graph file:src/auth.rs
engram graph file:src/auth.rs --depth 3
```

### Graphviz Output

Export as DOT format and render as SVG:

```bash
engram graph --dot | dot -Tsvg -o graph.svg
engram graph file:src/auth.rs --dot > auth-graph.dot
```

## Use Cases

- **Impact analysis** -- Which engrams touched this file? What other files were changed alongside it?
- **Agent patterns** -- Which agents have worked on which parts of the codebase?
- **Reasoning chains** -- Follow the `follows_from` edges to trace how a feature evolved across sessions
- **Collaboration** -- See which agents and humans have touched the same files

## See Also

- [graph](../cli/graph.md) -- CLI reference
