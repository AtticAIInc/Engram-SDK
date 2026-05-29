use std::io::Read as _;
use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use engram_capture::import::claude_code::ClaudeCodeImporter;
use engram_capture::summarize::summarize_intent;
use engram_core::config::EngramConfig;
use engram_core::eventlog;
use engram_core::hooks;
use engram_core::hooks::ActiveSession;
use engram_core::model::AgentInfo;
use engram_core::storage::GitStorage;
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
        "session-end" => {
            handle_session_end(&storage);
        }
        other => {
            tracing::debug!("Unknown hook: {other}, ignoring");
        }
    }

    Ok(())
}

/// Handle Claude Code's `SessionEnd` hook.
///
/// Reads JSON from stdin (provided by Claude Code), extracts the `transcript_path`,
/// and imports the session as an engram. All errors are logged at debug level and
/// silently ignored so we never interfere with the user's workflow.
fn handle_session_end(storage: &GitStorage) {
    let git_dir = storage.repo().path().to_path_buf();

    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        tracing::debug!("session-end: failed to read stdin");
        eventlog::warn(&git_dir, "session-end: failed to read stdin");
        return;
    }

    let json: serde_json::Value = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(e) => {
            tracing::debug!("session-end: failed to parse JSON: {e}");
            eventlog::warn(
                &git_dir,
                format!("session-end: failed to parse hook JSON: {e}"),
            );
            return;
        }
    };

    let transcript_path = match json.get("transcript_path").and_then(|v| v.as_str()) {
        Some(p) => std::path::PathBuf::from(p),
        None => {
            tracing::debug!("session-end: no transcript_path in input");
            eventlog::warn(&git_dir, "session-end: hook input had no transcript_path");
            return;
        }
    };

    if !transcript_path.exists() {
        tracing::debug!(
            "session-end: transcript file does not exist: {}",
            transcript_path.display()
        );
        eventlog::warn(
            &git_dir,
            format!(
                "session-end: transcript file does not exist: {}",
                transcript_path.display()
            ),
        );
        return;
    }

    let mut data = match ClaudeCodeImporter::import_session(&transcript_path) {
        Ok(d) => d,
        Err(e) => {
            tracing::debug!("session-end: failed to import session: {e}");
            eventlog::error(
                &git_dir,
                format!("session-end: failed to import session: {e}"),
            );
            return;
        }
    };

    // Best-effort LLM summarization
    if let Err(e) = summarize_intent(&mut data) {
        tracing::debug!("session-end: LLM summarization failed: {e}");
        eventlog::warn(
            &git_dir,
            format!("session-end: LLM summarization failed (kept heuristic summary): {e}"),
        );
    }

    // Check for duplicates via source_hash
    if let Some(existing_id) = data
        .manifest
        .source_hash
        .as_deref()
        .and_then(|h| storage.find_by_source_hash(h))
    {
        tracing::debug!(
            "session-end: already imported as {}",
            &existing_id.as_str()[..8]
        );
        return;
    }

    let id = data.manifest.id.clone();
    match storage.create(&data) {
        Ok(_) => {
            // Best-effort index update
            if let Ok(engine) = SearchEngine::open(storage) {
                if let Err(e) = engine.index_engram(&data) {
                    eventlog::warn(
                        &git_dir,
                        format!(
                            "session-end: search indexing failed for {}: {e}",
                            &id.as_str()[..8]
                        ),
                    );
                }
            }
            tracing::debug!("session-end: imported engram {}", &id.as_str()[..8]);
            eventlog::info(
                &git_dir,
                format!(
                    "session-end: captured engram {} ({} tokens)",
                    &id.as_str()[..8],
                    data.manifest.token_usage.total_tokens
                ),
            );

            // Best-effort: annotate commits that reference this engram
            auto_annotate_commits(storage, &data);
        }
        Err(e) => {
            tracing::debug!("session-end: failed to store engram: {e}");
            eventlog::error(
                &git_dir,
                format!("session-end: failed to store engram: {e}"),
            );
        }
    }
}

