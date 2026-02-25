use std::collections::HashMap;

use anyhow::{Context, Result};
use clap::Args;
use engram_core::config::{EngramConfig, GlobalConfig};
use engram_core::hooks;
use engram_core::notes::{format_note, ENGRAM_NOTES_REF};
use engram_core::storage::{GitStorage, ListOptions};

#[derive(Args)]
pub struct InitArgs {
    /// Force re-initialization
    #[arg(long)]
    pub force: bool,

    /// Remote name to configure refspecs on (default: all remotes)
    #[arg(long)]
    pub remote: Option<String>,

    /// Skip installing Claude Code SessionEnd hook
    #[arg(long)]
    pub no_claude_code: bool,

    /// Disable auto-capture of agent sessions on commit
    #[arg(long)]
    pub no_auto_capture: bool,

    /// Disable auto-push of engram refs on git push
    #[arg(long)]
    pub no_auto_push: bool,

    /// (Deprecated: Claude Code hook is now installed by default)
    #[arg(long, hide = true)]
    pub claude_code: bool,
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

    // Save config with smart defaults (opt-out overrides)
    let config = EngramConfig {
        auto_capture: !args.no_auto_capture,
        push_on_push: !args.no_auto_push,
        ..EngramConfig::default_init()
    };
    let mut git_config = storage
        .repo()
        .config()
        .context("Failed to open git config")?;
    config
        .save(&mut git_config)
        .context("Failed to save config")?;

    // Install git alias for viewing engram notes
    let _ = git_config.set_str("alias.loge", "log --notes=refs/notes/engram");

    // Install Claude Code SessionEnd hook (default: on)
    let claude_code_ok = if !args.no_claude_code {
        if let Some(workdir) = storage.workdir() {
            hooks::install_claude_code_hook(workdir).is_ok()
        } else {
            false
        }
    } else {
        false
    };

    // Register in global config for cross-repo search
    if let Some(workdir) = storage.workdir() {
        if let Ok(mut global) = GlobalConfig::load() {
            if global.add_repo(workdir) {
                let _ = global.save();
            }
        }
    }

    // Auto-annotate any existing engram-linked commits
    let annotated = auto_annotate_existing(&storage);

    // Print summary
    println!("Engram initialized. Reasoning capture is ready.");
    println!();

    let on = "ON ";
    let off = "OFF";

    print!("  Auto-capture:     ");
    if config.auto_capture {
        println!("{on} (agent sessions imported on commit)");
    } else {
        println!("{off} (disabled via --no-auto-capture)");
    }

    print!("  Auto-push:        ");
    if config.push_on_push {
        println!("{on} (engram refs sync on git push)");
    } else {
        println!("{off} (disabled via --no-auto-push)");
    }

    print!("  Claude Code hook: ");
    if claude_code_ok {
        println!("{on} (sessions auto-imported on exit)");
    } else if args.no_claude_code {
        println!("{off} (disabled via --no-claude-code)");
    } else {
        println!("{off} (could not detect working directory)");
    }

    let has_api_key = std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .filter(|k| !k.is_empty())
        .is_some()
        || GlobalConfig::load()
            .ok()
            .and_then(|c| c.settings.anthropic_api_key)
            .is_some();

    print!("  LLM summarize:    ");
    if has_api_key {
        println!("{on} (API key configured)");
    } else {
        println!("{off} (set API key for enhanced summaries)");
    }

    println!("  Git notes alias:  {on} (use `git loge` to view reasoning)");

    if annotated > 0 {
        println!("  Annotated {annotated} commit(s) with engram reasoning notes.");
    }

    println!();
    println!("Next steps:");
    println!("  engram log                         List captured engrams");
    println!("  engram search \"query\"              Search reasoning history");
    println!("  engram why src/file.rs             Why does this file exist?");

    if !has_api_key {
        println!();
        println!("Tip: Set an API key to enable LLM-powered summarization:");
        println!("  engram config set anthropic_api_key sk-ant-...");
        println!("  (or set ANTHROPIC_API_KEY environment variable)");
    }

    Ok(())
}

/// Best-effort: annotate any existing engram-linked commits with git notes.
fn auto_annotate_existing(storage: &GitStorage) -> usize {
    let manifests = match storage.list(&ListOptions::default()) {
        Ok(m) => m,
        Err(_) => return 0,
    };

    // Build commit SHA -> engram ID map
    let mut commit_to_engram: HashMap<String, String> = HashMap::new();
    for m in &manifests {
        for sha in &m.git_commits {
            commit_to_engram
                .entry(sha.clone())
                .or_insert_with(|| m.id.as_str().to_string());
        }
    }

    if commit_to_engram.is_empty() {
        return 0;
    }

    let repo = storage.repo();
    let sig = repo
        .signature()
        .unwrap_or_else(|_| git2::Signature::now("engram", "engram@localhost").unwrap());

    let mut annotated = 0;

    for (sha, engram_id) in &commit_to_engram {
        let oid = match git2::Oid::from_str(sha) {
            Ok(o) => o,
            Err(_) => continue,
        };

        // Skip if already annotated
        if repo.find_note(Some(ENGRAM_NOTES_REF), oid).is_ok() {
            continue;
        }

        let data = match storage.read(engram_id) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let note = format_note(&data);
        if repo
            .note(&sig, &sig, Some(ENGRAM_NOTES_REF), oid, &note, false)
            .is_ok()
        {
            annotated += 1;
        }
    }

    annotated
}
