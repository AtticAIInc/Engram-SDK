use std::collections::BTreeMap;

use anyhow::{Context, Result};
use engram_core::storage::{GitStorage, ListOptions};

use crate::output::OutputFormat;

pub fn run(format: OutputFormat) -> Result<()> {
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

    let total = manifests.len();
    let mut total_tokens: u64 = 0;
    let mut total_cost: f64 = 0.0;
    let mut by_agent: BTreeMap<String, (usize, u64, f64)> = BTreeMap::new();
    let mut by_mode: BTreeMap<String, usize> = BTreeMap::new();

    let earliest = manifests.last().map(|m| m.created_at);
    let latest = manifests.first().map(|m| m.created_at);

    for m in &manifests {
        total_tokens += m.token_usage.total_tokens;
        total_cost += m.token_usage.cost_usd.unwrap_or(0.0);

        let entry = by_agent.entry(m.agent.name.clone()).or_default();
        entry.0 += 1;
        entry.1 += m.token_usage.total_tokens;
        entry.2 += m.token_usage.cost_usd.unwrap_or(0.0);

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
