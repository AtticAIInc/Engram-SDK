use std::collections::HashSet;

use engram_core::storage::GitStorage;

use super::model::*;
use crate::error::QueryError;

/// Build a context graph from all engrams in storage.
pub fn build_graph(storage: &GitStorage) -> Result<ContextGraph, QueryError> {
    let manifests = storage.list(&Default::default())?;
    let mut graph = ContextGraph::default();
    let mut seen_agents = HashSet::new();
    let mut seen_files = HashSet::new();
    let mut seen_commits = HashSet::new();

    for manifest in &manifests {
        let data = match storage.read(manifest.id.as_str()) {
            Ok(d) => d,
            Err(e) => {
                tracing::warn!("Failed to read engram {}: {e}", manifest.id);
                continue;
            }
        };

        let engram_node_id = format!("engram:{}", manifest.id.as_str());

        // Add engram node
        graph.nodes.push(GraphNode {
            id: engram_node_id.clone(),
            node_type: NodeType::Engram,
            label: manifest
                .summary
                .clone()
                .unwrap_or_else(|| manifest.id.as_str()[..8].to_string()),
        });

        // Add agent node + edge
        let agent_id = format!("agent:{}", manifest.agent.name);
        if seen_agents.insert(agent_id.clone()) {
            graph.nodes.push(GraphNode {
                id: agent_id.clone(),
                node_type: NodeType::Agent,
                label: manifest.agent.name.clone(),
            });
        }
        graph.edges.push(GraphEdge {
            from: engram_node_id.clone(),
            to: agent_id,
            edge_type: EdgeType::UsedAgent,
        });

        // Add file nodes + edges
        for fc in &data.operations.file_changes {
            let file_id = format!("file:{}", fc.path);
            if seen_files.insert(file_id.clone()) {
                graph.nodes.push(GraphNode {
                    id: file_id.clone(),
                    node_type: NodeType::File,
                    label: fc.path.clone(),
                });
            }
            graph.edges.push(GraphEdge {
                from: engram_node_id.clone(),
                to: file_id.clone(),
                edge_type: EdgeType::TouchedFile,
            });
            graph.edges.push(GraphEdge {
                from: file_id,
                to: engram_node_id.clone(),
                edge_type: EdgeType::ModifiedBy,
            });
        }

        // Add commit nodes + edges
        for sha in &data.lineage.git_commits {
            let commit_id = format!("commit:{}", &sha[..std::cmp::min(8, sha.len())]);
            if seen_commits.insert(commit_id.clone()) {
                graph.nodes.push(GraphNode {
                    id: commit_id.clone(),
                    node_type: NodeType::Commit,
                    label: sha[..std::cmp::min(8, sha.len())].to_string(),
                });
            }
            graph.edges.push(GraphEdge {
                from: engram_node_id.clone(),
                to: commit_id,
                edge_type: EdgeType::ProducedBy,
            });
        }

        // Add lineage edges
        if let Some(parent) = &data.lineage.parent_engram {
            let parent_node_id = format!("engram:{}", parent.as_str());
            graph.edges.push(GraphEdge {
                from: engram_node_id,
                to: parent_node_id,
                edge_type: EdgeType::FollowsFrom,
            });
        }
    }

    Ok(graph)
}
