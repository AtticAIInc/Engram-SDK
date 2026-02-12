use std::collections::BTreeSet;

use anyhow::{Context, Result};
use clap::Args;

use engram_core::model::FileChangeType;
use engram_core::storage::GitStorage;
use engram_query::review_branch;

use crate::output::OutputFormat;

#[derive(Args)]
pub struct PrSummaryArgs {
    /// Commit range (e.g. "main..feature" or "HEAD~5..HEAD")
    pub range: String,
}

pub fn run(args: &PrSummaryArgs, format: OutputFormat) -> Result<()> {
    let storage = GitStorage::discover().context("Not in a Git repository with engram")?;

    let parts: Vec<&str> = args.range.splitn(2, "..").collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid range format. Use 'base..head' (e.g. 'main..feature')");
    }
    let (base, head) = (parts[0], parts[1]);

    let review = review_branch(&storage, base, head)?;

    if review.engrams.is_empty() {
        println!("No engrams found in range {}", review.range);
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "range": review.range,
                "total_commits": review.total_commits,
                "total_tokens": review.total_tokens,
                "total_cost": review.total_cost,
                "engrams": review.engrams.iter().map(|e| {
                    serde_json::json!({
                        "id": e.manifest.id.as_str(),
                        "agent": e.manifest.agent.name,
                        "model": e.manifest.agent.model,
                        "summary": e.manifest.summary,
                        "commit": e.commit_sha,
                        "tokens": e.manifest.token_usage.total_tokens,
                        "cost": e.manifest.token_usage.cost_usd,
                    })
                }).collect::<Vec<_>>(),
                "files_changed": review.files_changed,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Text => {
            print_text(&storage, &review);
        }
        OutputFormat::Markdown => {
            print_markdown(&storage, &review);
        }
    }

    Ok(())
}

fn print_text(storage: &GitStorage, review: &engram_query::review::BranchReview) {
    println!("PR Summary: {}\n", review.range);

    // Summary from intents
    println!("Summary:");
    for entry in &review.engrams {
        let summary = entry.manifest.summary.as_deref().unwrap_or("(no summary)");
        println!("  - {summary}");
    }
    println!();

    // Files
    if !review.files_changed.is_empty() {
        println!("Files changed ({}):", review.files_changed.len());
        let mut sorted: Vec<_> = review.files_changed.iter().collect();
        sorted.sort();
        for f in sorted {
            println!("  {f}");
        }
        println!();
    }

    // Dead ends
    let mut dead_ends = Vec::new();
    for entry in &review.engrams {
        if let Ok(data) = storage.read(entry.manifest.id.as_str()) {
            for de in &data.intent.dead_ends {
                dead_ends.push(format!("{} — {}", de.approach, de.reason));
            }
        }
    }
    if !dead_ends.is_empty() {
        println!("Dead ends:");
        for de in &dead_ends {
            println!("  - {de}");
        }
        println!();
    }

    // Economics
    println!("Tokens: {}", review.total_tokens);
    if let Some(cost) = review.total_cost {
        println!("Cost:   ${cost:.2}");
    }
    println!("Commits: {}", review.total_commits);
}

fn print_markdown(storage: &GitStorage, review: &engram_query::review::BranchReview) {
    // Summary
    println!("## Summary\n");
    for entry in &review.engrams {
        if let Some(summary) = &entry.manifest.summary {
            println!("- {summary}");
        }
    }
    println!();

    // Changes — collect file change types from full data
    if !review.files_changed.is_empty() {
        println!("## Changes\n");
        let mut file_types: Vec<(String, String)> = Vec::new();
        let mut seen: BTreeSet<String> = BTreeSet::new();

        for entry in &review.engrams {
            if let Ok(data) = storage.read(entry.manifest.id.as_str()) {
                for fc in &data.operations.file_changes {
                    if seen.insert(fc.path.clone()) {
                        let change_label = match &fc.change_type {
                            FileChangeType::Created => "Created",
                            FileChangeType::Modified => "Modified",
                            FileChangeType::Deleted => "Deleted",
                            FileChangeType::Renamed { from } => {
                                file_types
                                    .push((fc.path.clone(), format!("Renamed from `{from}`")));
                                continue;
                            }
                        };
                        file_types.push((fc.path.clone(), change_label.to_string()));
                    }
                }
            }
        }

        file_types.sort_by(|a, b| a.0.cmp(&b.0));
        for (path, change) in &file_types {
            println!("- `{path}` — {change}");
        }
        println!();
    }

    // Reasoning chain
    println!("## Reasoning\n");
    for entry in &review.engrams {
        let m = &entry.manifest;
        let short_id = &entry.commit_sha[..8.min(entry.commit_sha.len())];
        let agent = &m.agent.name;
        let model = m.agent.model.as_deref().unwrap_or("unknown");
        let summary = m.summary.as_deref().unwrap_or("(no summary)");
        println!("- **{short_id}** ({agent}/{model}): {summary}");
    }
    println!();

    // Dead ends
    let mut dead_ends = Vec::new();
    for entry in &review.engrams {
        if let Ok(data) = storage.read(entry.manifest.id.as_str()) {
            for de in &data.intent.dead_ends {
                dead_ends.push(format!("{} — {}", de.approach, de.reason));
            }
        }
    }
    if !dead_ends.is_empty() {
        println!("## Dead Ends\n");
        for de in &dead_ends {
            println!("- {de}");
        }
        println!();
    }

    // Economics
    println!("## Economics\n");
    println!("- **Tokens:** {} total", review.total_tokens);
    if let Some(cost) = review.total_cost {
        println!("- **Cost:** ${cost:.2}");
    }
    println!("- **Commits:** {}", review.total_commits);
    println!();

    println!("\u{1f916} Generated with [Engram](https://github.com/AtticAIInc/Engram-SDK)");
}
