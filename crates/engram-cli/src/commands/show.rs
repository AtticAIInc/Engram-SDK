use anyhow::{Context, Result};
use clap::Args;
use engram_core::storage::GitStorage;

use crate::output::format::{format_engram_full, format_intent};
use crate::output::OutputFormat;

#[derive(Args)]
pub struct ShowArgs {
    /// Engram ID (full or prefix)
    pub id: String,

    /// Show only the intent
    #[arg(long)]
    pub intent: bool,

    /// Show only the transcript (as JSONL)
    #[arg(long)]
    pub transcript: bool,

    /// Show only operations
    #[arg(long)]
    pub operations: bool,
}

pub fn run(args: &ShowArgs, format: OutputFormat) -> Result<()> {
    let storage = GitStorage::discover().context("Not inside a Git repository")?;

    if !storage.is_initialized() {
        anyhow::bail!("Engram is not initialized. Run `engram init` first.");
    }

    let resolved_id = storage
        .resolve(&args.id)
        .with_context(|| format!("Failed to resolve engram '{}'", args.id))?;

    let data = storage
        .read(&resolved_id)
        .with_context(|| format!("Failed to read engram '{}'", resolved_id))?;

    let output = if args.intent {
        format_intent(&data, format)
    } else if args.transcript {
        match format {
            OutputFormat::Json => {
                serde_json::to_string_pretty(&data.transcript.entries).unwrap_or_default()
            }
            OutputFormat::Text | OutputFormat::Markdown => {
                let jsonl = data.transcript.to_jsonl().unwrap_or_default();
                String::from_utf8_lossy(&jsonl).to_string()
            }
        }
    } else if args.operations {
        serde_json::to_string_pretty(&data.operations).unwrap_or_default()
    } else {
        format_engram_full(&data, format)
    };

    println!("{output}");
    Ok(())
}
