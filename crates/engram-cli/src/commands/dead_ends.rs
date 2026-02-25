use std::collections::BTreeMap;

use anyhow::{Context, Result};
use clap::Args;
use engram_core::storage::{GitStorage, ListOptions};

use crate::output::OutputFormat;

#[derive(Args)]
pub struct DeadEndsArgs {
    /// Only show dead ends that recur across multiple sessions
    #[arg(long)]
    pub recurring: bool,

    /// Filter dead ends by text match
    #[arg(long)]
    pub query: Option<String>,

    /// Show dead ends from a specific engram
    #[arg(long)]
    pub id: Option<String>,

    /// Maximum number of results
    #[arg(long, default_value = "10")]
    pub top: usize,
}

pub fn run(args: &DeadEndsArgs, format: OutputFormat) -> Result<()> {
    let storage = GitStorage::discover().context("Not inside a Git repository")?;

    if !storage.is_initialized() {
        anyhow::bail!("Engram is not initialized. Run `engram init` first.");
    }

    // Show dead ends from a specific engram
    if let Some(id) = &args.id {
        return run_single(&storage, id, format);
    }

    if args.recurring {
        return run_recurring(&storage, args, format);
    }

    // Default: list all dead ends (optionally filtered)
    run_all(&storage, args, format)
}

