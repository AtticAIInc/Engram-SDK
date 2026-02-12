use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Args, ValueEnum};

use engram_capture::import::aider::AiderImporter;
use engram_capture::import::claude_code::ClaudeCodeImporter;
use engram_capture::import::detect::detect_sources;
use engram_core::storage::GitStorage;
use engram_query::search::SearchEngine;

#[derive(Args)]
pub struct ImportArgs {
    /// Path to session file or directory
    pub path: Option<PathBuf>,

    /// Format hint
    #[arg(long, value_enum)]
    pub format: Option<ImportFormat>,

    /// Auto-detect and import all discoverable sessions
    #[arg(long)]
    pub auto_detect: bool,

    /// Only show what would be imported (dry run)
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Clone, ValueEnum)]
pub enum ImportFormat {
    ClaudeCode,
    Aider,
}

/// Check if this engram was already imported (by source hash).
fn check_duplicate(
    storage: &GitStorage,
    data: &engram_core::model::EngramData,
) -> Option<engram_core::model::EngramId> {
    data.manifest
        .source_hash
        .as_deref()
        .and_then(|h| storage.find_by_source_hash(h))
}

/// Best-effort incremental search index update after storing an engram.
fn try_index(storage: &GitStorage, data: &engram_core::model::EngramData) {
    if let Ok(search) = SearchEngine::open(storage) {
        let _ = search.index_engram(data);
    }
}

pub fn run(args: &ImportArgs) -> Result<()> {
    let storage = GitStorage::discover().context("Not inside a Git repository")?;

    if !storage.is_initialized() {
        anyhow::bail!("Engram is not initialized. Run `engram init` first.");
    }

    if args.auto_detect {
        return run_auto_detect(&storage, args.dry_run);
    }

    let path = args
        .path
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Specify a path or use --auto-detect"))?;

    let format = args.format.as_ref().ok_or_else(|| {
        anyhow::anyhow!("Specify --format (claude-code or aider) or use --auto-detect")
    })?;

    match format {
        ImportFormat::ClaudeCode => {
            println!("Importing Claude Code session: {}", path.display());
            if args.dry_run {
                println!("  (dry run - no changes made)");
                return Ok(());
            }
            let data = ClaudeCodeImporter::import_session(path)
                .context("Failed to parse Claude Code session")?;
            if let Some(existing) = check_duplicate(&storage, &data) {
                println!(
                    "  Skipped (already imported as {})",
                    &existing.as_str()[..8]
                );
                return Ok(());
            }
            let tokens = data.manifest.token_usage.total_tokens;
            let entries = data.transcript.entries.len();
            let id = storage.create(&data).context("Failed to store engram")?;
            try_index(&storage, &data);
            println!(
                "  Imported engram {} ({} transcript entries, {} tokens)",
                &id.as_str()[..8],
                entries,
                tokens
            );
        }
        ImportFormat::Aider => {
            println!("Importing Aider history: {}", path.display());
            if args.dry_run {
                println!("  (dry run - no changes made)");
                return Ok(());
            }
            let engrams =
                AiderImporter::import_history(path).context("Failed to parse Aider history")?;
            for data in engrams {
                if let Some(existing) = check_duplicate(&storage, &data) {
                    println!(
                        "  Skipped (already imported as {})",
                        &existing.as_str()[..8]
                    );
                    continue;
                }
                let entries = data.transcript.entries.len();
                let id = storage.create(&data).context("Failed to store engram")?;
                try_index(&storage, &data);
                println!(
                    "  Imported engram {} ({} transcript entries)",
                    &id.as_str()[..8],
                    entries
                );
            }
        }
    }

    Ok(())
}

fn run_auto_detect(storage: &GitStorage, dry_run: bool) -> Result<()> {
    let workdir = storage
        .workdir()
        .ok_or_else(|| anyhow::anyhow!("Cannot determine working directory"))?;

    let sources = detect_sources(workdir).context("Failed to detect import sources")?;

    if sources.is_empty() {
        println!("No importable sessions found.");
        println!();
        println!("Looked for:");
        println!("  - Claude Code sessions in ~/.claude/projects/");
        println!("  - Aider history in .aider.chat.history.md");
        return Ok(());
    }

    println!("Found {} importable source(s):", sources.len());
    for source in &sources {
        println!("  - {}", source.description());
    }

    if dry_run {
        println!();
        println!("(dry run - no changes made)");
        return Ok(());
    }

    println!();
    let mut total_imported = 0;

    for source in &sources {
        match source {
            engram_capture::import::detect::ImportSource::ClaudeCode { session_path } => {
                match ClaudeCodeImporter::import_session(session_path) {
                    Ok(data) => {
                        if let Some(existing) = check_duplicate(storage, &data) {
                            println!(
                                "  Skipped {} (already imported as {})",
                                session_path.display(),
                                &existing.as_str()[..8]
                            );
                            continue;
                        }
                        let entries = data.transcript.entries.len();
                        let tokens = data.manifest.token_usage.total_tokens;
                        match storage.create(&data) {
                            Ok(id) => {
                                try_index(storage, &data);
                                println!(
                                    "  Imported {} ({} entries, {} tokens)",
                                    &id.as_str()[..8],
                                    entries,
                                    tokens,
                                );
                                total_imported += 1;
                            }
                            Err(e) => {
                                eprintln!("  Error storing {}: {e}", session_path.display());
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("  Error importing {}: {e}", session_path.display());
                    }
                }
            }
            engram_capture::import::detect::ImportSource::Aider { history_path } => {
                match AiderImporter::import_history(history_path) {
                    Ok(engrams) => {
                        for data in engrams {
                            if let Some(existing) = check_duplicate(storage, &data) {
                                println!(
                                    "  Skipped aider session (already imported as {})",
                                    &existing.as_str()[..8]
                                );
                                continue;
                            }
                            let entries = data.transcript.entries.len();
                            match storage.create(&data) {
                                Ok(id) => {
                                    try_index(storage, &data);
                                    println!(
                                        "  Imported {} ({} entries)",
                                        &id.as_str()[..8],
                                        entries,
                                    );
                                    total_imported += 1;
                                }
                                Err(e) => {
                                    eprintln!("  Error storing aider session: {e}");
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("  Error importing {}: {e}", history_path.display());
                    }
                }
            }
        }
    }

    println!();
    println!("Imported {total_imported} engram(s).");

    Ok(())
}
