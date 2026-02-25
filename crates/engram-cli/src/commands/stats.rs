use std::collections::BTreeMap;

use anyhow::{Context, Result};
use clap::Args;
use engram_core::storage::{GitStorage, ListOptions};

use crate::output::OutputFormat;

#[derive(Args)]
pub struct StatsArgs {
    /// Show cost and token breakdown by file
    #[arg(long)]
    pub by_file: bool,

    /// Show cost and token breakdown by branch
    #[arg(long)]
    pub by_branch: bool,

    /// Show daily cost trend for the last 30 days
    #[arg(long)]
    pub trend: bool,

    /// Maximum number of entries in breakdowns
    #[arg(long, default_value = "10")]
    pub top: usize,
}

pub fn run(args: &StatsArgs, format: OutputFormat) -> Result<()> {
    let storage = GitStorage::discover().context("Not inside a Git repository")?;

    if !storage.is_initialized() {
        anyhow::bail!("Engram is not initialized. Run `engram init` first.");
    }

    let manifests = storage
        .list(&ListOptions::default())
        .context("Failed to list engrams")?;

    if manifests.is_empty() {
        println!("No engrams found.");
        return Ok(());
    }

    // If a specific breakdown was requested, run that and return
    if args.by_file {
        return run_by_file(&storage, &manifests, args.top, format);
    }
    if args.by_branch {
        return run_by_branch(&storage, &manifests, args.top, format);
    }
    if args.trend {
        return run_trend(&manifests, format);
    }

    // Default: show aggregate stats (original behavior)
    run_aggregate(&manifests, format)
}