fn run_single(storage: &GitStorage, id: &str, format: OutputFormat) -> Result<()> {
    let resolved = storage
        .resolve(id)
        .context(format!("Failed to resolve '{id}'"))?;
    let data = storage.read(&resolved).context("Failed to read engram")?;

    if data.intent.dead_ends.is_empty() && data.intent.decisions.is_empty() {
        println!(
            "No dead ends or decisions for engram {}.",
            &resolved[..8.min(resolved.len())]
        );
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            let output = serde_json::json!({
                "engram_id": resolved,
                "dead_ends": data.intent.dead_ends.iter().map(|de| serde_json::json!({
                    "approach": de.approach,
                    "reason": de.reason,
                })).collect::<Vec<_>>(),
                "decisions": data.intent.decisions.iter().map(|d| serde_json::json!({
                    "description": d.description,
                    "rationale": d.rationale,
                })).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Text | OutputFormat::Markdown => {
            let short_id = &resolved[..8.min(resolved.len())];
            if !data.intent.dead_ends.is_empty() {
                println!("Dead Ends ({short_id}):");
                for de in &data.intent.dead_ends {
                    println!("  - {}: {}", de.approach, de.reason);
                }
            }
            if !data.intent.decisions.is_empty() {
                println!("Decisions ({short_id}):");
                for d in &data.intent.decisions {
                    println!("  - {}: {}", d.description, d.rationale);
                }
            }
        }
    }

    Ok(())
}

/// Occurrence of a dead end in a specific engram.
struct DeadEndOccurrence {
    engram_id: String,
    date: chrono::DateTime<chrono::Utc>,
    agent: String,
    reason: String,
    summary: Option<String>,
}

fn run_recurring(storage: &GitStorage, args: &DeadEndsArgs, format: OutputFormat) -> Result<()> {
    let manifests = storage
        .list(&ListOptions::default())
        .context("Failed to list engrams")?;

    let query_lower = args.query.as_deref().unwrap_or("").to_lowercase();

    // Group dead ends by normalized approach name
    let mut groups: BTreeMap<String, Vec<DeadEndOccurrence>> = BTreeMap::new();

    for m in &manifests {
        if let Ok(data) = storage.read(m.id.as_str()) {
            for de in &data.intent.dead_ends {
                // Filter by query
                if !query_lower.is_empty()
                    && !de.approach.to_lowercase().contains(&query_lower)
                    && !de.reason.to_lowercase().contains(&query_lower)
                {
                    continue;
                }

                let normalized = de.approach.trim().to_lowercase();
                groups
                    .entry(normalized)
                    .or_default()
                    .push(DeadEndOccurrence {
                        engram_id: m.id.as_str()[..8.min(m.id.as_str().len())].to_string(),
                        date: m.created_at,
                        agent: m.agent.name.clone(),
                        reason: de.reason.clone(),
                        summary: m.summary.clone(),
                    });
            }
        }
    }

    // Filter to recurring (2+), sort by frequency
    let mut recurring: Vec<_> = groups
        .into_iter()
        .filter(|(_, occurrences)| occurrences.len() >= 2)
        .collect();
    recurring.sort_by(|a, b| b.1.len().cmp(&a.1.len()));
    recurring.truncate(args.top);

    if recurring.is_empty() {
        if query_lower.is_empty() {
            println!("No recurring dead ends found.");
        } else {
            println!(
                "No recurring dead ends matching '{}' found.",
                args.query.as_deref().unwrap_or("")
            );
        }
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            let entries: Vec<_> = recurring
                .iter()
                .map(|(approach, occurrences)| {
                    serde_json::json!({
                        "approach": approach,
                        "count": occurrences.len(),
                        "occurrences": occurrences.iter().map(|o| serde_json::json!({
                            "engram_id": o.engram_id,
                            "date": o.date.to_rfc3339(),
                            "agent": o.agent,
                            "reason": o.reason,
                            "summary": o.summary,
                        })).collect::<Vec<_>>(),
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&entries).unwrap());
        }
        OutputFormat::Text | OutputFormat::Markdown => {
            println!("Recurring Dead Ends");
            println!("{}", "=".repeat(50));
            println!();

            for (approach, occurrences) in &recurring {
                println!(
                    "{} (tried {} times, rejected every time):",
                    approach,
                    occurrences.len()
                );
                for o in occurrences {
                    let date = o.date.format("%Y-%m-%d");
                    println!("  [{date}] {}: {} — {}", o.engram_id, o.agent, o.reason);
                    if let Some(summary) = &o.summary {
                        println!("    Context: {summary}");
                    }
                }
                println!();
            }
        }
    }

    Ok(())
}

fn run_all(storage: &GitStorage, args: &DeadEndsArgs, format: OutputFormat) -> Result<()> {
    let manifests = storage
        .list(&ListOptions::default())
        .context("Failed to list engrams")?;

    let query_lower = args.query.as_deref().unwrap_or("").to_lowercase();

    struct DeadEndEntry {
        engram_id: String,
        date: chrono::DateTime<chrono::Utc>,
        agent: String,
        approach: String,
        reason: String,
        summary: Option<String>,
    }

    let mut all_dead_ends: Vec<DeadEndEntry> = Vec::new();

    for m in &manifests {
        if let Ok(data) = storage.read(m.id.as_str()) {
            for de in &data.intent.dead_ends {
                if !query_lower.is_empty()
                    && !de.approach.to_lowercase().contains(&query_lower)
                    && !de.reason.to_lowercase().contains(&query_lower)
                {
                    continue;
                }

                all_dead_ends.push(DeadEndEntry {
                    engram_id: m.id.as_str()[..8.min(m.id.as_str().len())].to_string(),
                    date: m.created_at,
                    agent: m.agent.name.clone(),
                    approach: de.approach.clone(),
                    reason: de.reason.clone(),
                    summary: m.summary.clone(),
                });
            }
        }
    }

    all_dead_ends.truncate(args.top);

    if all_dead_ends.is_empty() {
        if query_lower.is_empty() {
            println!("No dead ends found.");
        } else {
            println!(
                "No dead ends matching '{}' found.",
                args.query.as_deref().unwrap_or("")
            );
        }
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            let entries: Vec<_> = all_dead_ends
                .iter()
                .map(|e| {
                    serde_json::json!({
                        "engram_id": e.engram_id,
                        "date": e.date.to_rfc3339(),
                        "agent": e.agent,
                        "approach": e.approach,
                        "reason": e.reason,
                        "summary": e.summary,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&entries).unwrap());
        }
        OutputFormat::Text | OutputFormat::Markdown => {
            println!("Dead Ends ({} found)", all_dead_ends.len());
            println!("{}", "=".repeat(40));
            println!();

            for e in &all_dead_ends {
                let date = e.date.format("%Y-%m-%d");
                println!("[{date}] {} ({})", e.approach, e.engram_id);
                println!("  Reason: {}", e.reason);
                println!("  Agent: {}", e.agent);
                if let Some(summary) = &e.summary {
                    println!("  Context: {summary}");
                }
                println!();
            }
        }
    }

    Ok(())
}
