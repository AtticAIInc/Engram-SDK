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

    /// Configure Claude Code SessionEnd hook for auto-capture
    #[arg(long)]
    pub claude_code: bool,
}

pub fn run(args: &InitArgs) -> Result<()> {
    let storage =
        GitStorage::discover().context("Not inside a Git repository. Run `git init` first.")?;

    if storage.is_initialized() && !args.force {
        // Even if already initialized, allow --claude-code to be added
        if args.claude_code {
            let workdir = storage
                .workdir()
                .ok_or_else(|| anyhow::anyhow!("Cannot determine working directory"))?;
            hooks::install_claude_code_hook(workdir)
                .context("Failed to configure Claude Code hook")?;
            println!("Claude Code SessionEnd hook configured.");
            println!("  Sessions will be auto-imported when Claude Code exits.");
            return Ok(());
        }
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

    // Install git alias for viewing engram notes
    let _ = storage
        .repo()
        .config()
        .and_then(|mut c| c.set_str("alias.loge", "log --notes=refs/notes/engram"));

    if args.claude_code {
        let workdir = storage
            .workdir()
            .ok_or_else(|| anyhow::anyhow!("Cannot determine working directory"))?;
        hooks::install_claude_code_hook(workdir).context("Failed to configure Claude Code hook")?;
        println!("Claude Code SessionEnd hook configured.");
        println!("  Sessions will be auto-imported when Claude Code exits.");
        println!();
    }

    println!("Engram initialized. Reasoning capture is ready.");
    println!();
    println!("Next steps:");
    println!("  engram record -- <agent-command>   Record an agent session");
    println!("  engram import --auto-detect        Import existing sessions");
    println!("  engram log                         List captured engrams");
    Ok(())
}
