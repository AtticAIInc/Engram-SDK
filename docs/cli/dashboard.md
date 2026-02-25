# engram dashboard

Web-based dashboard for browsing engrams, cost breakdowns, trends, git notes, transcripts, and context graphs.

## Usage

```bash
engram dashboard --serve [--port <port>] [--open]
```

## Description

Starts a local web server that serves an interactive dashboard for exploring engram data. The dashboard is a single-page application served from a single binary (no external files or build steps required).

## Flags

| Flag | Type | Default | Description |
|------|------|---------|-------------|
| `--serve` | boolean | false | Start the web server (required) |
| `--port` | integer | 3000 | Port to listen on |
| `--open` | boolean | false | Open browser automatically after starting |

## Examples

```bash
# Start dashboard on default port
engram dashboard --serve

# Start on custom port and open browser
engram dashboard --serve --port 8080 --open
```

## Dashboard Features

### Engrams Tab

Searchable list of all engrams with ID, date, agent, model, summary, tokens, and cost. Click any engram to view full details including intent, file changes, dead ends, and decisions. The detail panel includes an inline transcript viewer for browsing the full session conversation.

### Trend Tab

Daily cost and token usage trend for the last 30 days. Shows sessions per day, tokens consumed, and cost.

### Files Tab

Top files by cost, showing which files have consumed the most AI reasoning effort. Configurable top-N.

### Agents Tab

Breakdown by agent showing session count, total tokens, and total cost per agent.

### Git Notes Tab

Browse git notes attached to commits. Shows commit SHA, message, date, and the full reasoning note with parsed metadata (agent, model, cost, tokens). Notes include intent, summary, dead ends, decisions, and file changes.

### Graph Tab

Interactive force-directed context graph visualization showing the relationships between engrams, files, agents, and commits. Nodes are color-coded by type:

- **Blue** -- Engrams
- **Yellow** -- Files
- **Green** -- Agents
- **Gray** -- Commits

Click any node to re-center the graph around it. Use the depth control to adjust how far the graph extends from the center node.

## API Endpoints

The dashboard exposes a JSON API that can be used programmatically:

| Endpoint | Description |
|----------|-------------|
| `GET /api/engrams` | List engrams (supports `?limit=N&agent=name`) |
| `GET /api/engrams/{id}` | Full engram detail (includes `transcript_count`) |
| `GET /api/engrams/{id}/transcript` | Full session transcript (all entries with role and content) |
| `GET /api/stats` | Aggregate statistics |
| `GET /api/stats/trend` | Daily cost trend (30 days) |
| `GET /api/stats/by-file` | Cost by file (supports `?top=N`) |
| `GET /api/stats/by-agent` | Cost by agent |
| `GET /api/search` | Full-text search (supports `?q=query&limit=N`) |
| `GET /api/notes` | Git notes on commits (supports `?limit=N`) |
| `GET /api/graph` | Context graph (supports `?center=id&depth=N`) |

## See Also

- [browse](browse.md) -- Terminal-based interactive browser
- [stats](stats.md) -- CLI statistics
- [log](log.md) -- List engrams in the terminal
