# Branch Review

Review a feature branch by reading the chain of intents instead of diffs.

## Intent-Based Review

Traditional code review reads diffs line by line. Intent-based review reads the reasoning chain: what was asked, what was tried, what was rejected, and what was decided.

```bash
engram review main..feature-branch
```

## How It Works

1. Walks `git log` for the given commit range
2. Finds commits with `Engram-Id:` trailers (injected by git hooks)
3. Reads the referenced engrams
4. Displays the chain of intents, summaries, dead ends, and decisions
5. Shows aggregate token usage and cost

## Output

The review shows:

- **Total commits** in the range
- **Engrams found** (linked via trailers)
- For each engram:
  - Summary
  - Agent and model
  - Dead ends explored
  - Decisions made
  - Files changed
- **Aggregate statistics**: total tokens, total cost

## Tips

- Review intents **before** reading code diffs
- Pay attention to **dead ends** -- they tell you what was tried and why it didn't work
- Check the **cost** -- it helps calibrate whether the agent's approach was efficient
- Use `engram show <id> --transcript` to dive into any specific session

## See Also

- [review](../cli/review.md) -- CLI reference
- [pr-summary](../cli/pr-summary.md) -- Auto-generate PR descriptions
- [Git Hooks](git-hooks.md) -- How `Engram-Id` trailers get injected
