use anyhow::{Context, Result};
use clap::Args;

use engram_core::config::EngramConfig;
use engram_core::eventlog::{self, Level};
use engram_core::hooks::{claude_code, installer};
use engram_core::storage::{GitStorage, ListOptions};

use crate::output::OutputFormat;

#[derive(Args)]
pub struct DoctorArgs {
    /// Number of recent log events to display
    #[arg(long, default_value = "15")]
    pub events: usize,
}

pub fn run(args: &DoctorArgs, format: OutputFormat) -> Result<()> {
    let storage = GitStorage::discover().context("Not inside a Git repository")?;
    let git_dir = storage.repo().path().to_path_buf();
    let workdir = storage.workdir().map(|w| w.to_path_buf());

    // --- Configuration ---------------------------------------------------
    let config = storage
        .repo()
        .config()
        .ok()
        .and_then(|c| EngramConfig::load(&c).ok());

    // --- Hooks -----------------------------------------------------------
    let installed = installer::installed_hooks(&git_dir);
    let managed = installer::managed_hooks();
    let claude_hook = workdir
        .as_deref()
        .map(claude_code::claude_code_hook_installed)
        .unwrap_or(false);

    // --- Storage / index -------------------------------------------------
    let manifests = storage.list(&ListOptions::default()).unwrap_or_default();
    let engram_count = manifests.len();
    let latest = manifests.iter().map(|m| m.created_at).max();
    let index_present =
        git_dir.join("engram-index").exists() && git_dir.join("engram-index/meta.json").exists();

    // --- Event log -------------------------------------------------------
    let events = eventlog::read_recent(&git_dir, args.events);
    let warn_count = events.iter().filter(|e| e.level == Level::Warn).count();
    let error_count = events.iter().filter(|e| e.level == Level::Error).count();
    let last_capture = events
        .iter()
        .rev()
        .find(|e| e.level == Level::Info && e.message.contains("captured engram"))
        .map(|e| e.timestamp.clone());

    if matches!(format, OutputFormat::Json) {
        let json = serde_json::json!({
            "enabled": config.as_ref().map(|c| c.enabled).unwrap_or(false),
            "auto_capture": config.as_ref().map(|c| c.auto_capture).unwrap_or(false),
            "push_on_push": config.as_ref().map(|c| c.push_on_push).unwrap_or(false),
            "default_agent": config.as_ref().and_then(|c| c.default_agent.clone()),
            "git_hooks_installed": installed,
            "git_hooks_managed": managed,
            "claude_code_hook_installed": claude_hook,
            "engram_count": engram_count,
            "latest_engram": latest.map(|t| t.to_rfc3339()),
            "search_index_present": index_present,
            "recent_warnings": warn_count,
            "recent_errors": error_count,
            "last_logged_capture": last_capture,
        });
        println!("{}", serde_json::to_string_pretty(&json)?);
        return Ok(());
    }

    // --- Text report -----------------------------------------------------
    let ok = "\u{2713}"; // ✓
    let bad = "\u{2717}"; // ✗
    let warn = "\u{26a0}"; // ⚠

    println!("engram doctor\n");

    // Repository
    match &config {
        Some(c) if c.enabled => println!("{ok} Repository initialized for engram"),
        _ => println!("{bad} Repository not initialized for engram — run `engram init`"),
    }

    // Configuration
    if let Some(c) = &config {
        let mark = |b: bool| if b { ok } else { bad };
        println!("\nConfiguration:");
        println!(
            "  {} auto-capture (capture sessions on commit)",
            mark(c.auto_capture)
        );
        println!(
            "  {} push-on-push (push engram refs with code)",
            mark(c.push_on_push)
        );
        if let Some(agent) = &c.default_agent {
            println!("  {ok} default agent: {agent}");
        }
    }

    // Hooks
    println!("\nGit hooks:");
    for hook in managed {
        if installed.contains(hook) {
            println!("  {ok} {hook}");
        } else {
            println!("  {bad} {hook} (not installed — run `engram init`)");
        }
    }
    if claude_hook {
        println!("  {ok} Claude Code SessionEnd hook");
    } else {
        println!("  {warn} Claude Code SessionEnd hook not found in .claude/settings.json");
    }

    // Storage
    println!("\nStorage:");
    println!("  {ok} {engram_count} engram(s) stored");
    if let Some(t) = latest {
        println!("  {ok} latest engram: {}", t.format("%Y-%m-%d %H:%M UTC"));
    }
    if index_present {
        println!("  {ok} search index present");
    } else if engram_count > 0 {
        println!("  {warn} search index missing — run `engram reindex`");
    }

    // Recent activity
    println!("\nRecent activity (.git/engram.log):");
    if events.is_empty() {
        println!("  (no events logged yet)");
    } else {
        if error_count > 0 || warn_count > 0 {
            println!(
                "  {warn} {error_count} error(s), {warn_count} warning(s) in last {} event(s)",
                events.len()
            );
        }
        if let Some(t) = &last_capture {
            println!("  {ok} last successful capture logged: {t}");
        }
        println!();
        for e in &events {
            let glyph = match e.level {
                Level::Info => ok,
                Level::Warn => warn,
                Level::Error => bad,
            };
            println!("  {glyph} {} {}", e.timestamp, e.message);
        }
    }

    // Verdict
    println!();
    if config.as_ref().map(|c| c.enabled).unwrap_or(false) && error_count == 0 {
        println!("{ok} engram looks healthy.");
    } else if error_count > 0 {
        println!("{warn} engram recorded {error_count} recent error(s) — see the log above.");
    }

    Ok(())
}
