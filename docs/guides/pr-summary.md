# PR Summary

Auto-generate structured PR descriptions from the engram chain.

## Usage

```bash
engram pr-summary main..feature-branch
```

## Output

Generates a structured description including:

- **Summary** -- What was accomplished across all sessions
- **File changes** -- All files created, modified, or deleted
- **Reasoning chain** -- Intent and decisions for each engram
- **Dead ends** -- Approaches that were tried and rejected
- **Token economics** -- Aggregate token usage and cost

## Output Formats

```bash
# Markdown (for pasting into a PR)
engram pr-summary main..feature-branch --format markdown

# JSON (for CI/CD integration)
engram pr-summary main..feature-branch --format json

# Plain text (default)
engram pr-summary main..feature-branch
```

## CI/CD Integration

You can automate PR description generation in your CI pipeline:

```yaml
# GitHub Actions example
- name: Generate PR description
  run: engram pr-summary ${{ github.event.pull_request.base.sha }}..${{ github.sha }} --format markdown
```

## See Also

- [pr-summary](../cli/pr-summary.md) -- CLI reference
- [Branch Review](branch-review.md) -- Interactive review
