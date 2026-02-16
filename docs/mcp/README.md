# MCP Integration

Engram exposes reasoning data to AI agents via the [Model Context Protocol](https://modelcontextprotocol.io) (MCP).

## What Is MCP?

MCP is a standard protocol for connecting AI agents to external tools and data sources. When an AI agent has access to engram's MCP tools, it can query reasoning history during its own sessions -- seeing what was tried before, what was rejected, and why.

## Why Use MCP with Engram?

- **Before modifying a file**: The agent can check `engram_trace` to understand prior reasoning about that file
- **Before starting a task**: The agent can `engram_search` to find related prior work
- **To avoid repeating mistakes**: The agent can check `engram_dead_ends` for previously rejected approaches
- **To understand context**: The agent can `engram_log` and `engram_show` to see recent sessions

## Available Tools

| Tool | Description |
|------|-------------|
| `engram_search` | Full-text search across intent, transcript, file paths, and dead ends |
| `engram_show` | Show full engram details (supports `HEAD` for most recent) |
| `engram_log` | List recent engrams with token usage and cost |
| `engram_trace` | Chronological reasoning history for a specific file |
| `engram_diff` | Compare two engrams: common/unique files, token and cost deltas |
| `engram_dead_ends` | Surface rejected approaches and architectural decisions |

## Getting Started

- [Setup](setup.md) -- Configure MCP for Claude Code and Claude Desktop
- [Tools Reference](tools-reference.md) -- Detailed parameter reference for all 6 tools
- [Best Practices](best-practices.md) -- Usage patterns for AI agents
