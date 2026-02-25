# engram audit

Generate compliance reports mapping commits to AI reasoning chains.

## Usage

```bash
engram audit [range] [--report <format>] [-o <file>]
```

## Description

Walks the Git commit history for a given range (or all commits), finds the associated engram for each commit via `Engram-Id` trailers or `manifest.git_commits`, and produces a structured audit report. Commits without engram linkage are flagged as "untraced".

Useful for compliance reviews, team retrospectives, and understanding AI contribution coverage.

## Arguments

| Argument | Description |
|----------|-------------|
| `[range]` | Git range to audit (e.g., `main..HEAD`). Omit to audit all commits on the current branch. |

## Flags

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--report` | | `json`, `markdown`, `csv` | `markdown` | Report format |
| `--output` | `-o` | string | stdout | Write report to a file |

## Examples

```bash
# Audit all commits on current branch (markdown)
engram audit

# Audit a specific range as JSON
engram audit main..HEAD --report json

# Audit and save to file
engram audit v1.0..HEAD --report csv -o audit.csv

# Audit with markdown output to file
engram audit main..feature-branch --report markdown -o report.md
```

## Report Formats

### Markdown

Generates a table with coverage statistics and per-commit details. Untraced commits are listed separately.

### JSON

Structured output with `audit_report` summary (coverage percentage, total tokens, total cost) and per-commit `entries` array.

### CSV

Flat rows suitable for spreadsheet import: commit, date, author, message, traced, engram_id, agent, model, tokens, cost_usd.

## See Also

- [review](review.md) -- Review intent chain for a branch
- [pr-summary](pr-summary.md) -- Generate PR descriptions from engram chain
- [blame](blame.md) -- Reasoning blame for a file
