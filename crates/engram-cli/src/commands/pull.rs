use anyhow::{Context, Result};
use clap::Args;

use engram_core::storage::GitStorage;
use engram_protocol::{fetch_engrams, SyncOptions};
use engram_query::SearchEngine;

#[derive(Args)]
pub struct PullArgs {
    /// Remote name (default: origin)
    #[arg(default_value = "origin")]
    pub remote: String,
}

pub fn run(args: &PullArgs) -> Result<()> {
    let storage = GitStorage::discover().context("Not in a Git repository with engram")?;
    let opts = SyncOptions::default();

    let result = fetch_engrams(storage.repo(), &args.remote, &opts)?;

    eprintln!(
        "Fetched {} new engram ref(s) from {}",
        result.refs_fetched, result.remote
    );

    // Reindex if new refs were fetched
    if result.refs_fetched > 0 {
        let engine = SearchEngine::open(&storage)?;
        let count = engine.rebuild(&storage)?;
        eprintln!("Reindexed {count} engram(s).");
    }

    Ok(())
}
