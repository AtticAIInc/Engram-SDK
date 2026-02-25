use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result};
use clap::Args;
use engram_core::notes::{format_note, ENGRAM_NOTES_REF};
use engram_core::storage::{GitStorage, ListOptions};

#[derive(Args)]
pub struct AnnotateArgs {
    /// Git range to annotate (e.g., "main..HEAD"). If omitted, annotates all linked commits.
    pub range: Option<String>,

    /// Preview what would be annotated without writing notes
    #[arg(long)]
    pub dry_run: bool,

    /// Overwrite existing notes
    #[arg(long)]
    pub force: bool,
}

pub fn run(args: &AnnotateArgs) -> Result<()> {
    let storage = GitStorage::discover().context("Not inside a Git repository")?;

    if !storage.is_initialized() {
        anyhow::bail!("Engram is not initialized. Run `engram init` first.");
    }

    let repo = storage.repo();

    // Collect allowed commit SHAs if a range is specified
    let range_filter: Option<HashSet<String>> = if let Some(range) = &args.range {
        let mut shas = HashSet::new();
        let parts: Vec<&str> = range.split("..").collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid range format. Expected 'base..head' (e.g., 'main..HEAD').");
        }
        let base_obj = repo
            .revparse_single(parts[0])
            .context(format!("Cannot resolve '{}'", parts[0]))?;
        let head_obj = repo
            .revparse_single(parts[1])
            .context(format!("Cannot resolve '{}'", parts[1]))?;

        let mut revwalk = repo.revwalk().context("Cannot create revwalk")?;
        revwalk
            .push(head_obj.id())
            .context("Cannot push head to revwalk")?;
        revwalk
            .hide(base_obj.id())
            .context("Cannot hide base in revwalk")?;

        for oid in revwalk {
            let oid = oid.context("Revwalk error")?;
            shas.insert(oid.to_string());
        }
        Some(shas)
    } else {
        None
    };

    // Build a map of commit SHA -> engram data
    let manifests = storage
        .list(&ListOptions::default())
        .context("Failed to list engrams")?;

    // Map commit SHA -> engram ID (first one wins if multiple engrams reference same commit)
    let mut commit_to_engram: HashMap<String, String> = HashMap::new();
    for m in &manifests {
        for sha in &m.git_commits {
            // If range filter is set, only include commits in that range
            if let Some(ref filter) = range_filter {
                if !filter.contains(sha) {
                    continue;
                }
            }
            commit_to_engram
                .entry(sha.clone())
                .or_insert_with(|| m.id.as_str().to_string());
        }
    }

    if commit_to_engram.is_empty() {
        if args.range.is_some() {
            println!("No engram-linked commits found in the specified range.");
        } else {
            println!("No engrams with linked commits found.");
            println!(
                "Hint: Commits are linked when using `engram record` or Claude Code auto-capture."
            );
        }
        return Ok(());
    }

    let sig = repo
        .signature()
        .unwrap_or_else(|_| git2::Signature::now("engram", "engram@localhost").unwrap());

    let mut annotated = 0;
    let mut skipped = 0;

    for (sha, engram_id) in &commit_to_engram {
        let oid = git2::Oid::from_str(sha).context(format!("Invalid commit SHA: {sha}"))?;

        // Check if note already exists
        let has_note = repo.find_note(Some(ENGRAM_NOTES_REF), oid).is_ok();
        if has_note && !args.force {
            skipped += 1;
            if args.dry_run {
                println!("  skip {}: already annotated (use --force)", &sha[..8]);
            }
            continue;
        }

        // Read the full engram data
        let data = match storage.read(engram_id) {
            Ok(d) => d,
            Err(e) => {
                tracing::debug!("Cannot read engram {engram_id}: {e}");
                continue;
            }
        };

        let note = format_note(&data);

        if args.dry_run {
            let action = if has_note { "overwrite" } else { "annotate" };
            println!("  {action} {}:", &sha[..8.min(sha.len())]);
            for line in note.lines() {
                println!("    {line}");
            }
            annotated += 1;
            continue;
        }

        // Remove existing note if force
        if has_note {
            let _ = repo.note_delete(oid, Some(ENGRAM_NOTES_REF), &sig, &sig);
        }

        repo.note(&sig, &sig, Some(ENGRAM_NOTES_REF), oid, &note, args.force)
            .context(format!("Failed to create note on {sha}"))?;
        annotated += 1;
    }

    if args.dry_run {
        println!(
            "\nWould annotate {} commit(s), skip {} (already annotated).",
            annotated, skipped
        );
    } else {
        println!("Annotated {} commit(s).", annotated);
        if skipped > 0 {
            println!("Skipped {skipped} (already annotated, use --force to overwrite).");
        }
        if annotated > 0 {
            println!("\nView with: git log --notes=engram");
            println!("Or use:    git loge");
        }
    }

    Ok(())
}
