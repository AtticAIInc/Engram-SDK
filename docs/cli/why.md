# engram why

Explain why a file exists through its full reasoning chain.

## Usage

```bash
engram why <file>
```

## Description

Goes beyond `git blame` (which shows *who* changed a file) to explain *why* it exists. Produces a rich narrative tracing the file's reasoning chain -- every session that touched it, what was requested, what goals were set, what was tried and rejected, and what decisions were made.

## Examples

```bash
# Explain a file's reasoning history
engram why src/auth.rs

# JSON output
engram why src/auth.rs --format json
```

## See Also

- [trace](trace.md) -- Chronological reasoning history for a file
- [blame](blame.md) -- Reasoning blame for a file
