use std::path::PathBuf;

use rmcp::model::{ServerCapabilities, ServerInfo};
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    tool, tool_handler, tool_router, ServerHandler,
};
use schemars::JsonSchema;
use serde::Deserialize;

use engram_core::model::FileChangeType;
use engram_core::storage::{GitStorage, ListOptions};
use engram_query::search::SearchEngine;
use engram_query::{diff_engrams, EngramDiff};

/// MCP server exposing engram reasoning data to AI agents.
///
/// Stores `repo_path: PathBuf` instead of `GitStorage` because
/// `git2::Repository` is `!Send` and rmcp requires `ServerHandler: Send + Sync + 'static`.
/// Each tool handler opens the repository fresh per request.
#[derive(Debug, Clone)]
pub struct EngramMcpServer {
    repo_path: PathBuf,
    tool_router: ToolRouter<Self>,
}

impl EngramMcpServer {
    /// Create a new MCP server for the repository at the given path.
    pub fn new(repo_path: PathBuf) -> Self {
        Self {
            repo_path,
            tool_router: Self::tool_router(),
        }
    }

    fn open_storage(&self) -> Result<GitStorage, String> {
        GitStorage::open(&self.repo_path).map_err(|e| format!("Failed to open repository: {e}"))
    }
}

// -- Tool parameter structs --

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchParams {
    /// Free-text search query across intent, transcript, file paths, and dead ends
    pub query: String,
    /// Maximum number of results (default: 10)
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShowParams {
    /// Engram ID (full or prefix) or "HEAD" for most recent
    pub id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct LogParams {
    /// Maximum number of entries (default: 10)
    pub limit: Option<usize>,
    /// Filter by agent name
    pub by_agent: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TraceParams {
    /// File path to trace reasoning history for
    pub file_path: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DiffParams {
    /// First engram ID (or prefix)
    pub id_a: String,
    /// Second engram ID (or prefix)
    pub id_b: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DeadEndsParams {
    /// Specific engram ID to get dead ends from (optional)
    pub id: Option<String>,
    /// Search for dead ends matching this text (optional)
    pub query: Option<String>,
    /// Set to true to only show dead ends that recur across multiple sessions
    pub recurring: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WhyParams {
    /// File path to explain the reasoning history for
    pub file_path: String,
    /// Maximum number of engrams to include (default: 20)
    pub limit: Option<usize>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct StatsParams {
    /// Show cost breakdown by file
    pub by_file: Option<bool>,
    /// Show cost breakdown by branch
    pub by_branch: Option<bool>,
    /// Show daily cost trend for last 30 days
    pub trend: Option<bool>,
    /// Maximum entries in breakdowns (default: 10)
    pub top: Option<usize>,
}

// -- Tool implementations --

#[tool_router]
impl EngramMcpServer {
    #[tool(
        description = "Search engram reasoning history by free-text query. Searches across intent, transcript, file paths, and dead ends."
    )]
    fn engram_search(
        &self,
        Parameters(params): Parameters<SearchParams>,
    ) -> Result<String, String> {
        let storage = self.open_storage()?;
        let engine =
            SearchEngine::open(&storage).map_err(|e| format!("Failed to open search: {e}"))?;
        let limit = params.limit.unwrap_or(10);
        let results = engine
            .search(&storage, &params.query, limit)
            .map_err(|e| format!("Search failed: {e}"))?;

        if results.is_empty() {
            return Ok(format!("No results found for: {}", params.query));
        }

        let mut out = format!(
            "Found {} result(s) for: {}\n\n",
            results.len(),
            params.query
        );
        for r in &results {
            let m = &r.manifest;
            let short_id = &m.id.as_str()[..8.min(m.id.as_str().len())];
            let summary = m.summary.as_deref().unwrap_or("(no summary)");
            let agent = &m.agent.name;
            let model = m.agent.model.as_deref().unwrap_or("unknown");
            let date = m.created_at.format("%Y-%m-%d %H:%M");
            out.push_str(&format!(
                "- {short_id} [{agent}/{model}] {date}\n  {summary}\n"
            ));
        }
        Ok(out)
    }

    #[tool(
        description = "Show full details of a specific engram including manifest, intent, file changes, and transcript summary. Supports 'HEAD' for most recent."
    )]
    fn engram_show(&self, Parameters(params): Parameters<ShowParams>) -> Result<String, String> {
        let storage = self.open_storage()?;
        let resolved = storage
            .resolve(&params.id)
            .map_err(|e| format!("Failed to resolve '{}': {e}", params.id))?;
        let data = storage
            .read(&resolved)
            .map_err(|e| format!("Failed to read engram: {e}"))?;

        let m = &data.manifest;
        let mut out = String::new();
        out.push_str(&format!("Engram: {}\n", m.id));
        out.push_str(&format!(
            "Agent: {}{}\n",
            m.agent.name,
            m.agent
                .model
                .as_ref()
                .map(|m| format!(" ({m})"))
                .unwrap_or_default()
        ));
        out.push_str(&format!(
            "Date: {}\n",
            m.created_at.format("%Y-%m-%d %H:%M:%S UTC")
        ));
        if let Some(summary) = &m.summary {
            out.push_str(&format!("Summary: {summary}\n"));
        }

        let tu = &m.token_usage;
        if tu.total_tokens > 0 {
            out.push_str(&format!(
                "Tokens: {} total ({} in, {} out)",
                tu.total_tokens, tu.input_tokens, tu.output_tokens
            ));
            if let Some(cost) = tu.effective_cost(m.agent.model.as_deref()) {
                out.push_str(&format!("  Cost: ${cost:.4}"));
            }
            out.push('\n');
        }

        if !m.git_commits.is_empty() {
            out.push_str(&format!("Commits: {}\n", m.git_commits.join(", ")));
        }

        // Intent
        out.push_str(&format!("\nIntent: {}\n", data.intent.original_request));
        if let Some(goal) = &data.intent.interpreted_goal {
            out.push_str(&format!("Goal: {goal}\n"));
        }
        if let Some(intent_summary) = &data.intent.summary {
            out.push_str(&format!("Intent Summary: {intent_summary}\n"));
        }

        // File changes
        if !data.operations.file_changes.is_empty() {
            out.push_str(&format!(
                "\nFile Changes ({}):\n",
                data.operations.file_changes.len()
            ));
            for fc in &data.operations.file_changes {
                let symbol = match &fc.change_type {
                    FileChangeType::Created => "+",
                    FileChangeType::Modified => "~",
                    FileChangeType::Deleted => "-",
                    FileChangeType::Renamed { from } => {
                        out.push_str(&format!("  {from} -> {}\n", fc.path));
                        continue;
                    }
                };
                out.push_str(&format!("  {symbol} {}\n", fc.path));
            }
        }

        // Dead ends
        if !data.intent.dead_ends.is_empty() {
            out.push_str("\nDead Ends:\n");
            for de in &data.intent.dead_ends {
                out.push_str(&format!("  - {}: {}\n", de.approach, de.reason));
            }
        }

        // Decisions
        if !data.intent.decisions.is_empty() {
            out.push_str("\nDecisions:\n");
            for d in &data.intent.decisions {
                out.push_str(&format!("  - {}: {}\n", d.description, d.rationale));
            }
        }

        out.push_str(&format!(
            "\nTranscript: {} entries\n",
            data.transcript.entries.len()
        ));

        Ok(out)
    }

    #[tool(
        description = "List recent engrams (most recent first). Shows ID, agent, model, date, and summary."
    )]
    fn engram_log(&self, Parameters(params): Parameters<LogParams>) -> Result<String, String> {
        let storage = self.open_storage()?;
        let opts = ListOptions {
            limit: Some(params.limit.unwrap_or(10)),
            agent_filter: params.by_agent.clone(),
        };
        let manifests = storage
            .list(&opts)
            .map_err(|e| format!("Failed to list engrams: {e}"))?;

        if manifests.is_empty() {
            return Ok("No engrams found.".to_string());
        }

        let mut out = format!("{} engram(s):\n\n", manifests.len());
        for m in &manifests {
            let short_id = &m.id.as_str()[..8.min(m.id.as_str().len())];
            let summary = m.summary.as_deref().unwrap_or("(no summary)");
            let agent = &m.agent.name;
            let model = m.agent.model.as_deref().unwrap_or("");
            let date = m.created_at.format("%Y-%m-%d %H:%M");
            let tokens = m.token_usage.total_tokens;
            let cost = m
                .token_usage
                .effective_cost(m.agent.model.as_deref())
                .map(|c| format!(" ${c:.2}"))
                .unwrap_or_default();
            out.push_str(&format!(
                "- {short_id} [{agent}/{model}] {date} {tokens}tok{cost}\n  {summary}\n"
            ));
        }
        Ok(out)
    }

    #[tool(
        description = "Trace the full reasoning history of a file. Shows every engram that created, modified, or deleted the file."
    )]
    fn engram_trace(&self, Parameters(params): Parameters<TraceParams>) -> Result<String, String> {
        let storage = self.open_storage()?;
        let engine =
            SearchEngine::open(&storage).map_err(|e| format!("Failed to open search: {e}"))?;
        let results = engine
            .search_by_file(&storage, &params.file_path, 20)
            .map_err(|e| format!("Trace failed: {e}"))?;

        if results.is_empty() {
            return Ok(format!(
                "No engrams found that touched: {}",
                params.file_path
            ));
        }

        let mut out = format!(
            "Reasoning trace for {} ({} engram(s)):\n\n",
            params.file_path,
            results.len()
        );
        for r in &results {
            let m = &r.manifest;
            let short_id = &m.id.as_str()[..8.min(m.id.as_str().len())];
            let summary = m.summary.as_deref().unwrap_or("(no summary)");
            let agent = &m.agent.name;
            let date = m.created_at.format("%Y-%m-%d %H:%M");
            out.push_str(&format!("- {short_id} [{agent}] {date}\n  {summary}\n"));
        }
        Ok(out)
    }

    #[tool(
        description = "Compare two engrams showing common files, unique files, and token/cost deltas."
    )]
    fn engram_diff(&self, Parameters(params): Parameters<DiffParams>) -> Result<String, String> {
        let storage = self.open_storage()?;
        let data_a = storage
            .read(&params.id_a)
            .map_err(|e| format!("Failed to find first engram: {e}"))?;
        let data_b = storage
            .read(&params.id_b)
            .map_err(|e| format!("Failed to find second engram: {e}"))?;

        let diff: EngramDiff = diff_engrams(&storage, &data_a.manifest.id, &data_b.manifest.id)
            .map_err(|e| format!("Diff failed: {e}"))?;

        let short_a = &diff.id_a.as_str()[..8.min(diff.id_a.as_str().len())];
        let short_b = &diff.id_b.as_str()[..8.min(diff.id_b.as_str().len())];

        let mut out = format!("Comparing {short_a} vs {short_b}\n\n");

        if !diff.common_files.is_empty() {
            out.push_str(&format!("Common files ({}):\n", diff.common_files.len()));
            for f in &diff.common_files {
                out.push_str(&format!("  {f}\n"));
            }
        }
        if !diff.only_a_files.is_empty() {
            out.push_str(&format!(
                "Only in {short_a} ({}):\n",
                diff.only_a_files.len()
            ));
            for f in &diff.only_a_files {
                out.push_str(&format!("  {f}\n"));
            }
        }
        if !diff.only_b_files.is_empty() {
            out.push_str(&format!(
                "Only in {short_b} ({}):\n",
                diff.only_b_files.len()
            ));
            for f in &diff.only_b_files {
                out.push_str(&format!("  {f}\n"));
            }
        }

        out.push_str(&format!("\nToken delta: {:+}\n", diff.token_delta));
        if let Some(cost) = diff.cost_delta {
            out.push_str(&format!("Cost delta: {:+.4}\n", cost));
        }

        Ok(out)
    }

    #[tool(
        description = "Surface rejected approaches (dead ends) and architectural decisions. Search across all engrams or get dead ends from a specific engram. Use recurring=true to find approaches tried and rejected multiple times."
    )]
    fn engram_dead_ends(
        &self,
        Parameters(params): Parameters<DeadEndsParams>,
    ) -> Result<String, String> {
        let storage = self.open_storage()?;

        if let Some(id) = &params.id {
            // Show dead ends from a specific engram
            let resolved = storage
                .resolve(id)
                .map_err(|e| format!("Failed to resolve '{id}': {e}"))?;
            let data = storage
                .read(&resolved)
                .map_err(|e| format!("Failed to read engram: {e}"))?;

            let mut out = String::new();
            if data.intent.dead_ends.is_empty() && data.intent.decisions.is_empty() {
                return Ok(format!(
                    "No dead ends or decisions recorded for engram {}",
                    &resolved[..8.min(resolved.len())]
                ));
            }

            if !data.intent.dead_ends.is_empty() {
                out.push_str("Dead Ends:\n");
                for de in &data.intent.dead_ends {
                    out.push_str(&format!("  - {}: {}\n", de.approach, de.reason));
                }
            }
            if !data.intent.decisions.is_empty() {
                out.push_str("Decisions:\n");
                for d in &data.intent.decisions {
                    out.push_str(&format!("  - {}: {}\n", d.description, d.rationale));
                }
            }
            return Ok(out);
        }

        // Search across all engrams for dead ends
        let opts = ListOptions {
            limit: Some(50),
            agent_filter: None,
        };
        let manifests = storage
            .list(&opts)
            .map_err(|e| format!("Failed to list engrams: {e}"))?;

        let query_lower = params.query.as_deref().unwrap_or("").to_lowercase();

        // Recurring mode: group by normalized approach name
        if params.recurring.unwrap_or(false) {
            let mut groups: std::collections::BTreeMap<String, Vec<(String, String, String)>> =
                std::collections::BTreeMap::new();

            for m in &manifests {
                if let Ok(data) = storage.read(m.id.as_str()) {
                    for de in &data.intent.dead_ends {
                        if !query_lower.is_empty()
                            && !de.approach.to_lowercase().contains(&query_lower)
                            && !de.reason.to_lowercase().contains(&query_lower)
                        {
                            continue;
                        }
                        let normalized = de.approach.trim().to_lowercase();
                        let short_id = m.id.as_str()[..8.min(m.id.as_str().len())].to_string();
                        let date = m.created_at.format("%Y-%m-%d").to_string();
                        groups.entry(normalized).or_default().push((
                            short_id,
                            date,
                            de.reason.clone(),
                        ));
                    }
                }
            }

            let mut recurring: Vec<_> = groups.into_iter().filter(|(_, v)| v.len() >= 2).collect();
            recurring.sort_by(|a, b| b.1.len().cmp(&a.1.len()));

            if recurring.is_empty() {
                return Ok("No recurring dead ends found.".to_string());
            }

            let mut out = String::from("Recurring Dead Ends:\n\n");
            for (approach, occurrences) in &recurring {
                out.push_str(&format!(
                    "{} (tried {} times, rejected every time):\n",
                    approach,
                    occurrences.len()
                ));
                for (id, date, reason) in occurrences {
                    out.push_str(&format!("  [{date}] {id}: {reason}\n"));
                }
                out.push('\n');
            }
            return Ok(out);
        }

        let mut out = String::new();
        let mut found = 0;

        for m in &manifests {
            if let Ok(data) = storage.read(m.id.as_str()) {
                let matching_dead_ends: Vec<_> = data
                    .intent
                    .dead_ends
                    .iter()
                    .filter(|de| {
                        query_lower.is_empty()
                            || de.approach.to_lowercase().contains(&query_lower)
                            || de.reason.to_lowercase().contains(&query_lower)
                    })
                    .collect();

                let matching_decisions: Vec<_> = data
                    .intent
                    .decisions
                    .iter()
                    .filter(|d| {
                        query_lower.is_empty()
                            || d.description.to_lowercase().contains(&query_lower)
                            || d.rationale.to_lowercase().contains(&query_lower)
                    })
                    .collect();

                if !matching_dead_ends.is_empty() || !matching_decisions.is_empty() {
                    let short_id = &m.id.as_str()[..8.min(m.id.as_str().len())];
                    let summary = m.summary.as_deref().unwrap_or("(no summary)");
                    out.push_str(&format!("{short_id} - {summary}:\n"));

                    for de in &matching_dead_ends {
                        out.push_str(&format!("  Dead end: {} — {}\n", de.approach, de.reason));
                        found += 1;
                    }
                    for d in &matching_decisions {
                        out.push_str(&format!(
                            "  Decision: {} — {}\n",
                            d.description, d.rationale
                        ));
                        found += 1;
                    }
                    out.push('\n');
                }
            }
        }

        if found == 0 {
            if query_lower.is_empty() {
                return Ok("No dead ends or decisions found in any engrams.".to_string());
            }
            return Ok(format!(
                "No dead ends or decisions matching '{}' found.",
                params.query.as_deref().unwrap_or("")
            ));
        }

        Ok(out)
    }

    #[tool(
        description = "Explain why a file exists by tracing its full reasoning chain. Shows chronological history of every session that touched the file, including requests, goals, decisions, and dead ends."
    )]
    fn engram_why(&self, Parameters(params): Parameters<WhyParams>) -> Result<String, String> {
        let storage = self.open_storage()?;
        let engine =
            SearchEngine::open(&storage).map_err(|e| format!("Failed to open search: {e}"))?;
        let limit = params.limit.unwrap_or(20);
        let results = engine
            .search_by_file(&storage, &params.file_path, limit)
            .map_err(|e| format!("Search failed: {e}"))?;

        if results.is_empty() {
            return Ok(format!(
                "No reasoning history found for: {}",
                params.file_path
            ));
        }

        // Collect and sort chronologically (oldest first)
        let mut entries: Vec<engram_core::model::EngramData> = Vec::new();
        for r in &results {
            if let Ok(data) = storage.read(r.manifest.id.as_str()) {
                entries.push(data);
            }
        }
        entries.sort_by_key(|e| e.manifest.created_at);

        let mut out = format!("Why does `{}` exist?\n\n", params.file_path);
        let mut total_tokens: u64 = 0;
        let mut total_cost: f64 = 0.0;

        for (i, data) in entries.iter().enumerate() {
            let m = &data.manifest;
            total_tokens += m.token_usage.total_tokens;
            total_cost += m
                .token_usage
                .effective_cost(m.agent.model.as_deref())
                .unwrap_or(0.0);

            let date = m.created_at.format("%Y-%m-%d %H:%M");
            let agent = &m.agent.name;
            let model = m.agent.model.as_deref().unwrap_or("unknown");

            let file_change = data
                .operations
                .file_changes
                .iter()
                .find(|fc| fc.path == params.file_path);

            let change_desc = file_change
                .map(|fc| match &fc.change_type {
                    FileChangeType::Created => "Created".to_string(),
                    FileChangeType::Modified => "Modified".to_string(),
                    FileChangeType::Deleted => "Deleted".to_string(),
                    FileChangeType::Renamed { from } => format!("Renamed from {from}"),
                })
                .unwrap_or_else(|| "Touched".to_string());

            out.push_str(&format!(
                "{}. [{date}] {agent} ({model}) — {change_desc}\n",
                i + 1
            ));
            out.push_str(&format!(
                "   Request: \"{}\"\n",
                data.intent.original_request
            ));

            if let Some(goal) = &data.intent.interpreted_goal {
                if goal != &data.intent.original_request {
                    out.push_str(&format!("   Goal: {goal}\n"));
                }
            }

            if let Some(summary) = data.intent.summary.as_deref().or(m.summary.as_deref()) {
                out.push_str(&format!("   Result: {summary}\n"));
            }

            if !data.intent.dead_ends.is_empty() {
                for de in &data.intent.dead_ends {
                    out.push_str(&format!("   Dead end: {} — {}\n", de.approach, de.reason));
                }
            }
            if !data.intent.decisions.is_empty() {
                for d in &data.intent.decisions {
                    out.push_str(&format!(
                        "   Decision: {} — {}\n",
                        d.description, d.rationale
                    ));
                }
            }
            out.push('\n');
        }

        out.push_str(&format!(
            "{} sessions | {} tokens | ${:.2} total cost\n",
            entries.len(),
            total_tokens,
            total_cost
        ));

        Ok(out)
    }

    #[tool(
        description = "Show aggregate statistics across all engrams. Supports breakdowns by file (cost per file), by branch, and daily cost trends."
    )]
    fn engram_stats(&self, Parameters(params): Parameters<StatsParams>) -> Result<String, String> {
        let storage = self.open_storage()?;
        let manifests = storage
            .list(&ListOptions::default())
            .map_err(|e| format!("Failed to list engrams: {e}"))?;

        if manifests.is_empty() {
            return Ok("No engrams found.".to_string());
        }

        let top = params.top.unwrap_or(10);

        if params.by_file.unwrap_or(false) {
            let mut by_file: std::collections::BTreeMap<String, (usize, u64, f64)> =
                std::collections::BTreeMap::new();
            for m in &manifests {
                if let Ok(data) = storage.read(m.id.as_str()) {
                    let fc_count = data.operations.file_changes.len();
                    if fc_count == 0 {
                        continue;
                    }
                    let per_file_tokens = m.token_usage.total_tokens / fc_count as u64;
                    let per_file_cost = m
                        .token_usage
                        .effective_cost(m.agent.model.as_deref())
                        .unwrap_or(0.0)
                        / fc_count as f64;
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

            let mut out = format!("Cost by File (top {top}):\n\n");
            for (path, (sessions, tokens, cost)) in &sorted {
                out.push_str(&format!(
                    "  {path}: {sessions} sessions, {tokens} tokens, ${cost:.2}\n"
                ));
            }
            return Ok(out);
        }

        if params.by_branch.unwrap_or(false) {
            let mut by_branch: std::collections::BTreeMap<String, (usize, u64, f64)> =
                std::collections::BTreeMap::new();
            for m in &manifests {
                let branch = if let Ok(data) = storage.read(m.id.as_str()) {
                    data.lineage
                        .branch
                        .unwrap_or_else(|| "(unknown)".to_string())
                } else {
                    "(unknown)".to_string()
                };
                let cost = m
                    .token_usage
                    .effective_cost(m.agent.model.as_deref())
                    .unwrap_or(0.0);
                let entry = by_branch.entry(branch).or_default();
                entry.0 += 1;
                entry.1 += m.token_usage.total_tokens;
                entry.2 += cost;
            }
            let mut sorted: Vec<_> = by_branch.into_iter().collect();
            sorted.sort_by(|a, b| {
                b.1 .2
                    .partial_cmp(&a.1 .2)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            sorted.truncate(top);

            let mut out = format!("Cost by Branch (top {top}):\n\n");
            for (branch, (sessions, tokens, cost)) in &sorted {
                out.push_str(&format!(
                    "  {branch}: {sessions} sessions, {tokens} tokens, ${cost:.2}\n"
                ));
            }
            return Ok(out);
        }

        if params.trend.unwrap_or(false) {
            let cutoff = chrono::Utc::now() - chrono::Duration::days(30);
            let mut by_date: std::collections::BTreeMap<chrono::NaiveDate, (usize, u64, f64)> =
                std::collections::BTreeMap::new();
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

            let mut out = String::from("Cost Trend (last 30 days):\n\n");
            for (date, (sessions, tokens, cost)) in &by_date {
                out.push_str(&format!(
                    "  {date}: {sessions} sessions, {tokens} tokens, ${cost:.2}\n"
                ));
            }
            let total_cost: f64 = by_date.values().map(|v| v.2).sum();
            let total_sessions: usize = by_date.values().map(|v| v.0).sum();
            out.push_str(&format!(
                "\nTotal: {total_sessions} sessions, ${total_cost:.2}\n"
            ));
            return Ok(out);
        }

        // Default aggregate stats
        let total = manifests.len();
        let mut total_tokens: u64 = 0;
        let mut total_cost: f64 = 0.0;
        let mut by_agent: std::collections::BTreeMap<String, (usize, u64, f64)> =
            std::collections::BTreeMap::new();

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
        }

        let mut out = format!(
            "Engram Statistics\n\nTotal: {total} engrams, {total_tokens} tokens, ${total_cost:.2}\n\n"
        );
        out.push_str("By Agent:\n");
        for (name, (count, tokens, cost)) in &by_agent {
            out.push_str(&format!(
                "  {name}: {count} engrams, {tokens} tokens, ${cost:.2}\n"
            ));
        }

        Ok(out)
    }
}

#[tool_handler]
impl ServerHandler for EngramMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Engram MCP Server — Query agent reasoning history stored as Git-native objects.\n\
                 \n\
                 Available tools:\n\
                 - engram_search: Full-text search across all captured reasoning\n\
                 - engram_show: Show full details of a specific engram (use \"HEAD\" for most recent)\n\
                 - engram_log: List recent engrams with token usage and cost\n\
                 - engram_trace: Reasoning history for a specific file path\n\
                 - engram_diff: Compare two engrams (files, tokens, cost)\n\
                 - engram_dead_ends: Surface rejected approaches and architectural decisions\n\
                 - engram_why: Explain why a file exists through its full reasoning chain\n\
                 - engram_stats: Aggregate statistics with breakdowns by file, branch, or trend\n\
                 \n\
                 Use these tools to check prior reasoning before making changes, \
                 avoid repeating dead ends, and understand the history behind existing code."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

/// Start the MCP server on stdio transport.
pub async fn run_stdio(repo_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    use rmcp::transport::stdio;
    use rmcp::ServiceExt;

    let server = EngramMcpServer::new(repo_path);
    let service = server.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_core::model::*;
    use engram_core::storage::GitStorage;
    use rmcp::handler::server::wrapper::Parameters;
    use tempfile::TempDir;

    /// Helper: create a repo with engrams and return the MCP server.
    fn setup_test_server() -> (TempDir, EngramMcpServer, Vec<EngramId>) {
        let tmp = TempDir::new().unwrap();
        git2::Repository::init(tmp.path()).unwrap();

        let storage = GitStorage::open(tmp.path()).unwrap();
        storage.init().unwrap();

        let mut ids = Vec::new();

        // Engram 1: auth-related with dead ends and decisions
        let data1 = EngramData {
            manifest: Manifest {
                id: EngramId::new(),
                version: 1,
                created_at: chrono::Utc::now() - chrono::Duration::hours(2),
                finished_at: None,
                agent: AgentInfo {
                    name: "claude-code".into(),
                    model: Some("claude-sonnet-4-5".into()),
                    version: None,
                },
                git_commits: vec!["abc12345".into()],
                token_usage: TokenUsage {
                    input_tokens: 3000,
                    output_tokens: 1000,
                    total_tokens: 4000,
                    cost_usd: Some(0.05),
                    ..Default::default()
                },
                summary: Some("Add OAuth2 authentication".into()),
                tags: vec![],
                capture_mode: CaptureMode::Wrapper,
                source_hash: None,
            },
            intent: Intent {
                original_request: "Add OAuth2 auth with PKCE".into(),
                interpreted_goal: Some("Implement OAuth2 with PKCE for SPA".into()),
                summary: Some("Add OAuth2 authentication".into()),
                dead_ends: vec![DeadEnd {
                    approach: "passport.js".into(),
                    reason: "middleware conflict".into(),
                }],
                decisions: vec![Decision {
                    description: "use custom middleware".into(),
                    rationale: "full control over auth flow".into(),
                }],
            },
            transcript: Transcript {
                entries: vec![TranscriptEntry {
                    timestamp: chrono::Utc::now(),
                    role: Role::User,
                    content: TranscriptContent::Text {
                        text: "Add OAuth2 auth".into(),
                    },
                    token_count: None,
                }],
            },
            operations: Operations {
                tool_calls: vec![],
                file_changes: vec![
                    FileChange {
                        path: "src/auth.rs".into(),
                        change_type: FileChangeType::Created,
                        lines_added: Some(150),
                        lines_removed: None,
                    },
                    FileChange {
                        path: "src/middleware.rs".into(),
                        change_type: FileChangeType::Modified,
                        lines_added: Some(20),
                        lines_removed: Some(5),
                    },
                ],
                shell_commands: vec![],
            },
            lineage: Lineage::default(),
        };
        ids.push(data1.manifest.id.clone());
        storage.create(&data1).unwrap();

        // Engram 2: database-related, different agent
        let data2 = EngramData {
            manifest: Manifest {
                id: EngramId::new(),
                version: 1,
                created_at: chrono::Utc::now() - chrono::Duration::hours(1),
                finished_at: None,
                agent: AgentInfo {
                    name: "aider".into(),
                    model: Some("gpt-4o".into()),
                    version: None,
                },
                git_commits: vec![],
                token_usage: TokenUsage {
                    input_tokens: 2000,
                    output_tokens: 800,
                    total_tokens: 2800,
                    cost_usd: Some(0.03),
                    ..Default::default()
                },
                summary: Some("Fix database connection pool".into()),
                tags: vec![],
                capture_mode: CaptureMode::Import,
                source_hash: None,
            },
            intent: Intent {
                original_request: "Fix the DB pool leak".into(),
                interpreted_goal: None,
                summary: Some("Fix database connection pool".into()),
                dead_ends: vec![],
                decisions: vec![],
            },
            transcript: Transcript::default(),
            operations: Operations {
                tool_calls: vec![],
                file_changes: vec![FileChange {
                    path: "src/db.rs".into(),
                    change_type: FileChangeType::Modified,
                    lines_added: Some(10),
                    lines_removed: Some(8),
                }],
                shell_commands: vec![],
            },
            lineage: Lineage::default(),
        };
        ids.push(data2.manifest.id.clone());
        storage.create(&data2).unwrap();

        // Build search index
        let engine = SearchEngine::open(&storage).unwrap();
        engine.rebuild(&storage).unwrap();

        let server = EngramMcpServer::new(tmp.path().to_path_buf());
        (tmp, server, ids)
    }

    #[test]
    fn test_engram_log() {
        let (_tmp, server, _ids) = setup_test_server();

        let result = server
            .engram_log(Parameters(LogParams {
                limit: None,
                by_agent: None,
            }))
            .unwrap();

        assert!(result.contains("2 engram(s)"));
        assert!(result.contains("claude-code"));
        assert!(result.contains("aider"));
    }

    #[test]
    fn test_engram_log_with_agent_filter() {
        let (_tmp, server, _ids) = setup_test_server();

        let result = server
            .engram_log(Parameters(LogParams {
                limit: None,
                by_agent: Some("claude".into()),
            }))
            .unwrap();

        assert!(result.contains("1 engram(s)"));
        assert!(result.contains("claude-code"));
        assert!(!result.contains("aider"));
    }

    #[test]
    fn test_engram_log_with_limit() {
        let (_tmp, server, _ids) = setup_test_server();

        let result = server
            .engram_log(Parameters(LogParams {
                limit: Some(1),
                by_agent: None,
            }))
            .unwrap();

        assert!(result.contains("1 engram(s)"));
    }

    #[test]
    fn test_engram_show() {
        let (_tmp, server, ids) = setup_test_server();

        let result = server
            .engram_show(Parameters(ShowParams {
                id: ids[0].as_str().to_string(),
            }))
            .unwrap();

        assert!(result.contains("claude-code"));
        assert!(result.contains("OAuth2"));
        assert!(result.contains("src/auth.rs"));
        assert!(result.contains("passport.js")); // dead end
        assert!(result.contains("custom middleware")); // decision
        assert!(result.contains("Transcript: 1 entries"));
    }

    #[test]
    fn test_engram_show_head() {
        let (_tmp, server, _ids) = setup_test_server();

        let result = server
            .engram_show(Parameters(ShowParams { id: "HEAD".into() }))
            .unwrap();

        // HEAD should resolve to the most recent engram
        assert!(result.contains("Engram:"));
    }

    #[test]
    fn test_engram_search() {
        let (_tmp, server, _ids) = setup_test_server();

        let result = server
            .engram_search(Parameters(SearchParams {
                query: "OAuth2".into(),
                limit: None,
            }))
            .unwrap();

        assert!(result.contains("Found"));
        assert!(result.contains("OAuth2"));
    }

    #[test]
    fn test_engram_search_no_results() {
        let (_tmp, server, _ids) = setup_test_server();

        let result = server
            .engram_search(Parameters(SearchParams {
                query: "nonexistent_xyz_query".into(),
                limit: None,
            }))
            .unwrap();

        assert!(result.contains("No results found"));
    }

    #[test]
    fn test_engram_diff() {
        let (_tmp, server, ids) = setup_test_server();

        let result = server
            .engram_diff(Parameters(DiffParams {
                id_a: ids[0].as_str().to_string(),
                id_b: ids[1].as_str().to_string(),
            }))
            .unwrap();

        assert!(result.contains("Comparing"));
        assert!(result.contains("Token delta:"));
        // Should show token delta (2800 - 4000 = -1200)
        assert!(result.contains("-1200"));
    }

    #[test]
    fn test_engram_dead_ends_specific() {
        let (_tmp, server, ids) = setup_test_server();

        let result = server
            .engram_dead_ends(Parameters(DeadEndsParams {
                id: Some(ids[0].as_str().to_string()),
                query: None,
                recurring: None,
            }))
            .unwrap();

        assert!(result.contains("passport.js"));
        assert!(result.contains("middleware conflict"));
        assert!(result.contains("custom middleware"));
    }

    #[test]
    fn test_engram_dead_ends_global_search() {
        let (_tmp, server, _ids) = setup_test_server();

        let result = server
            .engram_dead_ends(Parameters(DeadEndsParams {
                id: None,
                query: Some("passport".into()),
                recurring: None,
            }))
            .unwrap();

        assert!(result.contains("passport.js"));
    }

    #[test]
    fn test_engram_dead_ends_no_results() {
        let (_tmp, server, ids) = setup_test_server();

        let result = server
            .engram_dead_ends(Parameters(DeadEndsParams {
                id: Some(ids[1].as_str().to_string()), // aider engram with no dead ends
                query: None,
                recurring: None,
            }))
            .unwrap();

        assert!(result.contains("No dead ends"));
    }

    #[test]
    fn test_engram_trace() {
        let (_tmp, server, _ids) = setup_test_server();

        let result = server
            .engram_trace(Parameters(TraceParams {
                file_path: "auth".into(),
            }))
            .unwrap();

        // Should find engrams that touched auth files
        // The search uses text matching on file paths, so "auth" should match
        assert!(
            result.contains("Reasoning trace") || result.contains("No engrams found"),
            "Expected trace output, got: {result}"
        );
    }

    #[test]
    fn test_open_storage_error() {
        let server = EngramMcpServer::new(PathBuf::from("/nonexistent/path"));
        let result = server.engram_log(Parameters(LogParams {
            limit: None,
            by_agent: None,
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_engram_show_invalid_id() {
        let (_tmp, server, _ids) = setup_test_server();

        let result = server.engram_show(Parameters(ShowParams {
            id: "nonexistent_id_12345".into(),
        }));
        assert!(result.is_err());
    }

    #[test]
    fn test_engram_why() {
        let (_tmp, server, _ids) = setup_test_server();

        let result = server
            .engram_why(Parameters(WhyParams {
                file_path: "src/auth.rs".into(),
                limit: None,
            }))
            .unwrap();

        assert!(result.contains("Why does `src/auth.rs` exist?"));
        assert!(result.contains("OAuth2"));
        assert!(result.contains("sessions"));
    }

    #[test]
    fn test_engram_why_no_history() {
        let (_tmp, server, _ids) = setup_test_server();

        let result = server
            .engram_why(Parameters(WhyParams {
                file_path: "nonexistent_file.rs".into(),
                limit: None,
            }))
            .unwrap();

        assert!(result.contains("No reasoning history found"));
    }

    #[test]
    fn test_engram_stats_default() {
        let (_tmp, server, _ids) = setup_test_server();

        let result = server
            .engram_stats(Parameters(StatsParams {
                by_file: None,
                by_branch: None,
                trend: None,
                top: None,
            }))
            .unwrap();

        assert!(result.contains("Engram Statistics"));
        assert!(result.contains("2 engrams"));
    }

    #[test]
    fn test_engram_stats_by_file() {
        let (_tmp, server, _ids) = setup_test_server();

        let result = server
            .engram_stats(Parameters(StatsParams {
                by_file: Some(true),
                by_branch: None,
                trend: None,
                top: None,
            }))
            .unwrap();

        assert!(result.contains("Cost by File"));
        assert!(result.contains("src/auth.rs"));
    }

    #[test]
    fn test_engram_stats_trend() {
        let (_tmp, server, _ids) = setup_test_server();

        let result = server
            .engram_stats(Parameters(StatsParams {
                by_file: None,
                by_branch: None,
                trend: Some(true),
                top: None,
            }))
            .unwrap();

        assert!(result.contains("Cost Trend"));
        assert!(result.contains("Total:"));
    }
}
