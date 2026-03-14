use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse};
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use tower_http::cors::CorsLayer;

use engram_core::notes::ENGRAM_NOTES_REF;
use engram_core::storage::{GitStorage, ListOptions};
use engram_query::{build_graph, SearchEngine};

const INDEX_HTML: &str = include_str!("../static/index.html");

struct AppState {
    repo_path: PathBuf,
}

fn open_storage(state: &AppState) -> Result<GitStorage, StatusCode> {
    GitStorage::open(&state.repo_path).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// Build the axum router for the dashboard.
pub fn build_router(repo_path: &Path) -> Router {
    let state = Arc::new(AppState {
        repo_path: repo_path.to_path_buf(),
    });

    Router::new()
        .route("/", get(index_handler))
        .route("/api/engrams", get(list_engrams))
        .route("/api/engrams/{id}", get(show_engram))
        .route("/api/stats", get(stats_handler))
        .route("/api/stats/trend", get(stats_trend))
        .route("/api/stats/by-file", get(stats_by_file))
        .route("/api/stats/by-agent", get(stats_by_agent))
        .route("/api/search", get(search_handler))
        .route("/api/notes", get(notes_handler))
        .route("/api/engrams/{id}/transcript", get(transcript_handler))
        .route("/api/graph", get(graph_handler))
        .layer(
            CorsLayer::new()
                .allow_origin(tower_http::cors::AllowOrigin::predicate(|origin, _| {
                    origin.to_str().ok().is_some_and(|o| {
                        o.starts_with("http://localhost") || o.starts_with("http://127.0.0.1")
                    })
                }))
                .allow_methods(tower_http::cors::Any)
                .allow_headers(tower_http::cors::Any),
        )
        .with_state(state)
}

/// Start the dashboard server.
pub async fn serve(repo_path: &Path, port: u16) -> std::io::Result<()> {
    let app = build_router(repo_path);
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    tracing::info!("Dashboard listening on http://localhost:{port}");
    axum::serve(listener, app).await
}

async fn index_handler() -> Html<&'static str> {
    Html(INDEX_HTML)
}

#[derive(Deserialize)]
struct ListParams {
    limit: Option<usize>,
    agent: Option<String>,
}

