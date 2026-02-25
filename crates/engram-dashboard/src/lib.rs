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

use engram_core::storage::{GitStorage, ListOptions};
use engram_query::SearchEngine;

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
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Start the dashboard server.
pub async fn serve(repo_path: &Path, port: u16) -> std::io::Result<()> {
    let app = build_router(repo_path);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await?;
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
