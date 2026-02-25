# Git Hooks

Engram installs git hooks to automatically link commits to reasoning sessions, capture agent data, and sync refs.

## Smart Defaults

All hooks and automation are enabled by default when you run `engram init`. Opt out with flags:

| Feature | Default | Opt-out |
|---------|---------|---------|
| Auto-capture | ON | `--no-auto-capture` |
| Auto-push | ON | `--no-auto-push` |
| Claude Code hook | ON | `--no-claude-code` |

## Installed Hooks

### prepare-commit-msg

Injects trailers into commit messages during active sessions (recording or auto-capture):

```
Add OAuth2 authentication

Engram-Id: abc123def456...
Engram-Agent: claude-code
Engram-Model: claude-sonnet-4-5
Engram-Tokens: 47832
Engram-Cost: $0.23
```

When auto-capture is enabled and no active recording session exists, the hook auto-imports the most recent Claude Code session and injects trailers.

### post-commit

Records the new commit SHA in the active session, linking the commit to the engram. If the session was auto-captured, updates the engram with the commit and cleans up.

### pre-push

When auto-push is enabled, automatically pushes engram refs alongside code. Uses `ENGRAM_PUSHING=1` environment variable to prevent recursive invocation.

### Claude Code SessionEnd

Installed in `.claude/settings.json`. Fires when Claude Code exits a session:

1. Reads the session transcript path from stdin JSON
2. Runs LLM-powered summarization (if API key configured) for high-quality intent fields
3. Imports the session as an engram (with deduplication)
4. Auto-annotates recent commits with git notes containing reasoning metadata

## Hook Safety

- **Existing hooks are preserved** -- engram chains after them via `.pre-engram` backups
- **Hooks fail silently** -- a hook error never breaks your git workflow
- **File locking** -- `ActiveSession` uses `fs2` advisory locks to prevent concurrent commit conflicts

## Git Notes

Engram attaches rich reasoning metadata to commits as git notes under `refs/notes/engram`. Notes include intent, summary, dead ends, decisions, and files changed. They are:

- **Auto-attached** when the SessionEnd hook fires
- **Auto-attached** during `engram init` for existing linked commits
- **Manually attachable** via `engram annotate`
- **Viewable** via `git loge` (alias) or `git log --notes=engram`

Notes sync alongside engram refs via `refs/notes/engram` refspecs.

## Re-installing Hooks

```bash
engram init --force
```

## See Also

- [init](../cli/init.md) -- CLI reference
- [annotate](../cli/annotate.md) -- Manual git notes annotation
- [Remote Sync](remote-sync.md) -- Push/pull/fetch
