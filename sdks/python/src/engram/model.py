"""Data model types matching the Rust engram-core model."""

from __future__ import annotations

import json
import uuid
from dataclasses import dataclass, field
from datetime import datetime, timezone
from enum import Enum
from typing import Any


def _new_engram_id() -> str:
    """Generate a new engram ID (UUID v4 hex, no dashes)."""
    return uuid.uuid4().hex


def _now() -> datetime:
    return datetime.now(timezone.utc)


class CaptureMode(str, Enum):
    WRAPPER = "wrapper"
    IMPORT = "import"
    SDK = "sdk"


class FileChangeType(str, Enum):
    CREATED = "created"
    MODIFIED = "modified"
    DELETED = "deleted"
    # Renamed is handled separately since it carries data


@dataclass
class AgentInfo:
    name: str
    model: str | None = None
    version: str | None = None

    def to_dict(self) -> dict:
        d: dict[str, Any] = {"name": self.name}
        if self.model is not None:
            d["model"] = self.model
        if self.version is not None:
            d["version"] = self.version
        return d

    @classmethod
    def from_dict(cls, d: dict) -> AgentInfo:
        return cls(name=d["name"], model=d.get("model"), version=d.get("version"))


@dataclass
class TokenUsage:
    input_tokens: int = 0
    output_tokens: int = 0
    cache_read_tokens: int = 0
    cache_write_tokens: int = 0
    total_tokens: int = 0
    cost_usd: float | None = None

    def to_dict(self) -> dict:
        d: dict[str, Any] = {
            "input_tokens": self.input_tokens,
            "output_tokens": self.output_tokens,
            "cache_read_tokens": self.cache_read_tokens,
            "cache_write_tokens": self.cache_write_tokens,
            "total_tokens": self.total_tokens,
        }
        if self.cost_usd is not None:
            d["cost_usd"] = self.cost_usd
        return d

    @classmethod
    def from_dict(cls, d: dict) -> TokenUsage:
        return cls(
            input_tokens=d.get("input_tokens", 0),
            output_tokens=d.get("output_tokens", 0),
            cache_read_tokens=d.get("cache_read_tokens", 0),
            cache_write_tokens=d.get("cache_write_tokens", 0),
            total_tokens=d.get("total_tokens", 0),
            cost_usd=d.get("cost_usd"),
        )


@dataclass
class DeadEnd:
    approach: str
    reason: str

    def to_dict(self) -> dict:
        return {"approach": self.approach, "reason": self.reason}

    @classmethod
    def from_dict(cls, d: dict) -> DeadEnd:
        return cls(approach=d["approach"], reason=d["reason"])


@dataclass
class Decision:
    description: str
    rationale: str

    def to_dict(self) -> dict:
        return {"description": self.description, "rationale": self.rationale}

    @classmethod
    def from_dict(cls, d: dict) -> Decision:
        return cls(description=d["description"], rationale=d["rationale"])


