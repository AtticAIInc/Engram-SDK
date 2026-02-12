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
    TokenUsage,
    Transcript,
    TranscriptEntry,
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


def test_transcript_jsonl_roundtrip():
    now = datetime.now(timezone.utc)
    entries = [
        TranscriptEntry(timestamp=now, role="user", content={"type": "text", "text": "Hello"}),
        TranscriptEntry(
            timestamp=now, role="assistant", content={"type": "text", "text": "Hi there"}
        ),
    ]
    transcript = Transcript(entries=entries)
    jsonl = transcript.to_jsonl()
    restored = Transcript.from_jsonl(jsonl)
    assert len(restored.entries) == 2
    assert restored.entries[0].role == "user"
    assert restored.entries[1].content["text"] == "Hi there"
