# Remote Sync

Sync engram reasoning data alongside code using standard Git remotes.

## How It Works

Engram refs live under `refs/engrams/*`. During `engram init`, refspecs are added to each remote:

```
[remote "origin"]
    fetch = +refs/engrams/*:refs/engrams/*
    push = +refs/engrams/*:refs/engrams/*
```

This means engram data syncs with regular `git push`/`fetch`, and the `engram push`/`pull`/`fetch` commands provide convenience wrappers.

## Commands

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

## Automatic Sync

Enable auto-push to sync engrams on every `git push`:

```bash
git config engram.push_on_push true
```

See [Git Hooks](git-hooks.md) for details.

## Team Workflow

1. Developer records or imports sessions locally
2. `engram push` (or auto-push) sends engram refs to the remote
3. Teammates run `engram pull` to get the reasoning data
4. `engram search`, `engram trace`, and `engram review` work across all team members' sessions

## See Also

- [push](../cli/push.md) -- CLI reference
- [pull](../cli/pull.md) -- CLI reference
- [fetch](../cli/fetch.md) -- CLI reference
- [Git Hooks](git-hooks.md) -- Auto-push configuration