@dataclass
class Intent:
    original_request: str
    interpreted_goal: str | None = None
    summary: str | None = None
    dead_ends: list[DeadEnd] = field(default_factory=list)
    decisions: list[Decision] = field(default_factory=list)

    def to_markdown(self) -> str:
        lines = ["# Intent", "", self.original_request, ""]
        if self.summary:
            lines.extend(["## Summary", "", self.summary, ""])
        if self.dead_ends:
            lines.extend(["## Dead Ends", ""])
            for de in self.dead_ends:
                lines.append(f"- **{de.approach}**: {de.reason}")
            lines.append("")
        if self.decisions:
            lines.extend(["## Decisions", ""])
            for d in self.decisions:
                lines.append(f"- **{d.description}**: {d.rationale}")
            lines.append("")
        return "\n".join(lines)

    @classmethod
    def from_markdown(cls, md: str) -> Intent:
        """Parse intent from Markdown format (matching Rust parser)."""
        original_request = ""
        interpreted_goal: str | None = None
        summary: str | None = None
        dead_ends: list[DeadEnd] = []
        decisions: list[Decision] = []

        current_section = "intent"
        current_content = ""

        def save_section() -> None:
            nonlocal original_request, interpreted_goal, summary
            trimmed = current_content.strip()
            if not trimmed:
                return
            if current_section == "intent":
                original_request = trimmed
            elif current_section == "goal":
                interpreted_goal = trimmed
            elif current_section == "summary":
                summary = trimmed

        for line in md.splitlines():
            if line.startswith("# Intent"):
                current_section = "intent"
                current_content = ""
                continue
            elif line.startswith("## Original Request"):
                # backward compat: treat as intent section
                save_section()
                current_section = "intent"
                current_content = ""
                continue
            elif line.startswith("## Interpreted Goal"):
                save_section()
                current_section = "goal"
                current_content = ""
                continue
            elif line.startswith("## Summary"):
                save_section()
                current_section = "summary"
                current_content = ""
                continue
            elif line.startswith("## Dead Ends"):
                save_section()
                current_section = "dead_ends"
                current_content = ""
                continue
            elif line.startswith("## Decisions"):
                save_section()
                current_section = "decisions"
                current_content = ""
                continue

            if current_section == "dead_ends":
                if line.startswith("- **") and "**: " in line:
                    rest = line[4:]  # strip "- **"
                    approach, reason = rest.split("**: ", 1)
                    dead_ends.append(DeadEnd(approach=approach, reason=reason))
            elif current_section == "decisions":
                if line.startswith("- **") and "**: " in line:
                    rest = line[4:]
                    desc, rationale = rest.split("**: ", 1)
                    decisions.append(Decision(description=desc, rationale=rationale))
            else:
                if current_content or line:
                    if current_content:
                        current_content += "\n"
                    current_content += line

        save_section()

        return cls(
            original_request=original_request,
            interpreted_goal=interpreted_goal,
            summary=summary,
            dead_ends=dead_ends,
            decisions=decisions,
        )


@dataclass
class TranscriptEntry:
    timestamp: datetime
    role: str  # "user", "assistant", "system", "tool"
    content: dict  # {"type": "text", "text": "..."} or {"type": "tool_use", ...}
    token_count: int | None = None

    def to_dict(self) -> dict:
        d: dict[str, Any] = {
            "timestamp": self.timestamp.isoformat(),
            "role": self.role,
            "content": self.content,
        }
        if self.token_count is not None:
            d["token_count"] = self.token_count
        return d

    @classmethod
    def from_dict(cls, d: dict) -> TranscriptEntry:
        return cls(
            timestamp=datetime.fromisoformat(d["timestamp"]),
            role=d["role"],
            content=d["content"],
            token_count=d.get("token_count"),
        )


@dataclass
class Transcript:
    entries: list[TranscriptEntry] = field(default_factory=list)

    def to_jsonl(self) -> bytes:
        lines = [json.dumps(e.to_dict()) for e in self.entries]
        return ("\n".join(lines) + "\n").encode("utf-8") if lines else b""

    @classmethod
    def from_jsonl(cls, data: bytes) -> Transcript:
        entries = []
        for line in data.decode("utf-8").strip().splitlines():
            if line.strip():
                entries.append(TranscriptEntry.from_dict(json.loads(line)))
        return cls(entries=entries)


@dataclass
class ToolCall:
    timestamp: datetime
    tool_name: str
    input: Any
    output_summary: str | None = None
    duration_ms: int | None = None
    is_error: bool = False

    def to_dict(self) -> dict:
        d: dict[str, Any] = {
            "timestamp": self.timestamp.isoformat(),
            "tool_name": self.tool_name,
            "input": self.input,
            "is_error": self.is_error,
        }
        if self.output_summary is not None:
            d["output_summary"] = self.output_summary
        if self.duration_ms is not None:
            d["duration_ms"] = self.duration_ms
        return d


@dataclass
class FileChange:
    path: str
    change_type: str  # "created", "modified", "deleted", or "renamed"
    rename_from: str | None = None  # populated when change_type is "renamed"
    lines_added: int | None = None
    lines_removed: int | None = None

    def to_dict(self) -> dict:
        d: dict[str, Any] = {"path": self.path}
        if self.change_type == "renamed" and self.rename_from:
            d["change_type"] = {"renamed": {"from": self.rename_from}}
        else:
            d["change_type"] = self.change_type
        if self.lines_added is not None:
            d["lines_added"] = self.lines_added
        if self.lines_removed is not None:
            d["lines_removed"] = self.lines_removed
        return d

    @classmethod
    def from_dict(cls, d: dict) -> FileChange:
        ct = d.get("change_type", "modified")
        rename_from = None
        if isinstance(ct, dict) and "renamed" in ct:
            rename_from = ct["renamed"].get("from")
            ct = "renamed"
        return cls(
            path=d["path"],
            change_type=ct,
            rename_from=rename_from,
            lines_added=d.get("lines_added"),
            lines_removed=d.get("lines_removed"),
        )


