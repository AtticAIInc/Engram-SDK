/**
 * Git-native storage for engrams using git CLI commands.
 */

import { execFileSync, execSync } from "child_process";
import type { EngramData, Intent, Lineage, Manifest, Operations, Transcript } from "./model.js";
import { intentToMarkdown, transcriptToJsonl } from "./model.js";

const ENGRAM_REF_PREFIX = "refs/engrams/";

function refName(engramId: string): string {
  const prefix = engramId.slice(0, 2);
  return `refs/engrams/${prefix}/${engramId}`;
}

function gitCmd(repoPath: string, args: string[], input?: string): string {
  const opts: Parameters<typeof execFileSync>[2] = {
    cwd: repoPath,
    encoding: "utf-8" as const,
    maxBuffer: 50 * 1024 * 1024,
    stdio: ["pipe", "pipe", "pipe"],
  };
  if (input) {
    (opts as Record<string, unknown>).input = input;
  }
  return execFileSync("git", args, opts).toString().trim();
}

export class GitStorage {
  private repoPath: string;

  constructor(repoPath: string) {
    this.repoPath = repoPath;
  }

  static open(path: string): GitStorage {
    return new GitStorage(path);
  }

  static discover(startPath: string = "."): GitStorage {
    const toplevel = execFileSync("git", ["rev-parse", "--show-toplevel"], {
      cwd: startPath,
      encoding: "utf-8",
    }).toString().trim();
    return new GitStorage(toplevel);
  }

  /**
   * Store an engram as Git objects and create a ref. Returns the engram ID.
   */
  create(data: EngramData): string {
    const manifestJson = JSON.stringify(data.manifest, null, 2);
    const intentMd = intentToMarkdown(data.intent);
    const transcriptJsonl = transcriptToJsonl(data.transcript);
    const operationsJson = JSON.stringify(data.operations, null, 2);
    const lineageJson = JSON.stringify(data.lineage, null, 2);

    // Create blobs
    const manifestOid = gitCmd(this.repoPath, ["hash-object", "-w", "--stdin"], manifestJson);
    const intentOid = gitCmd(this.repoPath, ["hash-object", "-w", "--stdin"], intentMd);
    const transcriptOid = gitCmd(this.repoPath, ["hash-object", "-w", "--stdin"], transcriptJsonl);
    const operationsOid = gitCmd(this.repoPath, ["hash-object", "-w", "--stdin"], operationsJson);
    const lineageOid = gitCmd(this.repoPath, ["hash-object", "-w", "--stdin"], lineageJson);

    // Build tree via mktree
    const treeInput = [
      `100644 blob ${intentOid}\tintent.md`,
      `100644 blob ${lineageOid}\tlineage.json`,
      `100644 blob ${manifestOid}\tmanifest.json`,
      `100644 blob ${operationsOid}\toperations.json`,
      `100644 blob ${transcriptOid}\ttranscript.jsonl`,
    ].join("\n");

    const treeOid = gitCmd(this.repoPath, ["mktree"], treeInput);

    // Create commit
    const summary = data.manifest.summary || "engram session";
    const commitOid = gitCmd(
      this.repoPath,
      ["commit-tree", treeOid, "-m", `engram: ${summary}`],
    );

    // Create ref
    const ref = refName(data.manifest.id);
    gitCmd(this.repoPath, ["update-ref", ref, commitOid]);

    return data.manifest.id;
  }

  /**
   * Read an engram by its ID (or prefix). Parses all 5 blobs.
   */
  read(idOrPrefix: string): EngramData {
    const resolvedRef = this.resolve(idOrPrefix);
    const commitOid = gitCmd(this.repoPath, ["rev-parse", resolvedRef]);

    // Get tree from commit
    const treeOid = gitCmd(this.repoPath, ["rev-parse", `${commitOid}^{tree}`]);

    // Read each blob
    const manifestJson = gitCmd(this.repoPath, ["cat-file", "blob", `${treeOid}:manifest.json`]);
    const intentMd = gitCmd(this.repoPath, ["cat-file", "blob", `${treeOid}:intent.md`]);
    const transcriptJsonl = gitCmd(this.repoPath, ["cat-file", "blob", `${treeOid}:transcript.jsonl`]);
    const operationsJson = gitCmd(this.repoPath, ["cat-file", "blob", `${treeOid}:operations.json`]);
    const lineageJson = gitCmd(this.repoPath, ["cat-file", "blob", `${treeOid}:lineage.json`]);

    const manifest: Manifest = JSON.parse(manifestJson);
    const intent = parseIntentMarkdown(intentMd);
    const transcript = parseTranscriptJsonl(transcriptJsonl);
    const operations: Operations = JSON.parse(operationsJson);
    const lineage: Lineage = JSON.parse(lineageJson);

    return { manifest, intent, transcript, operations, lineage };
  }

  /**
   * Read only the manifest (fast path).
   */
  readManifest(idOrPrefix: string): Manifest {
    const resolvedRef = this.resolve(idOrPrefix);
    const commitOid = gitCmd(this.repoPath, ["rev-parse", resolvedRef]);
    const treeOid = gitCmd(this.repoPath, ["rev-parse", `${commitOid}^{tree}`]);
    const manifestJson = gitCmd(this.repoPath, ["cat-file", "blob", `${treeOid}:manifest.json`]);
    return JSON.parse(manifestJson);
  }

