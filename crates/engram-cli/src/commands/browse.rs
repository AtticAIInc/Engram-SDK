use anyhow::{Context, Result};
use clap::Args;
use engram_core::storage::GitStorage;

#[derive(Args)]
pub struct BrowseArgs {}

pub fn run(_args: &BrowseArgs) -> Result<()> {
    let storage = GitStorage::discover().context("Not inside a Git repository")?;

    if !storage.is_initialized() {
        anyhow::bail!("Engram is not initialized. Run `engram init` first.");
    }

    let repo_path = storage
        .workdir()
        .context("Cannot determine working directory")?
        .to_path_buf();

    engram_tui::run(&repo_path).context("TUI error")
}
