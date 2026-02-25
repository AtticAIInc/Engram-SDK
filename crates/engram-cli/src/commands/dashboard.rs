use anyhow::{Context, Result};
use clap::Args;
use engram_core::storage::GitStorage;

#[derive(Args)]
pub struct DashboardArgs {
    /// Start the dashboard web server
    #[arg(long)]
    pub serve: bool,

    /// Port to listen on
    #[arg(long, default_value = "3000")]
    pub port: u16,

    /// Open browser automatically
    #[arg(long)]
    pub open: bool,
}

pub fn run(args: &DashboardArgs) -> Result<()> {
    if !args.serve {
        println!("Usage: engram dashboard --serve [--port 3000] [--open]");
        println!();
        println!("Starts a local web dashboard showing engram data, cost breakdowns, and trends.");
        return Ok(());
    }

    let storage = GitStorage::discover().context("Not inside a Git repository")?;

    if !storage.is_initialized() {
        anyhow::bail!("Engram is not initialized. Run `engram init` first.");
    }

    let repo_path = storage
        .workdir()
        .context("Cannot determine working directory")?
        .to_path_buf();

    let port = args.port;
    let open_browser = args.open;

    let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
    rt.block_on(async {
        println!("Starting engram dashboard at http://localhost:{port}");

        if open_browser {
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("open")
                    .arg(format!("http://localhost:{port}"))
                    .spawn();
            }
            #[cfg(target_os = "linux")]
            {
                let _ = std::process::Command::new("xdg-open")
                    .arg(format!("http://localhost:{port}"))
                    .spawn();
            }
        }

        engram_dashboard::serve(&repo_path, port)
            .await
            .map_err(|e| anyhow::anyhow!("Server error: {e}"))
    })
}
