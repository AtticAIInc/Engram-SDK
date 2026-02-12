use anyhow::{Context, Result};
use clap::Args;

use engram_core::storage::GitStorage;
use engram_protocol::{push_engrams, SyncOptions};

#[derive(Args)]
pub struct PushArgs {
    /// Remote name (default: origin)
    #[arg(default_value = "origin")]
    pub remote: String,

    /// Dry run — show what would be pushed
    #[arg(long)]
    pub dry_run: bool,
}

pub fn run(args: &PushArgs) -> Result<()> {
    let storage = GitStorage::discover().context("Not in a Git repository with engram")?;
    let opts = SyncOptions {
        dry_run: args.dry_run,
        ..Default::default()
    };

    let result = push_engrams(storage.repo(), &args.remote, &opts)?;

    if args.dry_run {
        eprintln!(
            "Would push {} engram ref(s) to {}",
            result.refs_pushed, result.remote
        );
    } else {
        eprintln!(
            "Pushed {} engram ref(s) to {}",
            result.refs_pushed, result.remote
        );
    }

    Ok(())
}
