# Git Hooks

Engram installs git hooks to automatically link commits to reasoning sessions.

## Installed Hooks

When you run `engram init`, three hooks are installed:

### prepare-commit-msg

Injects `Engram-Id:` and `Engram-Agent:` trailers into commit messages during active sessions (recording or auto-capture).

```
Add OAuth2 authentication

Engram-Id: abc123def456...
Engram-Agent: claude-code/claude-sonnet-4-5
```

### post-commit

Records the new commit SHA in the active session, linking the commit to the engram.

### pre-push

If auto-push is enabled (`engram.push_on_push = true`), automatically pushes engram refs alongside code. Uses `ENGRAM_PUSHING=1` environment variable to prevent recursive invocation.

## Hook Safety

- **Existing hooks are preserved** -- engram chains after them via `.pre-engram` backups
- **Hooks fail silently** -- a hook error never breaks your git workflow
- **File locking** -- `ActiveSession` uses `fs2` advisory locks to prevent concurrent commit conflicts

## Auto-Capture

When `engram.auto_capture = true` in your git config:

1. On each commit, the `prepare-commit-msg` hook auto-imports the most recent Claude Code session
2. Creates a temporary `ActiveSession` to inject trailers
3. The `post-commit` hook links the commit SHA and cleans up

This means you get engram data without running `engram record` or `engram import` explicitly.

### Enable Auto-Capture

```bash
git config engram.auto_capture true
```

## Auto-Push

When `engram.push_on_push = true`:

- Every `git push` also pushes engram refs to the remote
- Uses the git CLI (not libgit2) to inherit your credential helpers

### Enable Auto-Push

```bash
git config engram.push_on_push true
```

## Re-installing Hooks

```bash
engram init --force
```

## See Also

- [init](../cli/init.md) -- CLI reference
- [Remote Sync](remote-sync.md) -- Manual push/pull/fetch
