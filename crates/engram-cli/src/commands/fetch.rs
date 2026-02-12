use anyhow::{Context, Result};
use clap::Args;

use engram_core::storage::GitStorage;
use engram_protocol::{fetch_engrams, SyncOptions};

#[derive(Args)]
pub struct FetchArgs {
    /// Remote name (default: origin)
    #[arg(default_value = "origin")]
    pub remote: String,

    /// Dry run — show what would be fetched
    #[arg(long)]
    pub dry_run: bool,
}

pub fn run(args: &FetchArgs) -> Result<()> {
    let storage = GitStorage::discover().context("Not in a Git repository with engram")?;
    let opts = SyncOptions {
        dry_run: args.dry_run,
        ..Default::default()
    };

    let result = fetch_engrams(storage.repo(), &args.remote, &opts)?;

    if args.dry_run {
        eprintln!("Would fetch engram refs from {}", result.remote);
    } else {
        eprintln!(
            "Fetched {} new engram ref(s) from {}",
            result.refs_fetched, result.remote
        );
    }

    Ok(())
}
