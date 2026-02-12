import { describe, expect, it } from "vitest";
import {
  defaultTokenUsage,
  intentToMarkdown,
  newEngramId,
  transcriptToJsonl,
} from "../src/model.js";
import type { Intent, Transcript, TokenUsage } from "../src/model.js";

describe("model", () => {
  it("generates unique engram IDs", () => {
    const id1 = newEngramId();
    const id2 = newEngramId();
    expect(id1).toHaveLength(32);
    expect(id2).toHaveLength(32);
    expect(id1).not.toBe(id2);
    // Should be hex only
    expect(id1).toMatch(/^[0-9a-f]{32}$/);
  });

  it("creates default token usage", () => {
    const usage = defaultTokenUsage();
    expect(usage.input_tokens).toBe(0);
    expect(usage.output_tokens).toBe(0);
    expect(usage.total_tokens).toBe(0);
    expect(usage.cost_usd).toBeUndefined();
  });

  it("converts intent to markdown", () => {
    const intent: Intent = {
      original_request: "Add authentication",
      summary: "Added JWT auth",
      dead_ends: [{ approach: "passport.js", reason: "Middleware conflict" }],
      decisions: [{ description: "Use JWT", rationale: "Stateless" }],
    };

    const md = intentToMarkdown(intent);
    expect(md).toContain("# Intent");
    expect(md).toContain("Add authentication");
    expect(md).toContain("passport.js");
    expect(md).toContain("Use JWT");
  });

  it("converts transcript to JSONL", () => {
    const transcript: Transcript = {
      entries: [
        {
          timestamp: "2024-01-01T00:00:00Z",
          role: "user",
          content: { type: "text", text: "Hello" },
        },
        {
          timestamp: "2024-01-01T00:00:01Z",
          role: "assistant",
          content: { type: "text", text: "Hi there" },
        },
      ],
    };

    const jsonl = transcriptToJsonl(transcript);
    const lines = jsonl.trim().split("\n");
    expect(lines).toHaveLength(2);

    const first = JSON.parse(lines[0]);
    expect(first.role).toBe("user");
    expect(first.content.text).toBe("Hello");
  });
});
