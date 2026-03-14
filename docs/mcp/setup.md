# MCP Setup

## Claude Code

### Per-Repository (recommended)

Repos that include a `.mcp.json` file are auto-configured -- Claude Code discovers and starts the engram MCP server automatically.

Create `.mcp.json` at the repo root:

```json
{
  "mcpServers": {
    "engram": {
      "command": "engram",
      "args": ["mcp"],
      "env": {
        "PATH": "${HOME}/.cargo/bin:${PATH}"
      }
    }
  }
}
```

### Global (all projects)

Use the Claude Code CLI to register engram at user scope:

```bash
claude mcp add --transport stdio --scope user engram -- engram mcp
```

This adds engram to `~/.claude.json` so it's available in every project without needing a `.mcp.json` file.

## Claude Desktop

Add to your Claude Desktop config:

**macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`

**Linux/Windows:** `~/.config/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "engram": {
      "command": "engram",
      "args": ["mcp"]
    }
  }
}
```

## Verification

After configuring, the MCP server starts automatically when the AI agent launches. You can verify by asking the agent to list its available tools -- it should include `engram_search`, `engram_show`, etc.

## Manual Start

To start the MCP server manually (e.g., for debugging):

```bash
engram mcp
```

This starts the server on stdio transport. The server auto-initializes engram if not already set up.

## See Also

- [Tools Reference](tools-reference.md) -- All 6 tools
- [Best Practices](best-practices.md) -- Usage patterns
- [mcp](../cli/mcp.md) -- CLI reference
