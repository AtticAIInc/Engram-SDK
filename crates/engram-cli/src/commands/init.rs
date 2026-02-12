use anyhow::{Context, Result};
use clap::Args;
use engram_core::hooks;
use engram_core::storage::GitStorage;

#[derive(Args)]
pub struct InitArgs {
    /// Force re-initialization
    #[arg(long)]
    pub force: bool,

    /// Remote name to configure refspecs on (default: all remotes)
    #[arg(long)]
    pub remote: Option<String>,
}

pub fn run(args: &InitArgs) -> Result<()> {
    let storage =
        GitStorage::discover().context("Not inside a Git repository. Run `git init` first.")?;

    if storage.is_initialized() && !args.force {
        println!("Engram is already initialized in this repository.");
        println!("Use --force to re-initialize.");
        return Ok(());
    }

    storage
        .init_with_remote(args.remote.as_deref())
        .context("Failed to initialize engram")?;

    // Install git hooks for commit trailer injection
    let git_dir = storage.repo().path().to_path_buf();
    hooks::install_hooks(&git_dir).context("Failed to install git hooks")?;

    println!("Engram initialized. Reasoning capture is ready.");
    println!();
    println!("Next steps:");
    println!("  engram record -- <agent-command>   Record an agent session");
    println!("  engram import --auto-detect        Import existing sessions");
    println!("  engram log                         List captured engrams");
    Ok(())
}
