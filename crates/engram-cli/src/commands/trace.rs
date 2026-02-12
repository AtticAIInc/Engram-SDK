use anyhow::{Context, Result};
use clap::Args;

use engram_core::storage::GitStorage;
use engram_query::{trace_file, SearchEngine};

use crate::output::OutputFormat;

#[derive(Args)]
pub struct TraceArgs {
    /// File path to trace reasoning history for
    pub file: String,
}

pub fn run(args: &TraceArgs, format: OutputFormat) -> Result<()> {
    let storage = GitStorage::discover().context("Not in a Git repository with engram")?;
    let engine = SearchEngine::open(&storage)?;

    let entries = trace_file(&storage, &engine, &args.file)?;

    if entries.is_empty() {
        eprintln!("No engrams found that touched: {}", args.file);
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            let manifests: Vec<_> = entries.iter().map(|e| &e.manifest).collect();
            println!("{}", serde_json::to_string_pretty(&manifests)?);
        }
        OutputFormat::Text => {
            eprintln!(
                "Reasoning trace for: {} ({} engram(s))\n",
                args.file,
                entries.len()
            );
            for entry in &entries {
                let m = &entry.manifest;
                let short_id = &m.id.as_str()[..8];
                let ts = m.created_at.format("%Y-%m-%d %H:%M");
                let summary = m.summary.as_deref().unwrap_or("(no summary)");
                let agent = &m.agent.name;
                println!("{short_id}  {ts}  [{agent}]  {summary}");
            }
        }
    }

    Ok(())
}
