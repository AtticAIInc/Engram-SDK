use anyhow::{Context, Result};
use clap::Args;

use engram_core::storage::GitStorage;
use engram_query::review_branch;

use crate::output::OutputFormat;

#[derive(Args)]
pub struct ReviewArgs {
    /// Commit range (e.g. "main..feature" or "HEAD~5..HEAD")
    pub range: String,
}

pub fn run(args: &ReviewArgs, format: OutputFormat) -> Result<()> {
    let storage = GitStorage::discover().context("Not in a Git repository with engram")?;

    // Parse range
    let parts: Vec<&str> = args.range.splitn(2, "..").collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid range format. Use 'base..head' (e.g. 'main..feature')");
    }
    let (base, head) = (parts[0], parts[1]);

    let review = review_branch(&storage, base, head)?;

    match format {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "range": review.range,
                "total_commits": review.total_commits,
                "engrams": review.engrams.iter().map(|e| {
                    serde_json::json!({
                        "id": e.manifest.id.as_str(),
                        "summary": e.manifest.summary,
                        "agent": e.manifest.agent.name,
                        "tokens": e.manifest.token_usage.total_tokens,
                        "cost": e.manifest.token_usage.cost_usd,
                        "commit": e.commit_sha,
                    })
                }).collect::<Vec<_>>(),
                "total_tokens": review.total_tokens,
                "total_cost": review.total_cost,
                "files_changed": review.files_changed,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Text | OutputFormat::Markdown => {
            println!("Branch review: {}\n", review.range);
            println!(
                "  Commits: {}   Engrams: {}",
                review.total_commits,
                review.engrams.len()
            );
            println!("  Total tokens: {}", review.total_tokens);
            if let Some(cost) = review.total_cost {
                println!("  Total cost: ${cost:.4}");
            }
            if !review.files_changed.is_empty() {
                println!("  Files changed: {}", review.files_changed.len());
            }

            if !review.engrams.is_empty() {
                println!("\nEngrams:");
                for entry in &review.engrams {
                    let m = &entry.manifest;
                    let short_id = &m.id.as_str()[..8];
                    let summary = m.summary.as_deref().unwrap_or("(no summary)");
                    let commit_short = &entry.commit_sha[..8];
                    println!("  {short_id}  [{commit_short}]  {summary}");
                }
            }
        }
    }

    Ok(())
}
