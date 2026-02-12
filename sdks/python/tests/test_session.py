"""Tests for engram session builder and storage lifecycle."""

import pygit2
import pytest

from engram.model import CaptureMode
from engram.session import EngramSession
from engram.storage import GitStorage


def test_session_build():
    """Test building an EngramData without storing."""
    session = EngramSession.begin("test-agent", "gpt-4")
    session.log_message("user", "Add auth to the API")
    session.log_message("assistant", "I'll add JWT auth.")
    session.log_tool_call("write_file", '{"path": "src/auth.rs"}', "Created auth module")
    session.log_file_change("src/auth.rs", "created")
    session.log_rejection("Session auth", "Too stateful")
    session.log_decision("Use JWT", "Stateless, works with load balancers")
    session.add_tokens(1500, 800, 0.02)
    session.tag("auth")

    data = session.build("abc123", "Add JWT authentication")

    assert data.manifest.agent.name == "test-agent"
    assert data.manifest.agent.model == "gpt-4"
    assert data.manifest.capture_mode == CaptureMode.SDK
    assert data.manifest.summary == "Add JWT authentication"
    assert data.manifest.token_usage.input_tokens == 1500
    assert data.manifest.token_usage.output_tokens == 800
    assert data.manifest.token_usage.total_tokens == 2300
    assert data.manifest.token_usage.cost_usd == pytest.approx(0.02)
    assert data.manifest.tags == ["auth"]

    assert data.intent.original_request == "Add auth to the API"
    assert len(data.intent.dead_ends) == 1
    assert len(data.intent.decisions) == 1

    assert len(data.transcript.entries) == 2
    assert len(data.operations.tool_calls) == 1
    assert len(data.operations.file_changes) == 1
    assert data.operations.file_changes[0].change_type == "created"

    assert data.lineage.git_commits == ["abc123"]


def test_session_store(tmp_git_repo):
    """Test full create → read lifecycle in a temp git repo."""
    storage = GitStorage(tmp_git_repo)

    session = EngramSession.begin("test-agent", "claude-sonnet")
    session.log_message("user", "Fix the login bug")
    session.log_message("assistant", "Found the issue in auth.rs")
    session.add_tokens(500, 200, 0.005)

    engram_id = session.commit(summary="Fixed login bug", storage=storage)

    # Read back
    data = storage.read(engram_id)
    assert data.manifest.agent.name == "test-agent"
    assert data.manifest.summary == "Fixed login bug"

    # List
    manifests = storage.list()
    assert len(manifests) == 1
    assert manifests[0].id == engram_id

    # Delete
    storage.delete(engram_id)
    assert len(storage.list()) == 0


def test_accumulate_tokens():
    """Test that tokens accumulate across multiple add_tokens calls."""
    session = EngramSession.begin("test", None)
    session.add_tokens(100, 50, 0.01)
    session.add_tokens(200, 100, 0.02)

    data = session.build()
    assert data.manifest.token_usage.input_tokens == 300
    assert data.manifest.token_usage.output_tokens == 150
    assert data.manifest.token_usage.total_tokens == 450
    assert data.manifest.token_usage.cost_usd == pytest.approx(0.03)


def test_context_manager(tmp_git_repo):
    """Test using EngramSession as a sync context manager."""
    storage = GitStorage(tmp_git_repo)

    with EngramSession.begin("ctx-agent", "gpt-4") as session:
        session._storage = storage
        session.log_message("user", "Hello")
        session.log_message("assistant", "Hi")

    manifests = storage.list()
    assert len(manifests) == 1
    assert manifests[0].agent.name == "ctx-agent"
