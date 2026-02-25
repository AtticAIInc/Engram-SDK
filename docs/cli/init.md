# engram init

Initialize engram in the current Git repository with smart defaults.

## Usage

```bash
engram init [--force] [--remote <name>] [--no-auto-capture] [--no-auto-push] [--no-claude-code]
```

## Description

Sets up engram with all automation enabled by default:

1. Installs git hooks (`prepare-commit-msg`, `post-commit`, `pre-push`) for commit trailers and auto-push
2. Configures Claude Code `SessionEnd` hook for auto-importing sessions on exit
3. Enables auto-capture (imports agent sessions when you commit)
4. Enables auto-push (syncs engram refs when you `git push`)
5. Configures refspecs on remotes for engram ref sync
6. Installs `git loge` alias for viewing reasoning on commits
7. Annotates any existing engram-linked commits with git notes

Existing hooks are preserved -- engram chains after them via `.pre-engram` backups. All hooks fail silently to never break your git workflow.

### Output

```
Engram initialized. Reasoning capture is ready.

  Auto-capture:     ON  (agent sessions imported on commit)
  Auto-push:        ON  (engram refs sync on git push)
  Claude Code hook: ON  (sessions auto-imported on exit)
  Git notes alias:  ON  (use `git loge` to view reasoning)
  Annotated 15 commit(s) with engram reasoning notes.

Next steps:
  engram log                         List captured engrams
  engram search "query"              Search reasoning history
  engram why src/file.rs             Why does this file exist?
```

## Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--force` | bool | false | Force re-initialization (resets to defaults) |
| `--remote <name>` | string | all remotes | Configure refspecs on a specific remote only |
| `--no-auto-capture` | bool | false | Disable auto-capture of agent sessions on commit |
| `--no-auto-push` | bool | false | Disable auto-push of engram refs on git push |
| `--no-claude-code` | bool | false | Skip installing Claude Code SessionEnd hook |

## Examples

```bash
# Initialize with all defaults (recommended)
engram init

# Force re-initialization
engram init --force

# Disable auto-push (manual sync only)
engram init --no-auto-push

# Minimal setup (no automation)
engram init --no-auto-capture --no-auto-push --no-claude-code

# Only configure a specific remote
engram init --remote upstream
```

## See Also

- [Quick Start](../getting-started/quick-start.md)
- [Git Hooks](../guides/git-hooks.md)
- [Remote Sync](../guides/remote-sync.md)
