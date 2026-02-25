# Intent

<command-message>init</command-message>
<command-name>/init</command-name>

## Interpreted Goal

Let me read the readme file directly to capture the important details.

## Summary

<command-message>init</command-message>
<command-name>/init</command-name>

## Dead Ends

- **actual type**: 9. **Trace change_type** — Always returns "modified"
- **hardcoding "modified"**: - `engram trace` now fetches actual `FileChangeType` (created/modified/deleted)
- **`drop()`**: - Writer thread now checks shutdown flag and `join()` is attempted
- **walkdir**: 3. PTY file change detector description says "walkdir+sha2" — now uses `ignore` crate
- **in-memory**: - Persistent graph database (SQLite or embedded graph store)
