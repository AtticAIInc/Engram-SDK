# engram browse

Interactive terminal UI for browsing engrams.

## Usage

```bash
engram browse
```

## Description

Opens a full-screen terminal interface (TUI) for browsing engrams with keyboard navigation. The interface has a split-panel layout: engram list on the left, detail panel on the right.

The detail panel shows intent, file changes, dead ends, decisions, token usage, and cost for the selected engram.

## Layout

```
+----------------------+---------------------------+
| Engrams              | Detail                    |
| > abc123 02-20 14:30 | Summary text...           |
|   def456 02-19 11:15 | Agent: claude-code (opus) |
|   ghi789 02-18 09:00 | Date:  2025-02-20 14:30   |
|                      | Tokens: 12345 | Cost: $0.15|
|                      |                           |
|                      | Intent                    |
|                      |   Request: "Add OAuth2"   |
|                      |                           |
|                      | File Changes              |
|                      |   + src/auth.rs           |
|                      |   ~ src/main.rs           |
+----------------------+---------------------------+
| / search  j/k navigate  d/u scroll  q quit       |
+--------------------------------------------------+
```

## Keybindings

| Key | Action |
|-----|--------|
| `j` / `Down` | Move selection down |
| `k` / `Up` | Move selection up |
| `/` | Enter search mode |
| `d` | Scroll detail panel down |
| `u` | Scroll detail panel up |
| `q` | Quit |
| `Enter` (search mode) | Execute search |
| `Esc` (search mode) | Cancel search |

## Search

Press `/` to enter search mode. Type a query and press Enter to filter engrams using the full-text search index. Press Esc or clear the query and press Enter to reset to the full list.

## See Also

- [log](log.md) -- List engrams in the terminal
- [show](show.md) -- Show details of a specific engram
- [dashboard](dashboard.md) -- Web-based dashboard
