"""Git-native storage for engrams using git CLI commands."""

from __future__ import annotations

import json
import subprocess
from pathlib import Path

from engram.model import EngramData, Manifest

# Ref layout: refs/engrams/<ab>/<full-id>
ENGRAM_REF_PREFIX = "refs/engrams/"


def _git(repo_path: str, args: list[str], input_data: bytes | None = None) -> str:
    """Run a git command and return its stdout, stripped."""
    result = subprocess.run(
        ["git"] + args,
        cwd=repo_path,
        input=input_data,
        capture_output=True,
    )
    if result.returncode != 0:
        stderr = result.stderr.decode("utf-8", errors="replace").strip()
        raise RuntimeError(f"git {args[0]} failed: {stderr}")
    return result.stdout.decode("utf-8").strip()


class GitStorage:
    """Store and retrieve engrams as native Git objects via git CLI."""

    def __init__(self, repo_path: str | Path) -> None:
        self._repo_path = str(repo_path)
        # Verify this is a git repository
        _git(self._repo_path, ["rev-parse", "--git-dir"])

    @classmethod
    def open(cls, path: str | Path) -> GitStorage:
        """Open a Git repository at the given path."""
        return cls(str(path))

    @classmethod
    def discover(cls, path: str | Path = ".") -> GitStorage:
        """Discover the Git repository from the given path."""
        toplevel = _git(str(path), ["rev-parse", "--show-toplevel"])
        return cls(toplevel)

    def create(self, data: EngramData) -> str:
        """Store an engram as Git objects and create a ref. Returns the engram ID."""
        # Serialize components
        manifest_bytes = json.dumps(data.manifest.to_dict(), indent=2).encode("utf-8")
        intent_bytes = data.intent.to_markdown().encode("utf-8")
        transcript_bytes = data.transcript.to_jsonl()
        operations_bytes = json.dumps(data.operations.to_dict(), indent=2).encode("utf-8")
        lineage_bytes = json.dumps(data.lineage.to_dict(), indent=2).encode("utf-8")

        # Create blobs
        manifest_oid = _git(self._repo_path, ["hash-object", "-w", "--stdin"], manifest_bytes)
        intent_oid = _git(self._repo_path, ["hash-object", "-w", "--stdin"], intent_bytes)
        transcript_oid = _git(self._repo_path, ["hash-object", "-w", "--stdin"], transcript_bytes)
        operations_oid = _git(self._repo_path, ["hash-object", "-w", "--stdin"], operations_bytes)
        lineage_oid = _git(self._repo_path, ["hash-object", "-w", "--stdin"], lineage_bytes)

        # Build tree via mktree (entries must be sorted alphabetically)
        tree_input = "\n".join([
            f"100644 blob {intent_oid}\tintent.md",
            f"100644 blob {lineage_oid}\tlineage.json",
            f"100644 blob {manifest_oid}\tmanifest.json",
            f"100644 blob {operations_oid}\toperations.json",
            f"100644 blob {transcript_oid}\ttranscript.jsonl",
        ]).encode("utf-8")
        tree_oid = _git(self._repo_path, ["mktree"], tree_input)

        # Create commit (standalone, no parent)
        summary = data.manifest.summary or "engram session"
        commit_oid = _git(
            self._repo_path,
            ["commit-tree", tree_oid, "-m", f"engram: {summary}"],
        )

        # Create ref
        engram_id = data.manifest.id
        ref_name = _ref_name(engram_id)
        _git(self._repo_path, ["update-ref", ref_name, commit_oid])

        return engram_id

    def read(self, id_or_prefix: str) -> EngramData:
        """Read an engram by its ID (or prefix)."""
        ref_name = self._resolve(id_or_prefix)
        commit_oid = _git(self._repo_path, ["rev-parse", ref_name])
        tree_oid = _git(self._repo_path, ["rev-parse", f"{commit_oid}^{{tree}}"])

        manifest_json = _git(self._repo_path, ["cat-file", "blob", f"{tree_oid}:manifest.json"])
        intent_md = _git(self._repo_path, ["cat-file", "blob", f"{tree_oid}:intent.md"])
        transcript_jsonl = _git(self._repo_path, ["cat-file", "blob", f"{tree_oid}:transcript.jsonl"])
        operations_json = _git(self._repo_path, ["cat-file", "blob", f"{tree_oid}:operations.json"])
        lineage_json = _git(self._repo_path, ["cat-file", "blob", f"{tree_oid}:lineage.json"])

        from engram.model import (
            Intent,
            Lineage,
            Operations,
            Transcript,
        )

        manifest = Manifest.from_dict(json.loads(manifest_json))
        intent = Intent.from_markdown(intent_md)
        transcript = Transcript.from_jsonl(transcript_jsonl.encode("utf-8"))
        operations = Operations.from_dict(json.loads(operations_json))
        lineage = Lineage.from_dict(json.loads(lineage_json))

        return EngramData(
            manifest=manifest,
            intent=intent,
            transcript=transcript,
            operations=operations,
            lineage=lineage,
        )

    def read_manifest(self, id_or_prefix: str) -> Manifest:
        """Read only the manifest (fast path)."""
        ref_name = self._resolve(id_or_prefix)
        commit_oid = _git(self._repo_path, ["rev-parse", ref_name])
        tree_oid = _git(self._repo_path, ["rev-parse", f"{commit_oid}^{{tree}}"])
        manifest_json = _git(self._repo_path, ["cat-file", "blob", f"{tree_oid}:manifest.json"])
        return Manifest.from_dict(json.loads(manifest_json))

    def list(self) -> list[Manifest]:
        """List all engrams, most recent first."""
        try:
            output = _git(
                self._repo_path,
                ["for-each-ref", "--format=%(refname)", "refs/engrams/"],
            )
        except RuntimeError:
            return []

        if not output.strip():
            return []

        manifests = []
        for ref in output.split("\n"):
            ref = ref.strip()
            if not ref:
                continue
            try:
                commit_oid = _git(self._repo_path, ["rev-parse", ref])
                tree_oid = _git(self._repo_path, ["rev-parse", f"{commit_oid}^{{tree}}"])
                manifest_json = _git(
                    self._repo_path,
                    ["cat-file", "blob", f"{tree_oid}:manifest.json"],
                )
                manifests.append(Manifest.from_dict(json.loads(manifest_json)))
            except Exception:
                continue

        manifests.sort(key=lambda m: m.created_at, reverse=True)
        return manifests

    def delete(self, id_or_prefix: str) -> None:
        """Delete an engram by removing its ref."""
        ref_name = self._resolve(id_or_prefix)
        _git(self._repo_path, ["update-ref", "-d", ref_name])

    def _resolve(self, id_or_prefix: str) -> str:
        """Resolve an engram ID or prefix to a ref name."""
        # Try exact match first
        exact_ref = _ref_name(id_or_prefix)
        try:
            _git(self._repo_path, ["rev-parse", "--verify", exact_ref])
            return exact_ref
        except RuntimeError:
            pass

        # Try prefix match
        try:
            output = _git(
                self._repo_path,
                ["for-each-ref", "--format=%(refname)", "refs/engrams/"],
            )
        except RuntimeError:
            raise KeyError(f"Engram not found: {id_or_prefix}")

        matches = []
        for ref in output.split("\n"):
            ref = ref.strip()
            if not ref:
                continue
            # Extract ID from refs/engrams/ab/full-id
            parts = ref[len(ENGRAM_REF_PREFIX):].split("/", 1)
            if len(parts) == 2:
                full_id = parts[1]
                if full_id.startswith(id_or_prefix):
                    matches.append(ref)

        if len(matches) == 0:
            raise KeyError(f"Engram not found: {id_or_prefix}")
        if len(matches) > 1:
            raise ValueError(
                f"Ambiguous engram prefix: {id_or_prefix} ({len(matches)} matches)"
            )
        return matches[0]


def _ref_name(engram_id: str) -> str:
    """Build the full ref name: refs/engrams/<ab>/<full-id>."""
    prefix = engram_id[:2]
    return f"refs/engrams/{prefix}/{engram_id}"
