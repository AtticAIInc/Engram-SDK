import { execSync } from "child_process";
import { mkdtempSync, rmSync } from "fs";
import { tmpdir } from "os";
import { join } from "path";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { EngramSession } from "../src/session.js";
import { GitStorage } from "../src/storage.js";

describe("EngramSession", () => {
  it("builds EngramData without storing", () => {
    const session = EngramSession.begin("test-agent", "gpt-4");
    session
      .logMessage("user", "Add auth to the API")
      .logMessage("assistant", "I'll add JWT auth.")
      .logToolCall("write_file", '{"path":"src/auth.rs"}', "Created auth module")
      .logFileChange("src/auth.rs", "created")
      .logRejection("Session auth", "Too stateful")
      .logDecision("Use JWT", "Stateless, works with load balancers")
      .addTokens(1500, 800, 0.02)
      .tag("auth");

    const data = session.build("abc123", "Add JWT authentication");

    expect(data.manifest.agent.name).toBe("test-agent");
    expect(data.manifest.agent.model).toBe("gpt-4");
    expect(data.manifest.capture_mode).toBe("sdk");
    expect(data.manifest.summary).toBe("Add JWT authentication");
    expect(data.manifest.token_usage.input_tokens).toBe(1500);
    expect(data.manifest.token_usage.output_tokens).toBe(800);
    expect(data.manifest.token_usage.total_tokens).toBe(2300);
    expect(data.manifest.token_usage.cost_usd).toBeCloseTo(0.02);
    expect(data.manifest.tags).toEqual(["auth"]);

    expect(data.intent.original_request).toBe("Add auth to the API");
    expect(data.intent.dead_ends).toHaveLength(1);
    expect(data.intent.decisions).toHaveLength(1);

    expect(data.transcript.entries).toHaveLength(2);
    expect(data.operations.tool_calls).toHaveLength(1);
    expect(data.operations.file_changes).toHaveLength(1);
    expect(data.operations.file_changes[0].change_type).toBe("created");

    expect(data.lineage.git_commits).toEqual(["abc123"]);
  });

  it("accumulates tokens across calls", () => {
    const session = EngramSession.begin("test");
    session.addTokens(100, 50, 0.01).addTokens(200, 100, 0.02);

    const data = session.build();
    expect(data.manifest.token_usage.input_tokens).toBe(300);
    expect(data.manifest.token_usage.output_tokens).toBe(150);
    expect(data.manifest.token_usage.total_tokens).toBe(450);
    expect(data.manifest.token_usage.cost_usd).toBeCloseTo(0.03);
  });

  describe("with git repo", () => {
    let tmpDir: string;

    beforeEach(() => {
      tmpDir = mkdtempSync(join(tmpdir(), "engram-ts-test-"));
      execSync("git init", { cwd: tmpDir });
      execSync('git config user.name "Test"', { cwd: tmpDir });
      execSync('git config user.email "test@test.com"', { cwd: tmpDir });
      // Create initial commit
      execSync("git commit --allow-empty -m 'init'", { cwd: tmpDir });
    });

    afterEach(() => {
      rmSync(tmpDir, { recursive: true, force: true });
    });

    it("stores and reads back an engram", () => {
      const storage = GitStorage.open(tmpDir);

      const session = EngramSession.begin("test-agent", "claude-sonnet");
      session
        .logMessage("user", "Fix the login bug")
        .logMessage("assistant", "Found the issue in auth.rs")
        .addTokens(500, 200, 0.005);

      const id = session.commit(undefined, "Fixed login bug", storage);
      expect(id).toHaveLength(32);

      // Read back manifest
      const manifest = storage.readManifest(id);
      expect(manifest.agent.name).toBe("test-agent");
      expect(manifest.summary).toBe("Fixed login bug");

      // List
      const manifests = storage.list();
      expect(manifests).toHaveLength(1);
      expect(manifests[0].id).toBe(id);

      // Delete
      storage.delete(id);
      expect(storage.list()).toHaveLength(0);
    });
  });
});