/// Scan recent commits for Engram-Id trailers matching the given engram,
/// and attach git notes with rich reasoning metadata.
fn auto_annotate_commits(storage: &GitStorage, data: &engram_core::model::EngramData) {
    use engram_core::notes::{format_note, ENGRAM_NOTES_REF};

    let repo = storage.repo();
    let engram_id_str = data.manifest.id.as_str().to_string();

    // Walk recent commits looking for Engram-Id trailers
    let head = match repo.head().and_then(|h| h.peel_to_commit()) {
        Ok(c) => c,
        Err(_) => return,
    };

    let sig = repo
        .signature()
        .unwrap_or_else(|_| git2::Signature::now("engram", "engram@localhost").unwrap());

    let mut revwalk = match repo.revwalk() {
        Ok(r) => r,
        Err(_) => return,
    };
    if revwalk.push(head.id()).is_err() {
        return;
    }

    let note = format_note(data);
    let mut annotated = 0;

    for oid_result in revwalk.take(50) {
        let oid = match oid_result {
            Ok(o) => o,
            Err(_) => break,
        };
        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if let Some(message) = commit.message() {
            for line in message.lines() {
                if let Some(trailer_id) = line.strip_prefix("Engram-Id: ") {
                    if trailer_id.trim() == engram_id_str {
                        // Skip if already annotated
                        if repo.find_note(Some(ENGRAM_NOTES_REF), oid).is_ok() {
                            continue;
                        }
                        if repo
                            .note(&sig, &sig, Some(ENGRAM_NOTES_REF), oid, &note, false)
                            .is_ok()
                        {
                            annotated += 1;
                        }
                    }
                }
            }
        }
    }

    if annotated > 0 {
        tracing::debug!("session-end: annotated {annotated} commit(s) with engram notes");
    }
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
            eventlog::warn(
                git_dir,
                format!("auto-capture: failed to discover sessions: {e}"),
            );
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
    let mut data = match ClaudeCodeImporter::import_session(&session_path) {
        Ok(d) => d,
        Err(e) => {
            tracing::debug!("Auto-capture: failed to import session: {e}");
            eventlog::error(
                git_dir,
                format!("auto-capture: failed to import session: {e}"),
            );
            return;
        }
    };

    // Best-effort LLM summarization
    if let Err(e) = summarize_intent(&mut data) {
        tracing::debug!("Auto-capture: LLM summarization failed: {e}");
        eventlog::warn(
            git_dir,
            format!("auto-capture: LLM summarization failed (kept heuristic summary): {e}"),
        );
    }

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
                    if let Err(e) = engine.index_engram(&data) {
                        eventlog::warn(
                            git_dir,
                            format!(
                                "auto-capture: search indexing failed for {}: {e}",
                                &id.as_str()[..8]
                            ),
                        );
                    }
                }
                tracing::debug!("Auto-capture: imported engram {}", &id.as_str()[..8]);
                eventlog::info(
                    git_dir,
                    format!(
                        "auto-capture: captured engram {} ({} tokens)",
                        &id.as_str()[..8],
                        data.manifest.token_usage.total_tokens
                    ),
                );
                id
            }
            Err(e) => {
                tracing::debug!("Auto-capture: failed to store engram: {e}");
                eventlog::error(
                    git_dir,
                    format!("auto-capture: failed to store engram: {e}"),
                );
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
        eventlog::warn(
            git_dir,
            format!("auto-capture: failed to save active session: {e}"),
        );
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
/// Uses the git CLI instead of libgit2 so it inherits the user's credential helpers.
/// Sets ENGRAM_PUSHING=1 to prevent recursive hook invocation.
fn maybe_auto_push(storage: &GitStorage) {
    // Guard against recursive invocation: our `git push` triggers pre-push again
    if std::env::var_os("ENGRAM_PUSHING").is_some() {
        return;
    }

    let config = match load_config(storage) {
        Some(c) => c,
        None => return,
    };

    if !config.push_on_push {
        return;
    }

    let workdir = match storage.workdir() {
        Some(w) => w.to_path_buf(),
        None => return,
    };
    let git_dir = storage.repo().path().to_path_buf();

    // Force-push engram refs: they may be updated in-place (e.g. when commit SHAs
    // are appended), so the push is not necessarily a fast-forward.
    match std::process::Command::new("git")
        .args([
            "push",
            "--force",
            "origin",
            "+refs/engrams/*:refs/engrams/*",
        ])
        .env("ENGRAM_PUSHING", "1")
        .current_dir(&workdir)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
    {
        Ok(output) if output.status.success() => {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let pushed = stderr.lines().filter(|l| l.contains("->")).count();
            if pushed > 0 {
                eprintln!("engram: pushed {pushed} engram ref(s) to origin");
                eventlog::info(
                    &git_dir,
                    format!("auto-push: pushed {pushed} engram ref(s) to origin"),
                );
            }
        }
        Ok(output) => {
            // Don't pollute the user's git push output, but leave a breadcrumb.
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::debug!("Auto-push: git push failed: {stderr}");
            eventlog::warn(
                &git_dir,
                format!("auto-push: git push failed: {}", stderr.trim()),
            );
        }
        Err(e) => {
            tracing::debug!("Auto-push: failed to run git: {e}");
            eventlog::warn(&git_dir, format!("auto-push: failed to run git: {e}"));
        }
    }
}

/// Load EngramConfig from the repo, returning None on any error.
fn load_config(storage: &GitStorage) -> Option<EngramConfig> {
    let config = storage.repo().config().ok()?;
    EngramConfig::load(&config).ok()
}
