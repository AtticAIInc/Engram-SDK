/**
 * Data model types matching the Rust engram-core model.
 */

import { randomUUID } from "crypto";

export type CaptureMode = "wrapper" | "import" | "sdk";

export type FileChangeType = "created" | "modified" | "deleted";

export interface AgentInfo {
  name: string;
  model?: string;
  version?: string;
}

export interface TokenUsage {
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_write_tokens: number;
  total_tokens: number;
  cost_usd?: number;
}

export function defaultTokenUsage(): TokenUsage {
  return {
    input_tokens: 0,
    output_tokens: 0,
    cache_read_tokens: 0,
    cache_write_tokens: 0,
    total_tokens: 0,
  };
}

export interface DeadEnd {
  approach: string;
  reason: string;
}

export interface Decision {
  description: string;
  rationale: string;
}

export interface Intent {
  original_request: string;
  interpreted_goal?: string;
  summary?: string;
  dead_ends: DeadEnd[];
  decisions: Decision[];
}

export interface TranscriptEntry {
  timestamp: string;
  role: string;
  content: Record<string, unknown>;
  token_count?: number;
}

export interface Transcript {
  entries: TranscriptEntry[];
}

export interface ToolCall {
  timestamp: string;
  tool_name: string;
  input: unknown;
  output_summary?: string;
  duration_ms?: number;
  is_error: boolean;
}

export interface FileChange {
  path: string;
  change_type: string | { renamed: { from: string } };
  lines_added?: number;
  lines_removed?: number;
}

export interface ShellCommand {
  timestamp: string;
  command: string;
  exit_code?: number;
  duration_ms?: number;
}

export interface Operations {
  tool_calls: ToolCall[];
  file_changes: FileChange[];
  shell_commands: ShellCommand[];
}

export interface Lineage {
  parent_engram?: string;
  child_engrams: string[];
  related_engrams: unknown[];
  git_commits: string[];
  branch?: string;
}

export interface Manifest {
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

export interface EngramData {
  manifest: Manifest;
  intent: Intent;
  transcript: Transcript;
  operations: Operations;
  lineage: Lineage;
}

export function newEngramId(): string {
  return randomUUID().replace(/-/g, "");
}

export function intentToMarkdown(intent: Intent): string {
  const lines: string[] = [
    "# Intent",
    "",
    intent.original_request,
    "",
  ];

  if (intent.summary) {
    lines.push("## Summary", "", intent.summary, "");
  }

  if (intent.dead_ends.length > 0) {
    lines.push("## Dead Ends", "");
    for (const de of intent.dead_ends) {
      lines.push(`- **${de.approach}**: ${de.reason}`);
    }
    lines.push("");
  }

  if (intent.decisions.length > 0) {
    lines.push("## Decisions", "");
    for (const d of intent.decisions) {
      lines.push(`- **${d.description}**: ${d.rationale}`);
    }
    lines.push("");
  }

  return lines.join("\n");
}

export function transcriptToJsonl(transcript: Transcript): string {
  return transcript.entries.map((e) => JSON.stringify(e)).join("\n") + "\n";
}