@dataclass
class ShellCommand:
    timestamp: datetime
    command: str
    exit_code: int | None = None
    duration_ms: int | None = None

    def to_dict(self) -> dict:
        d: dict[str, Any] = {
            "timestamp": self.timestamp.isoformat(),
            "command": self.command,
        }
        if self.exit_code is not None:
            d["exit_code"] = self.exit_code
        if self.duration_ms is not None:
            d["duration_ms"] = self.duration_ms
        return d


@dataclass
class Operations:
    tool_calls: list[ToolCall] = field(default_factory=list)
    file_changes: list[FileChange] = field(default_factory=list)
    shell_commands: list[ShellCommand] = field(default_factory=list)

    def to_dict(self) -> dict:
        return {
            "tool_calls": [tc.to_dict() for tc in self.tool_calls],
            "file_changes": [fc.to_dict() for fc in self.file_changes],
            "shell_commands": [sc.to_dict() for sc in self.shell_commands],
        }

    @classmethod
    def from_dict(cls, d: dict) -> Operations:
        return cls(
            tool_calls=[],  # ToolCall parsing requires timestamp handling; keep simple
            file_changes=[FileChange.from_dict(fc) for fc in d.get("file_changes", [])],
            shell_commands=[],
        )


@dataclass
class Lineage:
    parent_engram: str | None = None
    child_engrams: list[str] = field(default_factory=list)
    related_engrams: list[dict] = field(default_factory=list)
    git_commits: list[str] = field(default_factory=list)
    branch: str | None = None

    def to_dict(self) -> dict:
        d: dict[str, Any] = {
            "child_engrams": self.child_engrams,
            "related_engrams": self.related_engrams,
            "git_commits": self.git_commits,
        }
        if self.parent_engram is not None:
            d["parent_engram"] = self.parent_engram
        if self.branch is not None:
            d["branch"] = self.branch
        return d

    @classmethod
    def from_dict(cls, d: dict) -> Lineage:
        return cls(
            parent_engram=d.get("parent_engram"),
            child_engrams=d.get("child_engrams", []),
            related_engrams=d.get("related_engrams", []),
            git_commits=d.get("git_commits", []),
            branch=d.get("branch"),
        )


def _parse_capture_mode(value: str) -> CaptureMode:
    """Parse CaptureMode accepting both snake_case and legacy PascalCase."""
    _compat_map = {
        "Wrapper": "wrapper", "Import": "import", "Sdk": "sdk",
    }
    normalized = _compat_map.get(value, value)
    return CaptureMode(normalized)


@dataclass
class Manifest:
    id: str
    version: int
    created_at: datetime
    agent: AgentInfo
    token_usage: TokenUsage
    capture_mode: CaptureMode
    finished_at: datetime | None = None
    git_commits: list[str] = field(default_factory=list)
    summary: str | None = None
    tags: list[str] = field(default_factory=list)

    def to_dict(self) -> dict:
        d: dict[str, Any] = {
            "id": self.id,
            "version": self.version,
            "created_at": self.created_at.isoformat(),
            "agent": self.agent.to_dict(),
            "token_usage": self.token_usage.to_dict(),
            "capture_mode": self.capture_mode.value,
            "git_commits": self.git_commits,
            "tags": self.tags,
        }
        if self.finished_at is not None:
            d["finished_at"] = self.finished_at.isoformat()
        if self.summary is not None:
            d["summary"] = self.summary
        return d

    @classmethod
    def from_dict(cls, d: dict) -> Manifest:
        return cls(
            id=d["id"],
            version=d["version"],
            created_at=datetime.fromisoformat(d["created_at"]),
            finished_at=datetime.fromisoformat(d["finished_at"]) if d.get("finished_at") else None,
            agent=AgentInfo.from_dict(d["agent"]),
            token_usage=TokenUsage.from_dict(d["token_usage"]),
            capture_mode=_parse_capture_mode(d["capture_mode"]),
            git_commits=d.get("git_commits", []),
            summary=d.get("summary"),
            tags=d.get("tags", []),
        )


@dataclass
class EngramData:
    manifest: Manifest
    intent: Intent
    transcript: Transcript
    operations: Operations
    lineage: Lineage
