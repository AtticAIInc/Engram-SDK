"""Test fixtures for engram Python SDK."""

import subprocess
from pathlib import Path

import pytest


@pytest.fixture
def tmp_git_repo(tmp_path: Path):
    """Create a temporary Git repository."""
    repo_path = str(tmp_path)
    subprocess.run(["git", "init", repo_path], check=True, capture_output=True)
    subprocess.run(
        ["git", "config", "user.name", "Test User"],
        cwd=repo_path, check=True, capture_output=True,
    )
    subprocess.run(
        ["git", "config", "user.email", "test@example.com"],
        cwd=repo_path, check=True, capture_output=True,
    )
    return repo_path
