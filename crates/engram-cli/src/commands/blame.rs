use anyhow::{Context, Result};
use clap::Args;
use engram_core::storage::GitStorage;
use engram_query::search::SearchEngine;

use crate::output::OutputFormat;

#[derive(Args)]
pub struct BlameArgs {
    /// File path to find reasoning history for
    pub file: String,

    /// Maximum number of results
    #[arg(short = 'n', long, default_value = "20")]
    pub limit: usize,
}

pub fn run(args: &BlameArgs, format: OutputFormat) -> Result<()> {
    let storage = GitStorage::discover().context("Not inside a Git repository")?;

    if !storage.is_initialized() {
        anyhow::bail!("Engram is not initialized. Run `engram init` first.");
    }

    let search = SearchEngine::open(&storage).context("Failed to open search index")?;
    let results = search
        .search_by_file(&storage, &args.file, args.limit)
        .context("Search failed")?;

    if results.is_empty() {
        println!("No engrams found that touched '{}'.", args.file);
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            let entries: Vec<_> = results
                .iter()
                .map(|r| {
                    // Get actual change type
                    let change_info = storage.read(r.manifest.id.as_str()).ok().and_then(|data| {
                        data.operations
                            .file_changes
                            .iter()
                            .find(|fc| fc.path == args.file)
                            .map(|fc| {
                                serde_json::json!({
                                    "change_type": format!("{:?}", fc.change_type),
                                    "lines_added": fc.lines_added,
                                    "lines_removed": fc.lines_removed,
                                })
                            })
                    });

                    serde_json::json!({
                        "engram_id": r.manifest.id.as_str(),
                        "created_at": r.manifest.created_at,
                        "agent": r.manifest.agent.name,
                        "summary": r.manifest.summary,
                        "intent": r.manifest.summary,
                        "change": change_info,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&entries).unwrap());
        }
        OutputFormat::Text => {
            println!("Reasoning blame for: {}", args.file);
            println!("{}", "=".repeat(40 + args.file.len()));
            println!();

            for r in &results {
                let m = &r.manifest;
                let short_id = &m.id.as_str()[..8];
                let date = m.created_at.format("%Y-%m-%d %H:%M");
                let summary = m.summary.as_deref().unwrap_or("(no summary)");

                // Get change type from full data
                let change_type = storage
                    .read(m.id.as_str())
                    .ok()
                    .and_then(|data| {
                        data.operations
                            .file_changes
                            .iter()
                            .find(|fc| fc.path == args.file)
                            .map(|fc| format!("{:?}", fc.change_type).to_lowercase())
                    })
                    .unwrap_or_else(|| "touched".to_string());

                println!("{short_id} {date} [{change_type}] {}", m.agent.name);
                println!("  {summary}");

                // Show intent if we can read it
                if let Ok(data) = storage.read(m.id.as_str()) {
                    let intent = &data.intent.original_request;
                    if intent != summary {
                        println!("  Intent: \"{intent}\"");
                    }
                    if !data.intent.dead_ends.is_empty() {
                        let dead_ends: Vec<_> =
                            data.intent.dead_ends.iter().map(|d| &d.approach).collect();
                        println!(
                            "  Dead ends: {}",
                            dead_ends
                                .iter()
                                .map(|s| s.as_str())
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                    }
                }
                println!();
            }
        }
    }

    Ok(())
}
