/**
 * Fluent session builder for creating engrams programmatically.
 */

import type {
  AgentInfo,
  DeadEnd,
  Decision,
  EngramData,
  FileChange,
  ShellCommand,
  TokenUsage,
  ToolCall,
  TranscriptEntry,
} from "./model.js";
import { defaultTokenUsage, newEngramId } from "./model.js";
import { GitStorage } from "./storage.js";

const CHANGE_TYPE_MAP: Record<string, string> = {
  created: "created",
  create: "created",
  new: "created",
  modified: "modified",
  modify: "modified",
  changed: "modified",
  deleted: "deleted",
  delete: "deleted",
  removed: "deleted",
};

export class EngramSession {
  private agent: AgentInfo;
  private _transcript: TranscriptEntry[] = [];
  private _toolCalls: ToolCall[] = [];
  private _fileChanges: FileChange[] = [];
  private _shellCommands: ShellCommand[] = [];
  private _deadEnds: DeadEnd[] = [];
  private _decisions: Decision[] = [];
  private _tokenUsage: TokenUsage;
  private _originalRequest?: string;
  private _summary?: string;
  private _tags: string[] = [];
  private _parent?: string;
  private _startedAt: string;

  private constructor(agentName: string, model?: string) {
    this.agent = { name: agentName, model };
    this._tokenUsage = defaultTokenUsage();
    this._startedAt = new Date().toISOString();
  }

  /**
   * Create a new session for a given agent and optional model.
   */
  static begin(agentName: string, model?: string): EngramSession {
    return new EngramSession(agentName, model);
  }

  /**
   * Log a message (user, assistant, system, or tool).
   */
  logMessage(role: string, content: string): EngramSession {
    if (role === "user" && this._originalRequest === undefined) {
      this._originalRequest = content;
    }

    this._transcript.push({
      timestamp: new Date().toISOString(),
      role,
      content: { type: "text", text: content },
    });
    return this;
  }

  /**
   * Log a tool call.
   */
  logToolCall(
    toolName: string,
    input: string | Record<string, unknown>,
    outputSummary?: string,
  ): EngramSession {
    let parsedInput: unknown;
    if (typeof input === "string") {
      try {
        parsedInput = JSON.parse(input);
      } catch {
        parsedInput = input;
      }
    } else {
      parsedInput = input;
    }

    this._toolCalls.push({
      timestamp: new Date().toISOString(),
      tool_name: toolName,
      input: parsedInput,
      output_summary: outputSummary,
      is_error: false,
    });
    return this;
  }

  /**
   * Log a file change.
   */
  logFileChange(path: string, changeType: string): EngramSession {
    const ct = CHANGE_TYPE_MAP[changeType.toLowerCase()] || "modified";
    this._fileChanges.push({ path, change_type: ct });
    return this;
  }

  /**
   * Log a shell command execution.
   */
  logShellCommand(
    command: string,
    exitCode?: number,
    durationMs?: number,
  ): EngramSession {
    this._shellCommands.push({
      timestamp: new Date().toISOString(),
      command,
      exit_code: exitCode,
      duration_ms: durationMs,
    });
    return this;
  }

  /**
   * Log a rejected approach (dead end).
   */
  logRejection(approach: string, reason: string): EngramSession {
    this._deadEnds.push({ approach, reason });
    return this;
  }

  /**
   * Log a decision made during the session.
   */
  logDecision(description: string, rationale: string): EngramSession {
    this._decisions.push({ description, rationale });
    return this;
  }

  /**
   * Add token usage. Accumulates across multiple calls.
   */
  addTokens(
    inputTokens: number,
    outputTokens: number,
    costUsd?: number,
  ): EngramSession {
    this._tokenUsage.input_tokens += inputTokens;
    this._tokenUsage.output_tokens += outputTokens;
    this._tokenUsage.total_tokens += inputTokens + outputTokens;
    if (costUsd !== undefined) {
      this._tokenUsage.cost_usd = (this._tokenUsage.cost_usd || 0) + costUsd;
    }
    return this;
  }

  /**
   * Set a summary for this session.
   */
  setSummary(summary: string): EngramSession {
    this._summary = summary;
    return this;
  }

  /**
   * Add a tag.
   */
  tag(tagName: string): EngramSession {
    this._tags.push(tagName);
    return this;
  }

  /**
   * Set the parent engram ID.
   */
  parent(parentId: string): EngramSession {
    this._parent = parentId;
    return this;
  }

  /**
   * Build the EngramData without storing it.
   */
  build(gitSha?: string, summary?: string): EngramData {
    const engramId = newEngramId();
    const finishedAt = new Date().toISOString();
    const finalSummary = summary || this._summary || this._originalRequest;
    const gitCommits = gitSha ? [gitSha] : [];

    return {
      manifest: {
        id: engramId,
        version: 1,
        created_at: this._startedAt,
        finished_at: finishedAt,
        agent: this.agent,
        git_commits: gitCommits,
        token_usage: this._tokenUsage,
        summary: finalSummary,
        tags: this._tags,
        capture_mode: "sdk",
      },
      intent: {
        original_request: this._originalRequest || "SDK session",
        summary: finalSummary,
        dead_ends: this._deadEnds,
        decisions: this._decisions,
      },
      transcript: {
        entries: this._transcript,
      },
      operations: {
        tool_calls: this._toolCalls,
        file_changes: this._fileChanges,
        shell_commands: this._shellCommands,
      },
      lineage: {
        parent_engram: this._parent,
        child_engrams: [],
        related_engrams: [],
        git_commits: gitCommits,
      },
    };
  }

  /**
   * Finalize and store the engram in Git. Returns the engram ID.
   */
  commit(gitSha?: string, summary?: string, storage?: GitStorage): string {
    const data = this.build(gitSha, summary);
    const store = storage || GitStorage.discover();
    return store.create(data);
  }
}
