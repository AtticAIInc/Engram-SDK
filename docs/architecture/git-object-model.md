# Git Object Model

Engrams are stored as **native Git objects** -- blobs, trees, commits, and refs. No files on disk, no sidecar database. They travel with `clone`, `push`, and `pull`.

## Object Hierarchy

```
ref (refs/engrams/ab/abc123def456...)
  └── commit (orphan, no parent)
       └── tree
            ├── intent.md           (blob)
            ├── lineage.json        (blob)
            ├── manifest.json       (blob)
            ├── operations.json     (blob)
            └── transcript.jsonl    (blob)
```

### Refs

Engram refs live under `refs/engrams/` with a two-character fanout:

```
refs/engrams/<first-2-chars>/<full-id>
```

Example: engram `abc123def456...` is stored at `refs/engrams/ab/abc123def456...`

The fanout prevents having thousands of refs in a single directory, which would degrade Git performance.

### Commits

Each engram is stored as an **orphan commit** -- a commit with no parent that exists outside any branch. This keeps engram data completely separate from your code history.

The commit message is: `engram: <summary>`

### Trees

The commit points to a tree with exactly five entries, sorted alphabetically (required by `git mktree`):

1. `intent.md` -- Human-readable reasoning summary
2. `lineage.json` -- Relationships to other entities
3. `manifest.json` -- Compact metadata
4. `operations.json` -- Tool calls and file changes
5. `transcript.jsonl` -- Full session transcript

### Blobs

Each tree entry points to a blob containing the serialized component data.

## HEAD Pointer

Engram maintains a `.git/engram-head` file containing the ID of the most recently created engram. This enables O(1) resolution of `engram show HEAD` without scanning all refs.

## Ref Syncing

When you run `engram init`, refspecs are added to each remote:

```
[remote "origin"]
    fetch = +refs/engrams/*:refs/engrams/*
    push = +refs/engrams/*:refs/engrams/*
```

This means `git push` and `git fetch` automatically include engram refs. The `engram push`/`pull`/`fetch` commands provide a convenient interface for this.

## Commit Trailers

During active recording sessions, git hooks inject trailers into commit messages:

```
Add OAuth2 authentication

Engram-Id: abc123def456...
Engram-Agent: claude-code/claude-sonnet-4-5
```

The `engram review` command uses these trailers to link commits to engrams.

## Import Deduplication

Each imported session gets a `source_hash` (SHA-256 of the source file content) stored in the manifest. Before importing, engram checks if a manifest with the same `source_hash` already exists, preventing duplicate imports.

## Why Git Objects?

1. **No extra infrastructure** -- Works in any Git repo, no database to manage
2. **Automatic sync** -- Travels with push/pull/clone/fork
3. **Immutable history** -- Git's content-addressable storage provides integrity
4. **Efficient storage** -- Git's packfile compression works on engram objects too
5. **No vendor lock-in** -- Standard Git, readable by any Git tool
