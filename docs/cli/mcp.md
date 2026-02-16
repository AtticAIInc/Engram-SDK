# engram mcp

Start the MCP server for AI agent integration.

## Usage

```bash
engram mcp
```

## Description

Starts a [Model Context Protocol](https://modelcontextprotocol.io) (MCP) server on stdio transport. This exposes engram data to AI agents, letting them query reasoning history during sessions.

The server auto-initializes engram if not already set up.

### Available Tools

The MCP server provides 6 tools:

| Tool | Description |
|------|-------------|
| `engram_search` | Full-text search across engrams |
| `engram_show` | Show full engram details |
| `engram_log` | List recent engrams |
| `engram_trace` | File reasoning history |
| `engram_diff` | Compare two engrams |
| `engram_dead_ends` | Surface rejected approaches |

## Examples

```bash
# Start the MCP server
engram mcp
```

For setup instructions with Claude Code and Claude Desktop, see the [MCP Integration Guide](../mcp/README.md).

## See Also

- [MCP Overview](../mcp/README.md)
- [MCP Setup](../mcp/setup.md)
- [MCP Tools Reference](../mcp/tools-reference.md)
