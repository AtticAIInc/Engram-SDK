# engram annotate

Attach engram reasoning as git notes to commits.

## Usage

```bash
engram annotate [range] [--dry-run] [--force]
```

## Description

Attaches rich reasoning metadata (intent, summary, dead ends, decisions, files) as [git notes](https://git-scm.com/docs/git-notes) to commits linked to engrams. Notes are stored under `refs/notes/engram` and viewable with `git loge` or `git log --notes=engram`.

Notes are also automatically attached during:
- `engram init` (annotates existing linked commits)
- Claude Code `SessionEnd` hook (annotates commits after session import)

Use this command to retroactively annotate commits or to force-refresh notes.

## Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `range` | string | all | Git range to annotate (e.g., `main..HEAD`) |
| `--dry-run` | bool | false | Preview what would be annotated without writing |
| `--force` | bool | false | Overwrite existing notes |

## Examples

```bash
# Annotate all commits linked to engrams
engram annotate

# Only annotate commits in a range
engram annotate main..HEAD

# Preview what would be annotated
engram annotate --dry-run

# Overwrite existing notes
engram annotate --force

# View annotated commits
git loge
git log --notes=engram
```

## See Also

- [init](init.md) -- Auto-annotates during initialization
- [Git Hooks](../guides/git-hooks.md) -- Auto-annotate on session end
