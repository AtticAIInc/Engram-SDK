# engram doctor

Diagnose your engram setup and surface recent background failures.

## Usage

```bash
engram doctor [--events N]
```

## Description

`engram doctor` is a read-only health check. It inspects the current repository
and reports, in one place:

- **Repository** -- whether engram is initialized (`engram.enabled`).
- **Configuration** -- auto-capture, push-on-push, and default agent settings.
- **Git hooks** -- which of `prepare-commit-msg`, `post-commit`, and `pre-push`
  are installed, plus whether the Claude Code `SessionEnd` hook is present in
  `.claude/settings.json`.
- **Storage** -- how many engrams are stored, the timestamp of the most recent
  one, and whether the search index is present.
- **Recent activity** -- the tail of the event log at `.git/engram.log`, with a
  count of recent errors and warnings and the time of the last successful
  capture.

### Why this exists

Engram's git hooks and auto-capture run detached from your terminal and **fail
silently on purpose** -- they must never break `git commit` or `git push`. The
downside is that a misconfigured or broken capture looks identical to a repo
that simply has nothing to capture.

To make those outcomes visible, background operations append timestamped events
to `.git/engram.log` (a best-effort, size-bounded file inside `.git/`, so it is
never committed). `engram doctor` reads that log and the rest of your setup so
you can tell at a glance whether capture is working.

`--events N` controls how many recent log entries are shown (default `15`).

## Examples

```bash
# Full health report
engram doctor

# Show the last 50 logged events
engram doctor --events 50

# Machine-readable output for scripts/CI
engram doctor --format json
```

Example output:

```
engram doctor

✓ Repository initialized for engram

Configuration:
  ✓ auto-capture (capture sessions on commit)
  ✓ push-on-push (push engram refs with code)

Git hooks:
  ✓ prepare-commit-msg
  ✓ post-commit
  ✓ pre-push
  ✓ Claude Code SessionEnd hook

Storage:
  ✓ 45 engram(s) stored
  ✓ latest engram: 2026-03-14 02:57 UTC
  ✓ search index present

Recent activity (.git/engram.log):
  ✓ last successful capture logged: 2026-03-14T02:57:59+00:00

  ✓ 2026-03-14T02:57:59+00:00 session-end: captured engram a1b2c3d4 (47832 tokens)

✓ engram looks healthy.
```

## JSON output

`--format json` emits a structured object suitable for CI checks:

```json
{
  "enabled": true,
  "auto_capture": true,
  "push_on_push": true,
  "default_agent": null,
  "git_hooks_installed": ["prepare-commit-msg", "post-commit", "pre-push"],
  "git_hooks_managed": ["prepare-commit-msg", "post-commit", "pre-push"],
  "claude_code_hook_installed": true,
  "engram_count": 45,
  "latest_engram": "2026-03-14T02:57:59.744+00:00",
  "search_index_present": true,
  "recent_warnings": 0,
  "recent_errors": 0,
  "last_logged_capture": "2026-03-14T02:57:59+00:00"
}
```

## See Also

- [init](init.md) -- Sets up hooks and configuration that `doctor` checks
- [config](config.md) -- Global engram configuration
- [reindex](reindex.md) -- Rebuild the search index if `doctor` reports it missing
