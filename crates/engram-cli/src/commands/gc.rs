use anyhow::{Context, Result};
use chrono::{Duration, Utc};
use clap::Args;
use engram_core::storage::{GitStorage, ListOptions};

#[derive(Args)]
pub struct GcArgs {
    /// Delete engrams older than this duration (e.g. "30d", "6m", "1y")
    #[arg(long)]
    pub older_than: Option<String>,

    /// Preview what would be deleted without actually deleting
    #[arg(long)]
    pub dry_run: bool,

    /// Skip confirmation prompt
    #[arg(long, short)]
    pub yes: bool,
}

pub fn run(args: &GcArgs) -> Result<()> {
    let storage = GitStorage::discover().context("Not inside a Git repository")?;

    if !storage.is_initialized() {
        anyhow::bail!("Engram is not initialized. Run `engram init` first.");
    }

    let cutoff = if let Some(duration_str) = &args.older_than {
        let dur = parse_duration(duration_str)?;
        Some(Utc::now() - dur)
    } else {
        None
    };

    let manifests = storage
        .list(&ListOptions::default())
        .context("Failed to list engrams")?;

    let to_delete: Vec<_> = manifests
        .iter()
        .filter(|m| {
            if let Some(cutoff) = cutoff {
                m.created_at < cutoff
            } else {
                false
            }
        })
        .collect();

    if to_delete.is_empty() {
        println!("No engrams match the deletion criteria.");
        return Ok(());
    }

    println!(
        "{} engram(s) to {}:",
        to_delete.len(),
        if args.dry_run {
            "delete (dry run)"
        } else {
            "delete"
        }
    );
    for m in &to_delete {
        println!(
            "  {} {} [{}] {}",
            &m.id.as_str()[..8],
            m.created_at.format("%Y-%m-%d %H:%M"),
            m.agent.name,
            m.summary.as_deref().unwrap_or("(no summary)")
        );
    }

    if args.dry_run {
        println!("\nDry run — no engrams were deleted.");
        return Ok(());
    }

    if !args.yes {
        eprintln!("\nUse --yes to confirm deletion.");
        return Ok(());
    }

    let mut deleted = 0;
    for m in &to_delete {
        match storage.delete(m.id.as_str()) {
            Ok(()) => deleted += 1,
            Err(e) => eprintln!("Failed to delete {}: {e}", &m.id.as_str()[..8]),
        }
    }

    println!("\nDeleted {deleted} engram(s).");
    Ok(())
}

fn parse_duration(s: &str) -> Result<Duration> {
    let s = s.trim();
    if s.is_empty() {
        anyhow::bail!("Empty duration string");
    }

    let (num_str, unit) = s.split_at(s.len() - 1);
    let num: i64 = num_str
        .parse()
        .with_context(|| format!("Invalid duration number: {num_str}"))?;

    match unit {
        "d" => Ok(Duration::days(num)),
        "w" => Ok(Duration::weeks(num)),
        "m" => Ok(Duration::days(num * 30)),
        "y" => Ok(Duration::days(num * 365)),
        _ => anyhow::bail!(
            "Unknown duration unit '{unit}'. Use d (days), w (weeks), m (months), y (years)."
        ),
    }
}
