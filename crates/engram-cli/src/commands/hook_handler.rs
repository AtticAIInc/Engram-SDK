use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use engram_capture::import::claude_code::ClaudeCodeImporter;
use engram_core::config::EngramConfig;
use engram_core::hooks;
use engram_core::hooks::ActiveSession;
use engram_core::model::AgentInfo;
use engram_core::storage::GitStorage;
use engram_protocol::{push_engrams, SyncOptions};
use engram_query::search::SearchEngine;

#[derive(Args)]
pub struct HookHandlerArgs {
    /// The hook name (prepare-commit-msg, post-commit, pre-push)
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
            // Auto-capture: if enabled and no active session, import current Claude Code session
            maybe_auto_capture(&storage, &git_dir);

            let msg_file = args
                .args
                .first()
                .map(PathBuf::from)
                .context("prepare-commit-msg: missing message file argument")?;
            hooks::handle_prepare_commit_msg(&msg_file, &git_dir)?;
        }
        "post-commit" => {
            hooks::handle_post_commit(&git_dir)?;

            // Auto-capture cleanup: if session was auto-created, clean it up
            maybe_auto_capture_cleanup(&storage, &git_dir);
        }
        "pre-push" => {
            maybe_auto_push(&storage);
        }
        other => {
            tracing::debug!("Unknown hook: {other}, ignoring");
        }
    }

    Ok(())
}

/// If auto_capture is enabled and no active session exists, discover and import
/// the most recent Claude Code session, then create a temporary ActiveSession
/// so the prepare-commit-msg handler can inject trailers.
fn maybe_auto_capture(storage: &GitStorage, git_dir: &std::path::Path) {
    // Don't interfere with an existing session (e.g. from `engram record`)
    if ActiveSession::load(git_dir).is_some() {
        return;
    }

    let config = match load_config(storage) {
        Some(c) => c,
        None => return,
    };

    if !config.auto_capture {
        return;
    }

    let workdir = match storage.workdir() {
        Some(w) => w.to_path_buf(),
        None => return,
    };

    // Discover Claude Code session files for this project
    let session_files = match ClaudeCodeImporter::discover_sessions(&workdir) {
        Ok(files) => files,
        Err(e) => {
            tracing::debug!("Auto-capture: failed to discover sessions: {e}");
            return;
        }
    };

    if session_files.is_empty() {
        tracing::debug!("Auto-capture: no Claude Code sessions found");
        return;
    }

    // Pick the most recently modified session file
    let newest = session_files
        .into_iter()
        .filter_map(|p| {
            let mtime = std::fs::metadata(&p).ok()?.modified().ok()?;
            Some((p, mtime))
        })
        .max_by_key(|(_, mtime)| *mtime)
        .map(|(p, _)| p);

    let session_path = match newest {
        Some(p) => p,
        None => return,
    };

    // Import the session
    let data = match ClaudeCodeImporter::import_session(&session_path) {
        Ok(d) => d,
        Err(e) => {
            tracing::debug!("Auto-capture: failed to import session: {e}");
            return;
        }
    };

    // Check for duplicates — if already imported, reuse the existing engram ID
    let engram_id = if let Some(existing_id) = data
        .manifest
        .source_hash
        .as_deref()
        .and_then(|h| storage.find_by_source_hash(h))
    {
        // Already imported this exact snapshot — reuse the ID
        existing_id
    } else {
        // New content — store the engram
        let id = data.manifest.id.clone();
        match storage.create(&data) {
            Ok(_) => {
                // Best-effort index update
                if let Ok(engine) = SearchEngine::open(storage) {
                    let _ = engine.index_engram(&data);
                }
                tracing::debug!("Auto-capture: imported engram {}", &id.as_str()[..8]);
                id
            }
            Err(e) => {
                tracing::debug!("Auto-capture: failed to store engram: {e}");
                return;
            }
        }
    };

    // Create a temporary ActiveSession so the core handler can inject trailers
    let agent = data.manifest.agent.clone();
    let mut session = ActiveSession::new(
        engram_id,
        AgentInfo {
            name: agent.name,
            model: agent.model,
            version: agent.version,
        },
    );
    session.token_usage = data.manifest.token_usage.clone();
    session.auto_capture = true;

    if let Err(e) = session.save(git_dir) {
        tracing::debug!("Auto-capture: failed to save session: {e}");
    }
}

/// After post-commit, if the session was auto-created, update the engram
/// with the commit SHA and clean up the session file.
fn maybe_auto_capture_cleanup(storage: &GitStorage, git_dir: &std::path::Path) {
    let session = match ActiveSession::load(git_dir) {
        Some(s) if s.auto_capture => s,
        _ => return,
    };

    // The core post-commit handler already recorded the commit SHA in the session.
    // Now read back the session to get the commit, update the engram, and clean up.
    let updated = ActiveSession::load(git_dir).unwrap_or(session);

    if !updated.commits.is_empty() {
        // Re-read the engram, update its git_commits, and re-store it
        if let Ok(mut data) = storage.read(updated.engram_id.as_str()) {
            for sha in &updated.commits {
                if !data.manifest.git_commits.contains(sha) {
                    data.manifest.git_commits.push(sha.clone());
                }
                if !data.lineage.git_commits.contains(sha) {
                    data.lineage.git_commits.push(sha.clone());
                }
            }
            // Re-store the updated engram (overwrites the ref)
            let _ = storage.create(&data);
        }
    }

    ActiveSession::cleanup(git_dir);
    tracing::debug!(
        "Auto-capture: finalized engram {} with {} commit(s)",
        &updated.engram_id.as_str()[..8],
        updated.commits.len()
    );
}

/// If push_on_push is enabled, automatically push engram refs alongside code.
fn maybe_auto_push(storage: &GitStorage) {
    let config = match load_config(storage) {
        Some(c) => c,
        None => return,
    };

    if !config.push_on_push {
        return;
    }

    let opts = SyncOptions::default();
    match push_engrams(storage.repo(), "origin", &opts) {
        Ok(result) => {
            if result.refs_pushed > 0 {
                eprintln!(
                    "engram: pushed {} engram ref(s) to {}",
                    result.refs_pushed, result.remote
                );
            }
        }
        Err(e) => {
            tracing::debug!("Auto-push: failed to push engrams: {e}");
        }
    }
}

/// Load EngramConfig from the repo, returning None on any error.
fn load_config(storage: &GitStorage) -> Option<EngramConfig> {
    let config = storage.repo().config().ok()?;
    EngramConfig::load(&config).ok()
}
