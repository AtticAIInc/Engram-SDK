"""Fluent session builder for creating engrams programmatically."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from types import TracebackType

from engram.model import (
    AgentInfo,
    CaptureMode,
    DeadEnd,
    Decision,
    EngramData,
    FileChange,
    Intent,
    Lineage,
    Manifest,
    Operations,
    ShellCommand,
    TokenUsage,
    ToolCall,
    Transcript,
    TranscriptEntry,
    _new_engram_id,
    _now,
)
from engram.storage import GitStorage


class EngramSession:
    """A fluent session builder for creating engrams programmatically.

    Can be used as a context manager:

        async with EngramSession("my-agent", "claude-sonnet-4-5") as session:
            session.log_message("user", "Add auth")
            session.log_message("assistant", "Implementing...")

    Or manually:

        session = EngramSession.begin("my-agent", "claude-sonnet-4-5")
        session.log_message("user", "Add auth")
        result = session.commit("abc123", "Added auth")
    """

    def __init__(self, agent_name: str, model: str | None = None) -> None:
        self._agent = AgentInfo(name=agent_name, model=model)
        self._transcript: list[TranscriptEntry] = []
        self._tool_calls: list[ToolCall] = []
        self._file_changes: list[FileChange] = []
        self._shell_commands: list[ShellCommand] = []
        self._dead_ends: list[DeadEnd] = []
        self._decisions: list[Decision] = []
        self._token_usage = TokenUsage()
        self._original_request: str | None = None
        self._summary: str | None = None
        self._tags: list[str] = []
        self._parent: str | None = None
        self._started_at = _now()
        self._storage: GitStorage | None = None

    @classmethod
    def begin(cls, agent_name: str, model: str | None = None) -> EngramSession:
        """Create a new session for a given agent and optional model."""
        return cls(agent_name, model)

    async def __aenter__(self) -> EngramSession:
        return self

    async def __aexit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> None:
        if exc_type is None:
            self.commit()

    def __enter__(self) -> EngramSession:
        return self

    def __exit__(
        self,
        exc_type: type[BaseException] | None,
        exc_val: BaseException | None,
        exc_tb: TracebackType | None,
    ) -> None:
        if exc_type is None:
            self.commit()

    def log_message(self, role: str, content: str) -> EngramSession:
        """Log a message (user, assistant, system, or tool)."""
        if role == "user" and self._original_request is None:
            self._original_request = content

        self._transcript.append(
            TranscriptEntry(
                timestamp=_now(),
                role=role,
                content={"type": "text", "text": content},
            )
        )
        return self

    def log_tool_call(
        self,
        tool_name: str,
        input_data: str | dict,
        output_summary: str | None = None,
    ) -> EngramSession:
        """Log a tool call."""
        if isinstance(input_data, str):
            try:
                parsed = json.loads(input_data)
            except json.JSONDecodeError:
                parsed = input_data
        else:
            parsed = input_data

        self._tool_calls.append(
            ToolCall(
                timestamp=_now(),
                tool_name=tool_name,
                input=parsed,
                output_summary=output_summary,
            )
        )
        return self

    def log_file_change(self, path: str, change_type: str) -> EngramSession:
        """Log a file change (created, modified, deleted)."""
        ct_map = {
            "created": "created",
            "create": "created",
            "new": "created",
            "modified": "modified",
            "modify": "modified",
            "changed": "modified",
            "deleted": "deleted",
            "delete": "deleted",
            "removed": "deleted",
        }
        ct = ct_map.get(change_type.lower(), "modified")
        self._file_changes.append(FileChange(path=path, change_type=ct))
        return self

    def log_shell_command(
        self,
        command: str,
        exit_code: int | None = None,
        duration_ms: int | None = None,
    ) -> EngramSession:
        """Log a shell command execution."""
        self._shell_commands.append(
            ShellCommand(
                timestamp=_now(),
                command=command,
                exit_code=exit_code,
                duration_ms=duration_ms,
            )
        )
        return self

    def log_rejection(self, approach: str, reason: str) -> EngramSession:
        """Log a rejected approach (dead end)."""
        self._dead_ends.append(DeadEnd(approach=approach, reason=reason))
        return self

    def log_decision(self, description: str, rationale: str) -> EngramSession:
        """Log a decision made during the session."""
        self._decisions.append(Decision(description=description, rationale=rationale))
        return self

    def add_tokens(
        self,
        input_tokens: int,
        output_tokens: int,
        cost_usd: float | None = None,
    ) -> EngramSession:
        """Add token usage. Accumulates across multiple calls."""
        self._token_usage.input_tokens += input_tokens
        self._token_usage.output_tokens += output_tokens
        self._token_usage.total_tokens += input_tokens + output_tokens
        if cost_usd is not None:
            if self._token_usage.cost_usd is None:
                self._token_usage.cost_usd = 0.0
            self._token_usage.cost_usd += cost_usd
        return self

    def set_summary(self, summary: str) -> EngramSession:
        """Set a summary for this session."""
        self._summary = summary
        return self

    def tag(self, tag: str) -> EngramSession:
        """Add a tag."""
        self._tags.append(tag)
        return self

    def parent(self, parent_id: str) -> EngramSession:
        """Set the parent engram ID."""
        self._parent = parent_id
        return self

    def build(
        self,
        git_sha: str | None = None,
        summary: str | None = None,
    ) -> EngramData:
        """Build the EngramData without storing it."""
        engram_id = _new_engram_id()
        finished_at = _now()
        final_summary = summary or self._summary or self._original_request

        git_commits = [git_sha] if git_sha else []

        manifest = Manifest(
            id=engram_id,
            version=1,
            created_at=self._started_at,
            finished_at=finished_at,
            agent=self._agent,
            git_commits=git_commits,
            token_usage=self._token_usage,
            summary=final_summary,
            tags=self._tags,
            capture_mode=CaptureMode.SDK,
        )

        intent = Intent(
            original_request=self._original_request or "SDK session",
            summary=final_summary,
            dead_ends=self._dead_ends,
            decisions=self._decisions,
        )

        transcript = Transcript(entries=self._transcript)

        operations = Operations(
            tool_calls=self._tool_calls,
            file_changes=self._file_changes,
            shell_commands=self._shell_commands,
        )

        lineage = Lineage(
            parent_engram=self._parent,
            git_commits=git_commits,
        )

        return EngramData(
            manifest=manifest,
            intent=intent,
            transcript=transcript,
            operations=operations,
            lineage=lineage,
        )

    def commit(
        self,
        git_sha: str | None = None,
        summary: str | None = None,
        storage: GitStorage | None = None,
    ) -> str:
        """Finalize and store the engram in Git. Returns the engram ID."""
        data = self.build(git_sha, summary)
        store = storage or self._storage or GitStorage.discover()
        return store.create(data)
