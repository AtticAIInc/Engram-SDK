use std::collections::BTreeMap;

use anyhow::{Context, Result};
use clap::Args;
use engram_core::storage::{GitStorage, ListOptions};

use crate::output::format::format_manifest_list;
use crate::output::OutputFormat;

#[derive(Args)]
pub struct LogArgs {
    /// Show token costs
    #[arg(long)]
    pub cost: bool,

    /// Maximum number of entries
    #[arg(short = 'n', long, default_value = "20")]
    pub limit: usize,

    /// Filter by agent name
    #[arg(long)]
    pub agent: Option<String>,

    /// Group output by agent name
    #[arg(long)]
    pub by_agent: bool,
}

pub fn run(args: &LogArgs, format: OutputFormat) -> Result<()> {
    let storage = GitStorage::discover().context("Not inside a Git repository")?;

    if !storage.is_initialized() {
        anyhow::bail!("Engram is not initialized. Run `engram init` first.");
    }

    let opts = ListOptions {
        limit: Some(args.limit),
        agent_filter: args.agent.clone(),
    };
    let manifests = storage.list(&opts).context("Failed to list engrams")?;

    if args.by_agent {
        let mut grouped: BTreeMap<String, Vec<_>> = BTreeMap::new();
        for m in &manifests {
            grouped
                .entry(m.agent.name.clone())
                .or_default()
                .push(m.clone());
        }
        for (agent, entries) in &grouped {
            println!("## {agent} ({} engrams)", entries.len());
            let output = format_manifest_list(entries, args.cost, format);
            print!("{output}");
            println!();
        }
    } else {
        let output = format_manifest_list(&manifests, args.cost, format);
        print!("{output}");
    }

    Ok(())
}
