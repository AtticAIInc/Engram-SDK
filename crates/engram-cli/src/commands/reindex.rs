use anyhow::{Context, Result};

use engram_core::storage::GitStorage;
use engram_query::SearchEngine;

pub fn run() -> Result<()> {
    let storage = GitStorage::discover().context("Not in a Git repository with engram")?;
    let engine = SearchEngine::open(&storage)?;

    eprintln!("Rebuilding search index...");
    let count = engine.rebuild(&storage)?;
    eprintln!("Indexed {count} engram(s).");

    Ok(())
}
