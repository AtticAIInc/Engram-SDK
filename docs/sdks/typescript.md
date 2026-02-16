# TypeScript SDK

## Installation

```bash
npm install @engram/sdk
```

## Quick Start

```typescript
import { EngramSession } from '@engram/sdk';

const session = EngramSession.begin('my-agent', 'claude-sonnet-4-5');
session.logMessage('user', 'Add OAuth2 authentication');
session.logMessage('assistant', 'Implementing OAuth2 with PKCE...');
session.logToolCall('write_file', { path: 'src/auth.rs' }, 'Created auth module');
session.logFileChange('src/auth.rs', 'created');
session.logRejection('passport.js', 'Middleware conflict with existing stack');
session.logDecision('Use JWT', 'Stateless, works with load balancers');
session.addTokens(1500, 800, 0.02);

const id = session.commit('abc123', 'Implemented OAuth2 with PKCE');
console.log(`Stored engram: ${id}`);
```

## Session Lifecycle

1. **Begin** -- `EngramSession.begin(name, model)` creates a session
2. **Log** -- Chain `log*` methods to record the session
3. **Commit** -- `session.commit()` stores the engram in Git

All methods return `this` for chaining:

```typescript
session
  .logMessage('user', 'Fix the bug')
  .logMessage('assistant', 'Found the issue')
  .logFileChange('src/fix.ts', 'modified')
  .addTokens(500, 200, 0.005);
```

## API Reference

### EngramSession

| Method | Signature | Description |
|--------|-----------|-------------|
| `begin` | `static begin(agentName: string, model?: string): EngramSession` | Create a new session |
| `logMessage` | `logMessage(role: string, content: string): this` | Log a conversation message |
| `logToolCall` | `logToolCall(toolName: string, input: string \| object, outputSummary?: string): this` | Log a tool invocation |
| `logFileChange` | `logFileChange(path: string, changeType: string): this` | Log a file change |
| `logShellCommand` | `logShellCommand(command: string, exitCode?: number, durationMs?: number): this` | Log a shell command |
| `logRejection` | `logRejection(approach: string, reason: string): this` | Log a dead end |
| `logDecision` | `logDecision(description: string, rationale: string): this` | Log a decision |
| `addTokens` | `addTokens(inputTokens: number, outputTokens: number, costUsd?: number): this` | Add token usage (accumulates) |
| `setSummary` | `setSummary(summary: string): this` | Set session summary |
| `tag` | `tag(tagName: string): this` | Add a tag |
| `parent` | `parent(parentId: string): this` | Set parent engram ID |
| `build` | `build(gitSha?: string, summary?: string): EngramData` | Build without storing |
| `commit` | `commit(gitSha?: string, summary?: string, storage?: GitStorage): string` | Store in Git, returns engram ID |

### GitStorage

| Method | Signature | Description |
|--------|-----------|-------------|
| `open` | `static open(path: string): GitStorage` | Open a Git repo at path |
| `discover` | `static discover(startPath?: string): GitStorage` | Discover repo from path (default: `"."`) |
| `create` | `create(data: EngramData): string` | Store engram, returns ID |
| `read` | `read(idOrPrefix: string): EngramData` | Read engram by ID or prefix |
| `readManifest` | `readManifest(idOrPrefix: string): Manifest` | Read only manifest (fast) |
| `list` | `list(): Manifest[]` | List all engrams, newest first |
| `delete` | `delete(idOrPrefix: string): void` | Delete engram by ID or prefix |

### Types

```typescript
type CaptureMode = "wrapper" | "import" | "sdk";
type FileChangeType = "created" | "modified" | "deleted";

interface AgentInfo {
  name: string;
  model?: string;
  version?: string;
}

interface TokenUsage {
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_write_tokens: number;
  total_tokens: number;
  cost_usd?: number;
}

interface Manifest {
  id: string;
  version: number;
  created_at: string;
  finished_at?: string;
  agent: AgentInfo;
  git_commits: string[];
  token_usage: TokenUsage;
  summary?: string;
  tags: string[];
  capture_mode: CaptureMode;
}

interface EngramData {
  manifest: Manifest;
  intent: Intent;
  transcript: Transcript;
  operations: Operations;
  lineage: Lineage;
}
```

### Utility Functions

| Function | Signature | Description |
|----------|-----------|-------------|
| `newEngramId` | `newEngramId(): string` | Generate UUID v4 hex (32 chars, no dashes) |
| `defaultTokenUsage` | `defaultTokenUsage(): TokenUsage` | Create zeroed TokenUsage |
| `intentToMarkdown` | `intentToMarkdown(intent: Intent): string` | Convert Intent to Markdown |
| `transcriptToJsonl` | `transcriptToJsonl(transcript: Transcript): string` | Convert Transcript to JSONL |

## See Also

- [SDK Overview](README.md) -- Cross-SDK comparison
- [Rust SDK](rust.md)
- [Python SDK](python.md)
