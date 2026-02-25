# engram dashboard

Web-based dashboard for browsing engrams, cost breakdowns, and trends.

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

Searchable list of all engrams with ID, date, agent, model, summary, tokens, and cost. Click any engram to view full details including intent, file changes, dead ends, and decisions.

### Trend Tab

Daily cost and token usage trend for the last 30 days. Shows sessions per day, tokens consumed, and cost.

### Files Tab

Top files by cost, showing which files have consumed the most AI reasoning effort. Configurable top-N.

### Agents Tab

Breakdown by agent showing session count, total tokens, and total cost per agent.

## API Endpoints

The dashboard exposes a JSON API that can be used programmatically:

| Endpoint | Description |
|----------|-------------|
| `GET /api/engrams` | List engrams (supports `?limit=N&agent=name`) |
| `GET /api/engrams/{id}` | Full engram detail |
| `GET /api/stats` | Aggregate statistics |
| `GET /api/stats/trend` | Daily cost trend (30 days) |
| `GET /api/stats/by-file` | Cost by file (supports `?top=N`) |
| `GET /api/stats/by-agent` | Cost by agent |
| `GET /api/search` | Full-text search (supports `?q=query&limit=N`) |

## See Also

- [browse](browse.md) -- Terminal-based interactive browser
- [stats](stats.md) -- CLI statistics
- [log](log.md) -- List engrams in the terminal
