# engram graph

Show the context graph.

## Usage

```bash
engram graph [node] [--depth <n>] [--dot]
```

## Description

Displays the context graph -- a semantic reasoning layer over your codebase. Nodes are engrams, files, agents, and commits. Edges represent relationships like "modified by", "used agent", "touched file", and "produced by".

Without a center node, displays the entire graph. With a center node, shows only the subgraph within the specified depth.

## Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--depth <n>` | integer | 2 | Traversal depth from center node |
| `--dot` | bool | false | Output Graphviz DOT format |

## Arguments

| Argument | Description |
|----------|-------------|
| `[node]` | Optional center node (e.g., `file:src/auth.rs` or an engram ID prefix) |

## Examples

```bash
# Show full graph
engram graph

# Show graph centered on a file
engram graph file:src/auth.rs

# Increase depth
engram graph file:src/auth.rs --depth 3

# Export as SVG via Graphviz
engram graph --dot | dot -Tsvg -o graph.svg

# Export subgraph
engram graph file:src/auth.rs --dot > auth-graph.dot
```

## See Also

- [Context Graph Guide](../guides/context-graph.md)
- [trace](trace.md) -- Simpler file history view