async fn list_engrams(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> impl IntoResponse {
    let storage = open_storage(&state)?;
    let opts = ListOptions {
        limit: Some(params.limit.unwrap_or(50)),
        agent_filter: params.agent,
    };
    let manifests = storage
        .list(&opts)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let entries: Vec<_> = manifests
        .iter()
        .map(|m| {
            serde_json::json!({
                "id": m.id.as_str(),
                "short_id": &m.id.as_str()[..8.min(m.id.as_str().len())],
                "created_at": m.created_at.to_rfc3339(),
                "agent": m.agent.name,
                "model": m.agent.model,
                "summary": m.summary,
                "tokens": m.token_usage.total_tokens,
                "cost_usd": m.token_usage.effective_cost(m.agent.model.as_deref()),
                "capture_mode": format!("{:?}", m.capture_mode),
                "files_changed": m.git_commits.len(),
            })
        })
        .collect();

    Ok::<_, StatusCode>(axum::Json(serde_json::json!({
        "engrams": entries,
        "total": entries.len(),
    })))
}

async fn show_engram(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let storage = open_storage(&state)?;
    let resolved = storage.resolve(&id).map_err(|_| StatusCode::NOT_FOUND)?;
    let data = storage.read(&resolved).map_err(|_| StatusCode::NOT_FOUND)?;

    let m = &data.manifest;
    Ok::<_, StatusCode>(axum::Json(serde_json::json!({
        "id": m.id.as_str(),
        "created_at": m.created_at.to_rfc3339(),
        "agent": m.agent.name,
        "model": m.agent.model,
        "summary": m.summary,
        "tokens": m.token_usage.total_tokens,
        "cost_usd": m.token_usage.effective_cost(m.agent.model.as_deref()),
        "capture_mode": format!("{:?}", m.capture_mode),
        "git_commits": m.git_commits,
        "intent": {
            "request": data.intent.original_request,
            "goal": data.intent.interpreted_goal,
            "summary": data.intent.summary,
        },
        "file_changes": data.operations.file_changes.iter().map(|fc| {
            serde_json::json!({
                "path": fc.path,
                "change_type": format!("{:?}", fc.change_type).to_lowercase(),
                "lines_added": fc.lines_added,
                "lines_removed": fc.lines_removed,
            })
        }).collect::<Vec<_>>(),
        "dead_ends": data.intent.dead_ends.iter().map(|de| {
            serde_json::json!({ "approach": de.approach, "reason": de.reason })
        }).collect::<Vec<_>>(),
        "decisions": data.intent.decisions.iter().map(|d| {
            serde_json::json!({ "description": d.description, "rationale": d.rationale })
        }).collect::<Vec<_>>(),
        "transcript_count": data.transcript.entries.len(),
    })))
}

async fn stats_handler(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let storage = open_storage(&state)?;
    let manifests = storage
        .list(&ListOptions::default())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut total_tokens: u64 = 0;
    let mut total_cost: f64 = 0.0;
    let mut by_agent: BTreeMap<String, (usize, u64, f64)> = BTreeMap::new();
    let mut by_mode: BTreeMap<String, usize> = BTreeMap::new();

    for m in &manifests {
        let cost = m
            .token_usage
            .effective_cost(m.agent.model.as_deref())
            .unwrap_or(0.0);
        total_tokens += m.token_usage.total_tokens;
        total_cost += cost;

        let entry = by_agent.entry(m.agent.name.clone()).or_default();
        entry.0 += 1;
        entry.1 += m.token_usage.total_tokens;
        entry.2 += cost;

        *by_mode.entry(format!("{:?}", m.capture_mode)).or_default() += 1;
    }

    Ok::<_, StatusCode>(axum::Json(serde_json::json!({
        "total_engrams": manifests.len(),
        "total_tokens": total_tokens,
        "total_cost_usd": total_cost,
        "earliest": manifests.last().map(|m| m.created_at.to_rfc3339()),
        "latest": manifests.first().map(|m| m.created_at.to_rfc3339()),
        "by_agent": by_agent.iter().map(|(name, (count, tokens, cost))| {
            serde_json::json!({ "agent": name, "count": count, "tokens": tokens, "cost_usd": cost })
        }).collect::<Vec<_>>(),
        "by_capture_mode": by_mode,
    })))
}

async fn stats_trend(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let storage = open_storage(&state)?;
    let manifests = storage
        .list(&ListOptions::default())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let cutoff = chrono::Utc::now() - chrono::Duration::days(30);
    let mut by_date: BTreeMap<chrono::NaiveDate, (usize, u64, f64)> = BTreeMap::new();

    for m in &manifests {
        if m.created_at < cutoff {
            continue;
        }
        let cost = m
            .token_usage
            .effective_cost(m.agent.model.as_deref())
            .unwrap_or(0.0);
        let date = m.created_at.date_naive();
        let entry = by_date.entry(date).or_default();
        entry.0 += 1;
        entry.1 += m.token_usage.total_tokens;
        entry.2 += cost;
    }

    let entries: Vec<_> = by_date
        .iter()
        .map(|(date, (sessions, tokens, cost))| {
            serde_json::json!({
                "date": date.to_string(),
                "sessions": sessions,
                "tokens": tokens,
                "cost_usd": cost,
            })
        })
        .collect();

    Ok::<_, StatusCode>(axum::Json(serde_json::json!({ "trend": entries })))
}

#[derive(Deserialize)]
struct TopParams {
    top: Option<usize>,
}

async fn stats_by_file(
    State(state): State<Arc<AppState>>,
    Query(params): Query<TopParams>,
) -> impl IntoResponse {
    let storage = open_storage(&state)?;
    let manifests = storage
        .list(&ListOptions::default())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let top = params.top.unwrap_or(10);

    let mut by_file: BTreeMap<String, (usize, u64, f64)> = BTreeMap::new();

    for m in &manifests {
        if let Ok(data) = storage.read(m.id.as_str()) {
            let file_count = data.operations.file_changes.len();
            if file_count == 0 {
                continue;
            }
            let per_file_tokens = m.token_usage.total_tokens / file_count as u64;
            let per_file_cost = m
                .token_usage
                .effective_cost(m.agent.model.as_deref())
                .unwrap_or(0.0)
                / file_count as f64;

            for fc in &data.operations.file_changes {
                let entry = by_file.entry(fc.path.clone()).or_default();
                entry.0 += 1;
                entry.1 += per_file_tokens;
                entry.2 += per_file_cost;
            }
        }
    }

    let mut sorted: Vec<_> = by_file.into_iter().collect();
    sorted.sort_by(|a, b| {
        b.1 .2
            .partial_cmp(&a.1 .2)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    sorted.truncate(top);

    let entries: Vec<_> = sorted
        .iter()
        .map(|(path, (sessions, tokens, cost))| {
            serde_json::json!({
                "file": path,
                "sessions": sessions,
                "tokens": tokens,
                "cost_usd": cost,
            })
        })
        .collect();

    Ok::<_, StatusCode>(axum::Json(serde_json::json!({ "files": entries })))
}

async fn stats_by_agent(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let storage = open_storage(&state)?;
    let manifests = storage
        .list(&ListOptions::default())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut by_agent: BTreeMap<String, (usize, u64, f64)> = BTreeMap::new();

    for m in &manifests {
        let cost = m
            .token_usage
            .effective_cost(m.agent.model.as_deref())
            .unwrap_or(0.0);
        let entry = by_agent.entry(m.agent.name.clone()).or_default();
        entry.0 += 1;
        entry.1 += m.token_usage.total_tokens;
        entry.2 += cost;
    }

    let entries: Vec<_> = by_agent
        .iter()
        .map(|(name, (count, tokens, cost))| {
            serde_json::json!({
                "agent": name,
                "count": count,
                "tokens": tokens,
                "cost_usd": cost,
            })
        })
        .collect();

    Ok::<_, StatusCode>(axum::Json(serde_json::json!({ "agents": entries })))
}

#[derive(Deserialize)]
struct SearchParams {
    q: String,
    limit: Option<usize>,
}

async fn search_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> impl IntoResponse {
    let storage = open_storage(&state)?;
    let engine = SearchEngine::open(&storage).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let results = engine
        .search(&storage, &params.q, params.limit.unwrap_or(20))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let entries: Vec<_> = results
        .iter()
        .map(|r| {
            let m = &r.manifest;
            serde_json::json!({
                "id": m.id.as_str(),
                "short_id": &m.id.as_str()[..8.min(m.id.as_str().len())],
                "summary": m.summary,
                "agent": m.agent.name,
                "model": m.agent.model,
                "score": r.score,
                "created_at": m.created_at.to_rfc3339(),
                "tokens": m.token_usage.total_tokens,
                "cost_usd": m.token_usage.effective_cost(m.agent.model.as_deref()),
            })
        })
        .collect();

    Ok::<_, StatusCode>(axum::Json(serde_json::json!({
        "query": params.q,
        "results": entries,
        "total": entries.len(),
    })))
}

// --- Notes handler ---

#[derive(Deserialize)]
struct NotesParams {
    limit: Option<usize>,
}

async fn notes_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<NotesParams>,
) -> impl IntoResponse {
    let storage = open_storage(&state)?;
    let repo = storage.repo();
    let limit = params.limit.unwrap_or(50);

    let mut revwalk = repo
        .revwalk()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    if repo.head().is_err() {
        return Ok::<_, StatusCode>(axum::Json(serde_json::json!({ "notes": [] })));
    }
    revwalk
        .push_head()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    revwalk
        .set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut notes = Vec::new();
    let mut walked = 0;

    for oid_result in revwalk {
        if walked >= limit {
            break;
        }
        let oid = match oid_result {
            Ok(o) => o,
            Err(_) => continue,
        };
        walked += 1;

        let note = match repo.find_note(Some(ENGRAM_NOTES_REF), oid) {
            Ok(n) => n,
            Err(_) => continue,
        };

        let note_text = note.message().unwrap_or("").to_string();
        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let sha = oid.to_string();
        let short_sha = sha[..8.min(sha.len())].to_string();
        let message = commit.summary().unwrap_or("").to_string();
        let author = commit.author().name().unwrap_or("unknown").to_string();
        let time = commit.time();
        let date = chrono::DateTime::from_timestamp(time.seconds(), 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_default();

        // Parse first line: [agent/model] $cost Ntok
        let first_line = note_text.lines().next().unwrap_or("");
        let (agent, model, cost, tokens) = parse_note_header(first_line);

        notes.push(serde_json::json!({
            "commit_sha": sha,
            "commit_short": short_sha,
            "commit_message": message,
            "commit_author": author,
            "commit_date": date,
            "note_text": note_text,
            "agent": agent,
            "model": model,
            "cost": cost,
            "tokens": tokens,
        }));
    }

    Ok::<_, StatusCode>(axum::Json(serde_json::json!({ "notes": notes })))
}

/// Parse the first line of an engram note: `[agent/model] $cost Ntok`
fn parse_note_header(line: &str) -> (String, String, String, String) {
    let mut agent = String::new();
    let mut model = String::new();
    let mut cost = String::new();
    let mut tokens = String::new();

    if let Some(bracket_end) = line.find(']') {
        if bracket_end == 0 || !line.starts_with('[') {
            return (agent, model, cost, tokens);
        }
        let inner = &line[1..bracket_end];
        if let Some(slash) = inner.find('/') {
            agent = inner[..slash].to_string();
            model = inner[slash + 1..].to_string();
        }
        let rest = line[bracket_end + 1..].trim();
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.len() >= 2 {
            cost = parts[0].to_string();
            tokens = parts[1].to_string();
        }
    }

    (agent, model, cost, tokens)
}

// --- Transcript handler ---

async fn transcript_handler(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let storage = open_storage(&state)?;
    let resolved = storage.resolve(&id).map_err(|_| StatusCode::NOT_FOUND)?;
    let data = storage.read(&resolved).map_err(|_| StatusCode::NOT_FOUND)?;

    let entries: Vec<serde_json::Value> = data
        .transcript
        .entries
        .iter()
        .map(|e| {
            let mut obj = serde_json::json!({
                "timestamp": e.timestamp.to_rfc3339(),
                "role": serde_json::to_value(&e.role).unwrap_or_default(),
                "token_count": e.token_count,
            });
            // Merge content fields into the same object
            if let Ok(content_val) = serde_json::to_value(&e.content) {
                if let Some(content_obj) = content_val.as_object() {
                    for (k, v) in content_obj {
                        obj[k] = v.clone();
                    }
                }
            }
            obj
        })
        .collect();

    Ok::<_, StatusCode>(axum::Json(serde_json::json!({
        "engram_id": id,
        "entry_count": entries.len(),
        "entries": entries,
    })))
}

// --- Graph handler ---

#[derive(Deserialize)]
struct GraphParams {
    center: Option<String>,
    depth: Option<usize>,
}

async fn graph_handler(
    State(state): State<Arc<AppState>>,
    Query(params): Query<GraphParams>,
) -> impl IntoResponse {
    let storage = open_storage(&state)?;
    let graph = build_graph(&storage).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result = if let Some(center) = &params.center {
        graph.subgraph(center, params.depth.unwrap_or(2))
    } else {
        graph
    };

    Ok::<_, StatusCode>(axum::Json(serde_json::json!({
        "nodes": result.nodes,
        "edges": result.edges,
        "node_count": result.nodes.len(),
        "edge_count": result.edges.len(),
    })))
}
