use anyhow::{Context, Result};
use clap::Args;

use engram_core::storage::GitStorage;
use engram_query::SearchEngine;

use crate::output::OutputFormat;

#[derive(Args)]
pub struct SearchArgs {
    /// Search query (free-text, searches intent, transcript, file paths, dead ends)
    pub query: String,

    /// Maximum number of results
    #[arg(short = 'n', long, default_value = "10")]
    pub limit: usize,
}

pub fn run(args: &SearchArgs, format: OutputFormat) -> Result<()> {
    let storage = GitStorage::discover().context("Not in a Git repository with engram")?;
    let engine = SearchEngine::open(&storage)?;

    let results = engine.search(&storage, &args.query, args.limit)?;

    if results.is_empty() {
        eprintln!("No results found for: {}", args.query);
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            let manifests: Vec<_> = results.iter().map(|r| &r.manifest).collect();
            println!("{}", serde_json::to_string_pretty(&manifests)?);
        }
        OutputFormat::Text | OutputFormat::Markdown => {
            eprintln!("Found {} result(s) for: {}\n", results.len(), args.query);
            for result in &results {
                let m = &result.manifest;
                let short_id = &m.id.as_str()[..8];
                let summary = m.summary.as_deref().unwrap_or("(no summary)");
                let score = result.score;
                println!("{short_id}  {summary}  (score: {score:.2})");
            }
        }
    }

    Ok(())
}
