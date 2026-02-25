# Remote Sync

Sync engram reasoning data alongside code using standard Git remotes.

## How It Works

During `engram init`, refspecs are added to each remote for both engram refs and git notes:

```
[remote "origin"]
    fetch = +refs/engrams/*:refs/engrams/*
    push = +refs/engrams/*:refs/engrams/*
    fetch = +refs/notes/engram:refs/notes/engram
    push = refs/notes/engram:refs/notes/engram
```

## Automatic Sync (Default)

Auto-push is enabled by default. Every `git push` also pushes engram refs and notes to the remote automatically. No extra commands needed.

To disable: `engram init --no-auto-push` or `git config engram.pushOnPush false`.

## Manual Commands

### Push

Push engram refs to a remote:

```bash
engram push              # Push to origin
engram push upstream     # Push to specific remote
engram push --dry-run    # Preview what would be pushed
```

### Pull

Fetch engram refs and rebuild the search index:

```bash
engram pull              # Pull from origin
engram pull upstream     # Pull from specific remote
```

### Fetch

Fetch engram refs without rebuilding the search index:

```bash
engram fetch             # Fetch from origin
engram fetch --dry-run   # Preview what would be fetched
```

Use `engram reindex` separately to update the search index after fetching.

## Team Workflow

1. Developer uses Claude Code normally (sessions auto-captured)
2. `git push` automatically syncs engram refs and notes to the remote
3. Teammates run `engram pull` to get the reasoning data
4. `engram search`, `engram trace`, `engram why`, and `engram review` work across all team members' sessions

## See Also

- [push](../cli/push.md) -- CLI reference
- [pull](../cli/pull.md) -- CLI reference
- [fetch](../cli/fetch.md) -- CLI reference
- [Git Hooks](git-hooks.md) -- Auto-push configuration
