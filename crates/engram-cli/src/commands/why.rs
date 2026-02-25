use anyhow::{Context, Result};
use clap::Args;
use engram_core::model::FileChangeType;
use engram_core::storage::GitStorage;
use engram_query::search::SearchEngine;

use crate::output::OutputFormat;

#[derive(Args)]
pub struct WhyArgs {
    /// File path to explain
    pub file: String,

    /// Maximum number of engrams to include
    #[arg(short = 'n', long, default_value = "20")]
    pub limit: usize,
}

pub fn run(args: &WhyArgs, format: OutputFormat) -> Result<()> {
    let storage = GitStorage::discover().context("Not inside a Git repository")?;

    if !storage.is_initialized() {
        anyhow::bail!("Engram is not initialized. Run `engram init` first.");
    }

    let search = SearchEngine::open(&storage).context("Failed to open search index")?;
    let results = search
        .search_by_file(&storage, &args.file, args.limit)
        .context("Search failed")?;

    if results.is_empty() {
        println!("No reasoning history found for '{}'.", args.file);
        return Ok(());
    }

    // Collect full data for each engram, sorted chronologically (oldest first)
    let mut entries: Vec<engram_core::model::EngramData> = Vec::new();
    for r in &results {
        if let Ok(data) = storage.read(r.manifest.id.as_str()) {
            entries.push(data);
        }
    }
    entries.sort_by_key(|e| e.manifest.created_at);

    match format {
        OutputFormat::Json => print_json(&args.file, &entries),
        OutputFormat::Text | OutputFormat::Markdown => print_narrative(&args.file, &entries),
    }

    Ok(())
}

fn print_narrative(file: &str, entries: &[engram_core::model::EngramData]) {
    println!("Why does `{}` exist?", file);
    println!("{}", "=".repeat(20 + file.len()));
    println!();

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

        // Find the file change for this specific file
        let file_change = data
            .operations
            .file_changes
            .iter()
            .find(|fc| fc.path == file);

        let change_desc = file_change
            .map(|fc| {
                let action = match &fc.change_type {
                    FileChangeType::Created => "Created",
                    FileChangeType::Modified => "Modified",
                    FileChangeType::Deleted => "Deleted",
                    FileChangeType::Renamed { from } => {
                        return format!("Renamed from {from}");
                    }
                };
                let lines = match (fc.lines_added, fc.lines_removed) {
                    (Some(a), Some(r)) => format!(" (+{a}/-{r} lines)"),
                    (Some(a), None) => format!(" (+{a} lines)"),
                    (None, Some(r)) => format!(" (-{r} lines)"),
                    (None, None) => String::new(),
                };
                format!("{action}{lines}")
            })
            .unwrap_or_else(|| "Touched".to_string());

        // Chapter header
        println!(
            "{}. [{}] {} ({}) — {}",
            i + 1,
            date,
            agent,
            model,
            change_desc
        );

        // Request
        println!("   Request: \"{}\"", data.intent.original_request);

        // Goal (if different from request)
        if let Some(goal) = &data.intent.interpreted_goal {
            if goal != &data.intent.original_request {
                println!("   Goal: {goal}");
            }
        }

        // What happened
        if let Some(summary) = data.intent.summary.as_deref().or(m.summary.as_deref()) {
            println!("   Result: {summary}");
        }

        // Dead ends
        if !data.intent.dead_ends.is_empty() {
            println!("   Dead ends:");
            for de in &data.intent.dead_ends {
                println!("     - {}: {}", de.approach, de.reason);
            }
        }

        // Decisions
        if !data.intent.decisions.is_empty() {
            println!("   Decisions:");
            for d in &data.intent.decisions {
                println!("     - {}: {}", d.description, d.rationale);
            }
        }

        println!();
    }

    // Footer
    println!("{}", "-".repeat(50));
    println!(
        "{} sessions touched this file | {} tokens | ${:.2} total cost",
        entries.len(),
        total_tokens,
        total_cost
    );
}

fn print_json(file: &str, entries: &[engram_core::model::EngramData]) {
    let json_entries: Vec<_> = entries
        .iter()
        .map(|data| {
            let m = &data.manifest;
            let file_change = data
                .operations
                .file_changes
                .iter()
                .find(|fc| fc.path == file);

            serde_json::json!({
                "engram_id": m.id.as_str(),
                "date": m.created_at.to_rfc3339(),
                "agent": m.agent.name,
                "model": m.agent.model,
                "request": data.intent.original_request,
                "goal": data.intent.interpreted_goal,
                "summary": data.intent.summary.as_deref().or(m.summary.as_deref()),
                "change": file_change.map(|fc| serde_json::json!({
                    "type": format!("{:?}", fc.change_type).to_lowercase(),
                    "lines_added": fc.lines_added,
                    "lines_removed": fc.lines_removed,
                })),
                "dead_ends": data.intent.dead_ends.iter().map(|de| serde_json::json!({
                    "approach": de.approach,
                    "reason": de.reason,
                })).collect::<Vec<_>>(),
                "decisions": data.intent.decisions.iter().map(|d| serde_json::json!({
                    "description": d.description,
                    "rationale": d.rationale,
                })).collect::<Vec<_>>(),
                "tokens": m.token_usage.total_tokens,
                "cost_usd": m.token_usage.effective_cost(m.agent.model.as_deref()),
            })
        })
        .collect();

    let output = serde_json::json!({
        "file": file,
        "sessions": json_entries.len(),
        "entries": json_entries,
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());
}
