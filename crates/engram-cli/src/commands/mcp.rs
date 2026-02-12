use anyhow::{Context, Result};

use engram_core::storage::GitStorage;

pub fn run() -> Result<()> {
    let storage = GitStorage::discover().context("Not inside a Git repository")?;

    if !storage.is_initialized() {
        anyhow::bail!("Engram is not initialized. Run `engram init` first.");
    }

    let repo_path = storage
        .repo()
        .path()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| storage.repo().path().to_path_buf());

    let rt = tokio::runtime::Runtime::new().context("Failed to create async runtime")?;
    rt.block_on(async {
        engram_mcp::run_stdio(repo_path)
            .await
            .map_err(|e| anyhow::anyhow!("MCP server error: {e}"))
    })
}
