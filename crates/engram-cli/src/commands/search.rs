use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;

use engram_core::config::GlobalConfig;
use engram_core::storage::GitStorage;
use engram_query::{SearchEngine, SearchResult};

use crate::output::OutputFormat;

#[derive(Args)]
pub struct SearchArgs {
    /// Search query (free-text, searches intent, transcript, file paths, dead ends)
    pub query: String,

    /// Maximum number of results
    #[arg(short = 'n', long, default_value = "10")]
    pub limit: usize,

    /// Search across all registered repositories
    #[arg(long)]
    pub global: bool,

    /// Search specific repositories (comma-separated paths)
    #[arg(long, value_delimiter = ',')]
    pub repos: Vec<PathBuf>,
}

pub fn run(args: &SearchArgs, format: OutputFormat) -> Result<()> {
    if args.global || !args.repos.is_empty() {
        return run_multi_repo(args, format);
    }

    let storage = GitStorage::discover().context("Not in a Git repository with engram")?;
    let engine = SearchEngine::open(&storage)?;

    let results = engine.search(&storage, &args.query, args.limit)?;

    if results.is_empty() {
        eprintln!("No results found for: {}", args.query);
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            let manifests: Vec<_> = results.iter().map(|r| &r.manifest).collect();
            println!("{}", serde_json::to_string_pretty(&manifests)?);
        }
        OutputFormat::Text | OutputFormat::Markdown => {
            eprintln!("Found {} result(s) for: {}\n", results.len(), args.query);
            for result in &results {
                let m = &result.manifest;
                let short_id = &m.id.as_str()[..8];
                let summary = m.summary.as_deref().unwrap_or("(no summary)");
                let score = result.score;
                println!("{short_id}  {summary}  (score: {score:.2})");
            }
        }
    }

    Ok(())
}

fn run_multi_repo(args: &SearchArgs, format: OutputFormat) -> Result<()> {
    let repo_paths: Vec<PathBuf> = if !args.repos.is_empty() {
        args.repos.clone()
    } else {
        let config = GlobalConfig::load().context("Failed to load global config")?;
        if config.repos.is_empty() {
            anyhow::bail!(
                "No repositories registered. Run `engram init` in your repos, or use --repos."
            );
        }
        config.repos
    };

    let mut all_results: Vec<(String, SearchResult)> = Vec::new();

    for repo_path in &repo_paths {
        let storage = match GitStorage::open(repo_path) {
            Ok(s) if s.is_initialized() => s,
            _ => {
                tracing::debug!("Skipping {}: not initialized", repo_path.display());
                continue;
            }
        };

        let engine = match SearchEngine::open(&storage) {
            Ok(e) => e,
            Err(e) => {
                tracing::debug!("Skipping {}: {}", repo_path.display(), e);
                continue;
            }
        };

        match engine.search(&storage, &args.query, args.limit) {
            Ok(results) => {
                let label = repo_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| repo_path.display().to_string());
                for r in results {
                    all_results.push((label.clone(), r));
                }
            }
            Err(e) => {
                tracing::debug!("Search failed in {}: {}", repo_path.display(), e);
            }
        }
    }

    // Sort by score descending
    all_results.sort_by(|a, b| {
        b.1.score
            .partial_cmp(&a.1.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    all_results.truncate(args.limit);

    if all_results.is_empty() {
        eprintln!(
            "No results found for '{}' across {} repositories.",
            args.query,
            repo_paths.len()
        );
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            let entries: Vec<_> = all_results
                .iter()
                .map(|(repo, r)| {
                    serde_json::json!({
                        "repo": repo,
                        "manifest": r.manifest,
                        "score": r.score,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&entries)?);
        }
        OutputFormat::Text | OutputFormat::Markdown => {
            eprintln!(
                "Found {} result(s) for '{}' across {} repositories:\n",
                all_results.len(),
                args.query,
                repo_paths.len()
            );
            for (repo, result) in &all_results {
                let m = &result.manifest;
                let short_id = &m.id.as_str()[..8];
                let summary = m.summary.as_deref().unwrap_or("(no summary)");
                let score = result.score;
                println!("[{repo}] {short_id}  {summary}  (score: {score:.2})");
            }
        }
    }

    Ok(())
}
