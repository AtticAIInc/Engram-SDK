"""Tests for engram data model serialization."""

import json
from datetime import datetime, timezone

from engram.model import (
    AgentInfo,
    CaptureMode,
    DeadEnd,
    Decision,
    Intent,
    Manifest,
    TextContent,
    ThinkingContent,
    TokenUsage,
    ToolResultContent,
    ToolUseContent,
    Transcript,
    TranscriptEntry,
    parse_transcript_content,
)


def test_token_usage_roundtrip():
    usage = TokenUsage(
        input_tokens=1000,
        output_tokens=500,
        total_tokens=1500,
        cost_usd=0.02,
    )
    d = usage.to_dict()
    restored = TokenUsage.from_dict(d)
    assert restored.input_tokens == 1000
    assert restored.output_tokens == 500
    assert restored.total_tokens == 1500
    assert restored.cost_usd == 0.02


def test_token_usage_no_cost():
    usage = TokenUsage(input_tokens=100, output_tokens=50, total_tokens=150)
    d = usage.to_dict()
    assert "cost_usd" not in d
    restored = TokenUsage.from_dict(d)
    assert restored.cost_usd is None


def test_agent_info_roundtrip():
    agent = AgentInfo(name="claude-code", model="claude-sonnet-4-5", version="1.0")
    d = agent.to_dict()
    restored = AgentInfo.from_dict(d)
    assert restored.name == "claude-code"
    assert restored.model == "claude-sonnet-4-5"
    assert restored.version == "1.0"


def test_manifest_roundtrip():
    now = datetime.now(timezone.utc)
    manifest = Manifest(
        id="abcdef1234567890abcdef1234567890",
        version=1,
        created_at=now,
        agent=AgentInfo(name="test-agent", model="gpt-4"),
        token_usage=TokenUsage(input_tokens=100, output_tokens=50, total_tokens=150),
        capture_mode=CaptureMode.SDK,
        summary="Test engram",
        tags=["auth"],
    )
    d = manifest.to_dict()
    json_str = json.dumps(d)
    restored_d = json.loads(json_str)
    restored = Manifest.from_dict(restored_d)
    assert restored.id == "abcdef1234567890abcdef1234567890"
    assert restored.agent.name == "test-agent"
    assert restored.capture_mode == CaptureMode.SDK
    assert restored.summary == "Test engram"


def test_manifest_source_hash():
    now = datetime.now(timezone.utc)
    manifest = Manifest(
        id="abcdef1234567890abcdef1234567890",
        version=1,
        created_at=now,
        agent=AgentInfo(name="test-agent"),
        token_usage=TokenUsage(),
        capture_mode=CaptureMode.IMPORT,
        source_hash="abc123def456",
    )
    d = manifest.to_dict()
    assert d["source_hash"] == "abc123def456"
    restored = Manifest.from_dict(d)
    assert restored.source_hash == "abc123def456"


def test_manifest_no_source_hash():
    now = datetime.now(timezone.utc)
    manifest = Manifest(
        id="abcdef1234567890abcdef1234567890",
        version=1,
        created_at=now,
        agent=AgentInfo(name="test-agent"),
        token_usage=TokenUsage(),
        capture_mode=CaptureMode.SDK,
    )
    d = manifest.to_dict()
    assert "source_hash" not in d
    restored = Manifest.from_dict(d)
    assert restored.source_hash is None


def test_intent_to_markdown():
    intent = Intent(
        original_request="Add authentication",
        summary="Added JWT auth",
        dead_ends=[DeadEnd(approach="passport.js", reason="Middleware conflict")],
        decisions=[Decision(description="Use JWT", rationale="Stateless")],
    )
    md = intent.to_markdown()
    assert "# Intent" in md
    assert "Add authentication" in md
    assert "passport.js" in md
    assert "Use JWT" in md


def test_transcript_content_types():
    text = TextContent(text="Hello")
    assert text.to_dict() == {"type": "text", "text": "Hello"}
    assert TextContent.from_dict({"text": "Hello"}).text == "Hello"

    tool_use = ToolUseContent(tool_name="Bash", tool_id="id1", input={"command": "ls"})
    d = tool_use.to_dict()
    assert d["type"] == "tool_use"
    assert d["tool_name"] == "Bash"
    restored = ToolUseContent.from_dict(d)
    assert restored.tool_id == "id1"

    tool_result = ToolResultContent(tool_id="id1", output="done", is_error=False)
    d = tool_result.to_dict()
    assert d["type"] == "tool_result"
    assert not d["is_error"]

    thinking = ThinkingContent(text="Let me think...")
    d = thinking.to_dict()
    assert d["type"] == "thinking"
    assert ThinkingContent.from_dict(d).text == "Let me think..."


def test_parse_transcript_content():
    parsed = parse_transcript_content({"type": "text", "text": "hi"})
    assert isinstance(parsed, TextContent)
    assert parsed.text == "hi"

    parsed = parse_transcript_content({"type": "tool_use", "tool_name": "X", "tool_id": "1", "input": {}})
    assert isinstance(parsed, ToolUseContent)

    # Unknown types return raw dict
    parsed = parse_transcript_content({"type": "unknown", "data": 42})
    assert isinstance(parsed, dict)


def test_transcript_jsonl_roundtrip():
    now = datetime.now(timezone.utc)
    entries = [
        TranscriptEntry(timestamp=now, role="user", content=TextContent(text="Hello")),
        TranscriptEntry(
            timestamp=now, role="assistant", content=TextContent(text="Hi there")
        ),
    ]
    transcript = Transcript(entries=entries)
    jsonl = transcript.to_jsonl()
    restored = Transcript.from_jsonl(jsonl)
    assert len(restored.entries) == 2
    assert restored.entries[0].role == "user"
    assert isinstance(restored.entries[1].content, TextContent)
    assert restored.entries[1].content.text == "Hi there"
