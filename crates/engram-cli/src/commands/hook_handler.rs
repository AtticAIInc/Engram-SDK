use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use engram_core::hooks;
use engram_core::storage::GitStorage;

#[derive(Args)]
pub struct HookHandlerArgs {
    /// The hook name (prepare-commit-msg, post-commit)
    pub hook_name: String,

    /// Extra arguments passed by git to the hook
    #[arg(trailing_var_arg = true)]
    pub args: Vec<String>,
}

pub fn run(args: &HookHandlerArgs) -> Result<()> {
    // Find the git dir by discovering the repo
    let storage = GitStorage::discover().context("Not inside a Git repository")?;
    let git_dir = storage.repo().path().to_path_buf();

    match args.hook_name.as_str() {
        "prepare-commit-msg" => {
            let msg_file = args
                .args
                .first()
                .map(PathBuf::from)
                .context("prepare-commit-msg: missing message file argument")?;
            hooks::handle_prepare_commit_msg(&msg_file, &git_dir)?;
        }
        "post-commit" => {
            hooks::handle_post_commit(&git_dir)?;
        }
        other => {
            tracing::debug!("Unknown hook: {other}, ignoring");
        }
    }

    Ok(())
}
