# engram pr-summary

Generate a PR description from the engram chain.

## Usage

```bash
engram pr-summary <base>..<head>
```

## Description

Generates a structured pull request description from the engram chain in a commit range. The output includes:

- Summary of changes
- File changes across all sessions
- Reasoning chain (intents and decisions)
- Dead ends explored
- Aggregate token economics

## Arguments

| Argument | Description |
|----------|-------------|
| `<range>` | Commit range in `base..head` format |

## Examples

```bash
# Generate PR summary
engram pr-summary main..feature-branch

# JSON output
engram pr-summary main..feature-branch --format json

# Markdown output (for pasting into a PR)
engram pr-summary main..feature-branch --format markdown
```

## See Also

- [review](review.md) -- Interactive intent-based review
- [PR Summary Guide](../guides/pr-summary.md)
