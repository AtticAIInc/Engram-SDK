use anyhow::{Context, Result};
use clap::Args;

use engram_core::storage::GitStorage;
use engram_query::build_graph;

use crate::output::OutputFormat;

#[derive(Args)]
pub struct GraphArgs {
    /// Center node (e.g. "file:src/auth.rs" or engram ID prefix)
    pub node: Option<String>,

    /// Traversal depth from center node
    #[arg(long, default_value = "2")]
    pub depth: usize,

    /// Output DOT format for Graphviz
    #[arg(long)]
    pub dot: bool,
}

pub fn run(args: &GraphArgs, format: OutputFormat) -> Result<()> {
    let storage = GitStorage::discover().context("Not in a Git repository with engram")?;

    let full_graph = build_graph(&storage)?;

    let graph = if let Some(center) = &args.node {
        // Convert user-friendly node references to internal IDs
        let node_id = if center.starts_with("file:") || center.starts_with("agent:") {
            center.clone()
        } else {
            // Assume engram ID prefix
            format!("engram:{center}")
        };
        full_graph.subgraph(&node_id, args.depth)
    } else {
        full_graph
    };

    if args.dot {
        print!("{}", graph.to_dot());
        return Ok(());
    }

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&graph)?);
        }
        OutputFormat::Text | OutputFormat::Markdown => {
            println!(
                "Context graph: {} nodes, {} edges",
                graph.nodes.len(),
                graph.edges.len()
            );
            println!();
            for node in &graph.nodes {
                println!("  [{:?}] {} - {}", node.node_type, node.id, node.label);
            }
            if !graph.edges.is_empty() {
                println!();
                for edge in &graph.edges {
                    println!("  {} --[{:?}]--> {}", edge.from, edge.edge_type, edge.to);
                }
            }
            println!();
            println!("Use --dot to output Graphviz format");
        }
    }

    Ok(())
}
