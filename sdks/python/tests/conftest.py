"""Test fixtures for engram Python SDK."""

import os
import tempfile
from pathlib import Path

import pygit2
import pytest


@pytest.fixture
def tmp_git_repo(tmp_path: Path):
    """Create a temporary Git repository with an initial commit."""
    repo = pygit2.init_repository(str(tmp_path))

    # Create initial commit
    sig = pygit2.Signature("Test User", "test@example.com")
    tree = repo.TreeBuilder().write()
    repo.create_commit("HEAD", sig, sig, "Initial commit", tree, [])

    return repo
