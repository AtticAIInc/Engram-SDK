# Intent

looks like the engram mcp isnt working properly.  here are the docs https://the-attic-ai.gitbook.io/engram/mcp-integration/setup

## Interpreted Goal

The user wanted to use the Engram MCP server globally across all repositories but encountered it not showing up in Claude Code on a fresh repo. The strategy was to: (1) verify the MCP server worked locally, (2) check the global configuration, (3) discover the incorrect file path was being used, (4) use the correct `claude mcp add --scope user` command, and (5) update the misleading documentation to prevent future users from hitting the same issue.

## Summary

Diagnosed and fixed the Engram MCP server not appearing in Claude Code by identifying that the global MCP config was in the wrong file location (~/.claude/mcp.json instead of ~/.claude.json), registered it correctly using the CLI, and updated the GitBook documentation and README to reflect the correct setup procedure.

## Dead Ends

- **Testing the engram MCP server directly with JSON-RPC requests via bash**: The subprocess handling in the sandboxed shell made it difficult to properly pipe input; the server was working fine so this debugging path wasn't needed
- **Adding a project-level .mcp.json file to the demo-workflow repo**: User wanted global access across all repos, not per-project configuration

## Decisions

- **Used `claude mcp add --transport stdio --scope user engram -- engram mcp` command instead of manual config file editing**: This is the official Claude Code way to register MCP servers at user scope; it handles the correct file path (~/.claude.json) and structure automatically, avoiding the error the docs were causing
- **Removed the stale ~/.claude/mcp.json file after successful registration**: The file was never being read by Claude Code and could cause confusion; cleaning it up prevents accidental reliance on incorrect configuration
- **Updated both docs/mcp/setup.md and README.md to recommend the CLI command instead of manual config file editing**: The docs were actively misleading users by pointing to a non-existent or ignored config file; the CLI command is the canonical, correct approach that Claude Code maintains
