# Intent

looks like the engram mcp isnt working properly.  here are the docs https://the-attic-ai.gitbook.io/engram/mcp-integration/setup

## Interpreted Goal

Debug why the engram MCP server wasn't appearing in Claude Code across different repositories, identify the root cause (incorrect config file location), register it properly using the Claude CLI, and update the documentation to prevent future confusion.

## Summary

Fixed the engram MCP configuration to work globally across all repos by correcting the setup documentation and verifying the user-scoped MCP registration was properly configured in `~/.claude.json`.

## Dead Ends

- **Testing the MCP server directly with JSON-RPC requests via bash**: The server was working fine locally; the issue wasn't with the server itself but with Claude Code's configuration
- **Checking for project-level `.mcp.json` files as the solution**: User wanted global access across all repos, not per-project configuration

## Decisions

- **Use `claude mcp add --transport stdio --scope user engram -- engram mcp` instead of manually editing config files**: The CLI command ensures the config is written to the correct location (`~/.claude.json` under `mcpServers`) and avoids the common mistake of trying to use `~/.claude/mcp.json` which Claude Code doesn't read
- **Updated both `docs/mcp/setup.md` and `README.md` to recommend the CLI command instead of manual file configuration**: The original documentation contained incorrect instructions that directed users to the wrong config file location, causing the exact issue the user encountered
- **Fixed a broken table row in README.md while making MCP documentation changes**: Noticed pre-existing formatting issue that should be corrected alongside the MCP documentation fixes in the same commit
