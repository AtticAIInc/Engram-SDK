use anyhow::{Context, Result};
use clap::Args;

use engram_core::storage::GitStorage;
use engram_query::diff_engrams;

use crate::output::OutputFormat;

#[derive(Args)]
pub struct DiffArgs {
    /// First engram ID (or prefix)
    pub id_a: String,

    /// Second engram ID (or prefix)
    pub id_b: String,
}

pub fn run(args: &DiffArgs, format: OutputFormat) -> Result<()> {
    let storage = GitStorage::discover().context("Not in a Git repository with engram")?;

    // Resolve IDs through storage to get full EngramIds
    let data_a = storage
        .read(&args.id_a)
        .context("Failed to find first engram")?;
    let data_b = storage
        .read(&args.id_b)
        .context("Failed to find second engram")?;

    let diff = diff_engrams(&storage, &data_a.manifest.id, &data_b.manifest.id)?;

    match format {
        OutputFormat::Json => {
            let json = serde_json::json!({
                "id_a": diff.id_a.as_str(),
                "id_b": diff.id_b.as_str(),
                "common_files": diff.common_files,
                "only_a_files": diff.only_a_files,
                "only_b_files": diff.only_b_files,
                "token_delta": diff.token_delta,
                "cost_delta": diff.cost_delta,
            });
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        OutputFormat::Text | OutputFormat::Markdown => {
            let short_a = &diff.id_a.as_str()[..8];
            let short_b = &diff.id_b.as_str()[..8];
            println!("Comparing {short_a} vs {short_b}\n");

            if !diff.common_files.is_empty() {
                println!("Common files ({}):", diff.common_files.len());
                for f in &diff.common_files {
                    println!("  {f}");
                }
            }
            if !diff.only_a_files.is_empty() {
                println!("Only in {short_a} ({}):", diff.only_a_files.len());
                for f in &diff.only_a_files {
                    println!("  {f}");
                }
            }
            if !diff.only_b_files.is_empty() {
                println!("Only in {short_b} ({}):", diff.only_b_files.len());
                for f in &diff.only_b_files {
                    println!("  {f}");
                }
            }

            println!();
            println!("Token delta: {:+}", diff.token_delta);
            if let Some(cost) = diff.cost_delta {
                println!("Cost delta:  {:+.4}", cost);
            }
        }
    }

    Ok(())
}