  /**
   * List all engrams, most recent first.
   */
  list(): Manifest[] {
    let output: string;
    try {
      output = gitCmd(
        this.repoPath,
        ["for-each-ref", "--format=%(refname)", "refs/engrams/"],
      );
    } catch {
      return [];
    }

    if (!output.trim()) return [];

    const manifests: Manifest[] = [];
    for (const ref of output.split("\n")) {
      if (!ref.trim()) continue;
      try {
        const commitOid = gitCmd(this.repoPath, ["rev-parse", ref]);
        const treeOid = gitCmd(this.repoPath, ["rev-parse", `${commitOid}^{tree}`]);
        const manifestJson = gitCmd(this.repoPath, ["cat-file", "blob", `${treeOid}:manifest.json`]);
        manifests.push(JSON.parse(manifestJson));
      } catch {
        continue;
      }
    }

    manifests.sort(
      (a, b) => new Date(b.created_at).getTime() - new Date(a.created_at).getTime(),
    );

    return manifests;
  }

  /**
   * Delete an engram by removing its ref.
   */
  delete(idOrPrefix: string): void {
    const resolvedRef = this.resolve(idOrPrefix);
    gitCmd(this.repoPath, ["update-ref", "-d", resolvedRef]);
  }

  private resolve(idOrPrefix: string): string {
    // Try exact match
    const exactRef = refName(idOrPrefix);
    try {
      gitCmd(this.repoPath, ["rev-parse", "--verify", exactRef]);
      return exactRef;
    } catch {
      // fall through
    }

    // Try prefix match
    let output: string;
    try {
      output = gitCmd(
        this.repoPath,
        ["for-each-ref", "--format=%(refname)", "refs/engrams/"],
      );
    } catch {
      throw new Error(`Engram not found: ${idOrPrefix}`);
    }

    const matches: string[] = [];
    for (const ref of output.split("\n")) {
      if (!ref.trim()) continue;
      const parts = ref.replace(ENGRAM_REF_PREFIX, "").split("/");
      if (parts.length === 2 && parts[1].startsWith(idOrPrefix)) {
        matches.push(ref);
      }
    }

    if (matches.length === 0) {
      throw new Error(`Engram not found: ${idOrPrefix}`);
    }
    if (matches.length > 1) {
      throw new Error(`Ambiguous engram prefix: ${idOrPrefix} (${matches.length} matches)`);
    }

    return matches[0];
  }
}

/**
 * Parse intent.md Markdown back into an Intent object.
 */
function parseIntentMarkdown(md: string): Intent {
  let originalRequest = "";
  let interpretedGoal: string | undefined;
  let summary: string | undefined;
  const deadEnds: Intent["dead_ends"] = [];
  const decisions: Intent["decisions"] = [];

  let currentSection = "intent";
  let currentContent = "";

  for (const line of md.split("\n")) {
    if (line.startsWith("# Intent")) {
      currentSection = "intent";
      currentContent = "";
      continue;
    } else if (line.startsWith("## Original Request")) {
      // backward compat: treat as intent section
      saveSection();
      currentSection = "intent";
      currentContent = "";
      continue;
    } else if (line.startsWith("## Interpreted Goal")) {
      saveSection();
      currentSection = "goal";
      currentContent = "";
      continue;
    } else if (line.startsWith("## Summary")) {
      saveSection();
      currentSection = "summary";
      currentContent = "";
      continue;
    } else if (line.startsWith("## Dead Ends")) {
      saveSection();
      currentSection = "dead_ends";
      currentContent = "";
      continue;
    } else if (line.startsWith("## Decisions")) {
      saveSection();
      currentSection = "decisions";
      currentContent = "";
      continue;
    }

    if (currentSection === "dead_ends") {
      const match = line.match(/^- \*\*(.+?)\*\*: (.+)$/);
      if (match) {
        deadEnds.push({ approach: match[1], reason: match[2] });
      }
    } else if (currentSection === "decisions") {
      const match = line.match(/^- \*\*(.+?)\*\*: (.+)$/);
      if (match) {
        decisions.push({ description: match[1], rationale: match[2] });
      }
    } else {
      if (currentContent || line) {
        if (currentContent) currentContent += "\n";
        currentContent += line;
      }
    }
  }

  saveSection();

  return {
    original_request: originalRequest,
    interpreted_goal: interpretedGoal,
    summary,
    dead_ends: deadEnds,
    decisions,
  };

  function saveSection() {
    const trimmed = currentContent.trim();
    if (!trimmed) return;
    switch (currentSection) {
      case "intent":
        originalRequest = trimmed;
        break;
      case "goal":
        interpretedGoal = trimmed;
        break;
      case "summary":
        summary = trimmed;
        break;
    }
  }
}

/**
 * Parse transcript.jsonl back into a Transcript object.
 */
function parseTranscriptJsonl(jsonl: string): Transcript {
  const entries = jsonl
    .split("\n")
    .filter((line) => line.trim())
    .map((line) => JSON.parse(line));
  return { entries };
}
