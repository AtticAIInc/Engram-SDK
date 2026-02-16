# engram init

Initialize engram in the current Git repository.

## Usage

```bash
engram init [--force] [--remote <name>]
```

## Description

Sets up engram in a Git repository by:

1. Installing git hooks (`prepare-commit-msg`, `post-commit`, `pre-push`) that automatically inject `Engram-Id:` and `Engram-Agent:` trailers into commit messages during active sessions
2. Configuring refspecs on remotes so `git push`/`fetch` include engram refs
3. Creating the search index directory at `.git/engram-index/`

Existing hooks are preserved -- engram chains after them via `.pre-engram` backups. Hooks fail silently to never break your git workflow.

## Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--force` | bool | false | Force re-initialization (overwrites existing hooks) |
| `--remote <name>` | string | all remotes | Configure refspecs on a specific remote only |

## Examples

```bash
# Initialize in current repo
engram init

# Force re-initialization
engram init --force

# Only configure a specific remote
engram init --remote upstream
```

## See Also

- [Quick Start](../getting-started/quick-start.md)
- [Git Hooks](../guides/git-hooks.md)
- [Remote Sync](../guides/remote-sync.md)
