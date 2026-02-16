# engram review

Review intent chain for a branch range.

## Usage

```bash
engram review <base>..<head>
```

## Description

Walks the git log for the given commit range, finds commits with `Engram-Id:` trailers, and collects the referenced engrams. Displays the chain of intents, summaries, dead ends, decisions, and aggregate token usage.

This enables **intent-based code review** -- instead of reading diffs line by line, you read the reasoning chain: what was asked, what was tried, what was rejected, and what was decided.

## Arguments

| Argument | Description |
|----------|-------------|
| `<range>` | Commit range in `base..head` format |

## Examples

```bash
# Review a feature branch
engram review main..feature-branch

# Review recent commits
engram review HEAD~5..HEAD

# JSON output
engram review main..feature-branch --format json
```

## See Also

- [pr-summary](pr-summary.md) -- Generate a PR description
- [Branch Review Guide](../guides/branch-review.md)
