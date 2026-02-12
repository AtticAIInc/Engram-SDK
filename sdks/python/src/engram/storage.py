"""Git-native storage for engrams using pygit2."""

from __future__ import annotations

import json
from pathlib import Path

import pygit2

from engram.model import EngramData, Manifest

# Ref layout: refs/engrams/<ab>/<full-id>
ENGRAM_REF_PREFIX = "refs/engrams/"


class GitStorage:
    """Store and retrieve engrams as native Git objects."""

    def __init__(self, repo: pygit2.Repository) -> None:
        self._repo = repo

    @classmethod
    def open(cls, path: str | Path) -> GitStorage:
        """Open a Git repository at the given path."""
        repo = pygit2.Repository(str(path))
        return cls(repo)

    @classmethod
    def discover(cls, path: str | Path = ".") -> GitStorage:
        """Discover the Git repository from the given path."""
        repo_path = pygit2.discover_repository(str(path))
        if repo_path is None:
            raise RuntimeError(f"No Git repository found at {path}")
        repo = pygit2.Repository(repo_path)
        return cls(repo)

    @property
    def repo(self) -> pygit2.Repository:
        return self._repo

    def create(self, data: EngramData) -> str:
        """Store an engram as Git objects and create a ref. Returns the engram ID."""
        # Serialize components
        manifest_bytes = json.dumps(data.manifest.to_dict(), indent=2).encode("utf-8")
        intent_bytes = data.intent.to_markdown().encode("utf-8")
        transcript_bytes = data.transcript.to_jsonl()
        operations_bytes = json.dumps(data.operations.to_dict(), indent=2).encode("utf-8")
        lineage_bytes = json.dumps(data.lineage.to_dict(), indent=2).encode("utf-8")

        # Create blobs
        manifest_oid = self._repo.create_blob(manifest_bytes)
        intent_oid = self._repo.create_blob(intent_bytes)
        transcript_oid = self._repo.create_blob(transcript_bytes)
        operations_oid = self._repo.create_blob(operations_bytes)
        lineage_oid = self._repo.create_blob(lineage_bytes)

        # Build tree
        tb = self._repo.TreeBuilder()
        tb.insert("manifest.json", manifest_oid, pygit2.GIT_FILEMODE_BLOB)
        tb.insert("intent.md", intent_oid, pygit2.GIT_FILEMODE_BLOB)
        tb.insert("transcript.jsonl", transcript_oid, pygit2.GIT_FILEMODE_BLOB)
        tb.insert("operations.json", operations_oid, pygit2.GIT_FILEMODE_BLOB)
        tb.insert("lineage.json", lineage_oid, pygit2.GIT_FILEMODE_BLOB)
        tree_oid = tb.write()

        # Create commit (standalone, no parent)
        sig = pygit2.Signature("engram", "engram@local")
        summary = data.manifest.summary or "engram session"
        message = f"engram: {summary}"
        commit_oid = self._repo.create_commit(
            None,  # Don't update any ref
            sig,
            sig,
            message,
            tree_oid,
            [],  # No parents
        )

        # Create ref
        engram_id = data.manifest.id
        ref_name = _ref_name(engram_id)
        self._repo.references.create(ref_name, commit_oid, force=True)

        return engram_id

    def read(self, id_or_prefix: str) -> EngramData:
        """Read an engram by its ID (or prefix)."""
        ref_name, _oid = self._resolve(id_or_prefix)
        ref = self._repo.references.get(ref_name)
        commit = self._repo.get(ref.target)
        tree = commit.tree

        manifest_blob = self._repo.get(tree["manifest.json"].id)
        intent_blob = self._repo.get(tree["intent.md"].id)
        transcript_blob = self._repo.get(tree["transcript.jsonl"].id)
        operations_blob = self._repo.get(tree["operations.json"].id)
        lineage_blob = self._repo.get(tree["lineage.json"].id)

        from engram.model import (
            Intent,
            Lineage,
            Operations,
            Transcript,
        )

        manifest = Manifest.from_dict(json.loads(manifest_blob.data))
        intent = Intent.from_markdown(intent_blob.data.decode("utf-8"))
        transcript = Transcript.from_jsonl(transcript_blob.data)
        operations = Operations.from_dict(json.loads(operations_blob.data))
        lineage = Lineage.from_dict(json.loads(lineage_blob.data))

        return EngramData(
            manifest=manifest,
            intent=intent,
            transcript=transcript,
            operations=operations,
            lineage=lineage,
        )

    def read_manifest(self, id_or_prefix: str) -> Manifest:
        """Read only the manifest (fast path)."""
        ref_name, _oid = self._resolve(id_or_prefix)
        ref = self._repo.references.get(ref_name)
        commit = self._repo.get(ref.target)
        tree = commit.tree
        manifest_blob = self._repo.get(tree["manifest.json"].id)
        return Manifest.from_dict(json.loads(manifest_blob.data))

    def list(self) -> list[Manifest]:
        """List all engrams, most recent first."""
        manifests = []
        for ref_name in self._repo.references:
            if ref_name.startswith(ENGRAM_REF_PREFIX):
                try:
                    ref = self._repo.references.get(ref_name)
                    commit = self._repo.get(ref.target)
                    tree = commit.tree
                    blob = self._repo.get(tree["manifest.json"].id)
                    manifests.append(Manifest.from_dict(json.loads(blob.data)))
                except Exception:
                    continue
        manifests.sort(key=lambda m: m.created_at, reverse=True)
        return manifests

    def delete(self, id_or_prefix: str) -> None:
        """Delete an engram by removing its ref."""
        ref_name, _oid = self._resolve(id_or_prefix)
        self._repo.references.delete(ref_name)

    def _resolve(self, id_or_prefix: str) -> tuple[str, pygit2.Oid]:
        """Resolve an engram ID or prefix to (ref_name, oid)."""
        # Try exact match first
        exact_ref = _ref_name(id_or_prefix)
        try:
            ref = self._repo.references.get(exact_ref)
            if ref is not None:
                return exact_ref, ref.target
        except (KeyError, ValueError):
            pass

        # Try prefix match
        matches = []
        for ref_name in self._repo.references:
            if ref_name.startswith(ENGRAM_REF_PREFIX):
                # Extract ID from refs/engrams/ab/full-id
                parts = ref_name[len(ENGRAM_REF_PREFIX):].split("/", 1)
                if len(parts) == 2:
                    full_id = parts[1]
                    if full_id.startswith(id_or_prefix):
                        ref = self._repo.references.get(ref_name)
                        matches.append((ref_name, ref.target))

        if len(matches) == 0:
            raise KeyError(f"Engram not found: {id_or_prefix}")
        if len(matches) > 1:
            raise ValueError(f"Ambiguous engram prefix: {id_or_prefix} ({len(matches)} matches)")
        return matches[0]


def _ref_name(engram_id: str) -> str:
    """Build the full ref name: refs/engrams/<ab>/<full-id>."""
    prefix = engram_id[:2]
    return f"refs/engrams/{prefix}/{engram_id}"
