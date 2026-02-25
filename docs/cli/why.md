# engram why

Explain why a file (or a specific line) exists through its reasoning chain.

## Usage

```bash
engram why <file>[:<line>] [-n <limit>]
```

## Description

Goes beyond `git blame` (which shows *who* changed a file) to explain *why* it exists. Supports two modes:

### File-level

Produces a rich narrative tracing the file's reasoning chain -- every session that touched it, what was requested, what goals were set, what was tried and rejected, and what decisions were made.

### Line-level

When you append `:<line>` to the file path, engram uses `git blame` to find which commit last touched that specific line, then maps the commit back to the engram that produced it. Shows the exact AI reasoning behind a single line of code.

## Flags

| Flag | Short | Type | Default | Description |
|------|-------|------|---------|-------------|
| `--limit` | `-n` | integer | 20 | Maximum number of engrams to include (file-level only) |

## Examples

```bash
# Explain a file's full reasoning history
engram why src/auth.rs

# Explain why a specific line exists
engram why src/auth.rs:42

# Limit results
engram why src/auth.rs -n 5

# JSON output (file-level)
engram why src/auth.rs --format json

# JSON output (line-level)
engram why src/auth.rs:42 --format json
```

## Line-level Output

When a line is specified, the output includes:

- **Commit info**: SHA, author, message of the commit that last touched the line
- **AI reasoning**: Agent, model, original request, interpreted goal, summary
- **Dead ends**: Approaches tried and rejected during the session
- **Decisions**: Architectural decisions made during the session

If the commit has no associated engram, engram reports that the line was changed outside of a tracked session and suggests using file-level `engram why` instead.

## See Also

- [trace](trace.md) -- Chronological reasoning history for a file
- [blame](blame.md) -- Reasoning blame for a file