fn run_aggregate(manifests: &[engram_core::model::Manifest], format: OutputFormat) -> Result<()> {
    let total = manifests.len();
    let mut total_tokens: u64 = 0;
    let mut total_cost: f64 = 0.0;
    let mut by_agent: BTreeMap<String, (usize, u64, f64)> = BTreeMap::new();
    let mut by_mode: BTreeMap<String, usize> = BTreeMap::new();

    let earliest = manifests.last().map(|m| m.created_at);
    let latest = manifests.first().map(|m| m.created_at);

    for m in manifests {
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

    match format {
        OutputFormat::Json => {
            let stats = serde_json::json!({
                "total_engrams": total,
                "total_tokens": total_tokens,
                "total_cost_usd": total_cost,
                "earliest": earliest,
                "latest": latest,
                "by_agent": by_agent.iter().map(|(name, (count, tokens, cost))| {
                    serde_json::json!({
                        "agent": name,
                        "count": count,
                        "tokens": tokens,
                        "cost_usd": cost,
                    })
                }).collect::<Vec<_>>(),
                "by_capture_mode": by_mode,
            });
            println!("{}", serde_json::to_string_pretty(&stats).unwrap());
        }
        OutputFormat::Text | OutputFormat::Markdown => {
            println!("Engram Statistics");
            println!("=================");
            println!("Total engrams:  {total}");
            println!("Total tokens:   {total_tokens}");
            println!("Total cost:     ${total_cost:.2}");
            if let (Some(e), Some(l)) = (earliest, latest) {
                println!(
                    "Date range:     {} to {}",
                    e.format("%Y-%m-%d"),
                    l.format("%Y-%m-%d")
                );
            }
            println!();

            println!("By Agent:");
            for (name, (count, tokens, cost)) in &by_agent {
                println!("  {name}: {count} engrams, {tokens} tokens, ${cost:.2}");
            }
            println!();

            println!("By Capture Mode:");
            for (mode, count) in &by_mode {
                println!("  {mode}: {count}");
            }
        }
    }

    Ok(())
}

fn run_by_file(
    storage: &GitStorage,
    manifests: &[engram_core::model::Manifest],
    top: usize,
    format: OutputFormat,
) -> Result<()> {
    // Accumulate (sessions, tokens, cost) per file
    let mut by_file: BTreeMap<String, (usize, u64, f64)> = BTreeMap::new();

    for m in manifests {
        if let Ok(data) = storage.read(m.id.as_str()) {
            let session_tokens = m.token_usage.total_tokens;
            let session_cost = m
                .token_usage
                .effective_cost(m.agent.model.as_deref())
                .unwrap_or(0.0);
            let file_count = data.operations.file_changes.len();

            if file_count == 0 {
                continue;
            }

            // Distribute session cost evenly across files touched
            let per_file_tokens = session_tokens / file_count as u64;
            let per_file_cost = session_cost / file_count as f64;

            for fc in &data.operations.file_changes {
                let entry = by_file.entry(fc.path.clone()).or_default();
                entry.0 += 1;
                entry.1 += per_file_tokens;
                entry.2 += per_file_cost;
            }
        }
    }

    // Sort by cost descending
    let mut sorted: Vec<_> = by_file.into_iter().collect();
    sorted.sort_by(|a, b| {
        b.1 .2
            .partial_cmp(&a.1 .2)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    sorted.truncate(top);

    match format {
        OutputFormat::Json => {
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
            println!("{}", serde_json::to_string_pretty(&entries).unwrap());
        }
        OutputFormat::Text | OutputFormat::Markdown => {
            println!("Cost by File (top {top})");
            println!("{}", "=".repeat(60));
            println!(
                "{:<40} {:>8} {:>10} {:>8}",
                "File", "Sessions", "Tokens", "Cost"
            );
            println!("{}", "-".repeat(60));
            for (path, (sessions, tokens, cost)) in &sorted {
                let display_path = if path.len() > 38 {
                    format!("...{}", &path[path.len() - 35..])
                } else {
                    path.clone()
                };
                println!(
                    "{:<40} {:>8} {:>10} ${:>7.2}",
                    display_path, sessions, tokens, cost
                );
            }
        }
    }

    Ok(())
}

fn run_by_branch(
    storage: &GitStorage,
    manifests: &[engram_core::model::Manifest],
    top: usize,
    format: OutputFormat,
) -> Result<()> {
    // Accumulate (sessions, tokens, cost) per branch
    let mut by_branch: BTreeMap<String, (usize, u64, f64)> = BTreeMap::new();

    for m in manifests {
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

    // Sort by cost descending
    let mut sorted: Vec<_> = by_branch.into_iter().collect();
    sorted.sort_by(|a, b| {
        b.1 .2
            .partial_cmp(&a.1 .2)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    sorted.truncate(top);

    match format {
        OutputFormat::Json => {
            let entries: Vec<_> = sorted
                .iter()
                .map(|(branch, (sessions, tokens, cost))| {
                    serde_json::json!({
                        "branch": branch,
                        "sessions": sessions,
                        "tokens": tokens,
                        "cost_usd": cost,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&entries).unwrap());
        }
        OutputFormat::Text | OutputFormat::Markdown => {
            println!("Cost by Branch (top {top})");
            println!("{}", "=".repeat(60));
            println!(
                "{:<30} {:>8} {:>12} {:>8}",
                "Branch", "Sessions", "Tokens", "Cost"
            );
            println!("{}", "-".repeat(60));
            for (branch, (sessions, tokens, cost)) in &sorted {
                let display = if branch.len() > 28 {
                    format!("...{}", &branch[branch.len() - 25..])
                } else {
                    branch.clone()
                };
                println!(
                    "{:<30} {:>8} {:>12} ${:>7.2}",
                    display, sessions, tokens, cost
                );
            }
        }
    }

    Ok(())
}

fn run_trend(manifests: &[engram_core::model::Manifest], format: OutputFormat) -> Result<()> {
    use chrono::{Duration, Utc};

    let cutoff = Utc::now() - Duration::days(30);
    let today = Utc::now().date_naive();

    // Bucket by date
    let mut by_date: BTreeMap<chrono::NaiveDate, (usize, u64, f64)> = BTreeMap::new();

    for m in manifests {
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

    match format {
        OutputFormat::Json => {
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
            println!("{}", serde_json::to_string_pretty(&entries).unwrap());
        }
        OutputFormat::Text | OutputFormat::Markdown => {
            println!("Cost Trend (last 30 days)");
            println!("{}", "=".repeat(50));
            println!(
                "{:<12} {:>8} {:>12} {:>8}",
                "Date", "Sessions", "Tokens", "Cost"
            );
            println!("{}", "-".repeat(50));

            // Fill in days with no activity
            let start =
                (today - Duration::days(29)).max(by_date.keys().next().copied().unwrap_or(today));
            let mut date = start;
            while date <= today {
                let (sessions, tokens, cost) = by_date.get(&date).copied().unwrap_or((0, 0, 0.0));
                if sessions > 0 {
                    println!("{:<12} {:>8} {:>12} ${:>7.2}", date, sessions, tokens, cost);
                }
                date += Duration::days(1);
            }

            // Summary
            let total_cost: f64 = by_date.values().map(|v| v.2).sum();
            let total_sessions: usize = by_date.values().map(|v| v.0).sum();
            let active_days = by_date.len();
            println!("{}", "-".repeat(50));
            println!(
                "Total: {total_sessions} sessions over {active_days} active days, ${total_cost:.2}"
            );
        }
    }

    Ok(())
}
