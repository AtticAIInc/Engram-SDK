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

#[cfg(test)]
mod tests {
    use super::*;
    use engram_core::model::*;
    use tempfile::TempDir;

    fn make_test_data(files: &[&str], agent: &str, commits: &[&str]) -> EngramData {
        EngramData {
            manifest: Manifest {
                id: EngramId::new(),
                version: 1,
                created_at: chrono::Utc::now(),
                finished_at: None,
                agent: AgentInfo {
                    name: agent.into(),
                    model: None,
                    version: None,
                },
                git_commits: commits.iter().map(|s| s.to_string()).collect(),
                token_usage: TokenUsage::default(),
                summary: Some("Test engram".into()),
                tags: vec![],
                capture_mode: CaptureMode::Sdk,
                source_hash: None,
            },
            intent: Intent {
                original_request: "test".into(),
                interpreted_goal: None,
                summary: None,
                dead_ends: vec![],
                decisions: vec![],
            },
            transcript: Transcript::default(),
            operations: Operations {
                tool_calls: vec![],
                file_changes: files
                    .iter()
                    .map(|f| FileChange {
                        path: f.to_string(),
                        change_type: FileChangeType::Modified,
                        lines_added: None,
                        lines_removed: None,
                    })
                    .collect(),
                shell_commands: vec![],
            },
            lineage: Lineage {
                git_commits: commits.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            },
        }
    }

    #[test]
    fn test_build_graph_empty() {
        let tmp = TempDir::new().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        let storage = GitStorage::open(tmp.path()).unwrap();
        storage.init().unwrap();

        let graph = build_graph(&storage).unwrap();
        assert!(graph.nodes.is_empty());
        assert!(graph.edges.is_empty());
    }

    #[test]
    fn test_build_graph_single_engram() {
        let tmp = TempDir::new().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        let storage = GitStorage::open(tmp.path()).unwrap();
        storage.init().unwrap();

        let data = make_test_data(&["src/main.rs"], "claude-code", &["abc12345"]);
        storage.create(&data).unwrap();

        let graph = build_graph(&storage).unwrap();

        // Should have: 1 engram + 1 file + 1 agent + 1 commit = 4 nodes
        assert_eq!(graph.nodes.len(), 4);
        assert!(graph.nodes.iter().any(|n| n.node_type == NodeType::Engram));
        assert!(graph.nodes.iter().any(|n| n.node_type == NodeType::File));
        assert!(graph.nodes.iter().any(|n| n.node_type == NodeType::Agent));
        assert!(graph.nodes.iter().any(|n| n.node_type == NodeType::Commit));

        // Should have edges: touched_file, modified_by, used_agent, produced_by
        assert!(graph
            .edges
            .iter()
            .any(|e| e.edge_type == EdgeType::TouchedFile));
        assert!(graph
            .edges
            .iter()
            .any(|e| e.edge_type == EdgeType::ModifiedBy));
        assert!(graph
            .edges
            .iter()
            .any(|e| e.edge_type == EdgeType::UsedAgent));
        assert!(graph
            .edges
            .iter()
            .any(|e| e.edge_type == EdgeType::ProducedBy));
    }

    #[test]
    fn test_build_graph_shared_files_deduplicated() {
        let tmp = TempDir::new().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        let storage = GitStorage::open(tmp.path()).unwrap();
        storage.init().unwrap();

        // Two engrams touching the same file
        let data1 = make_test_data(&["src/shared.rs"], "claude-code", &[]);
        let data2 = make_test_data(&["src/shared.rs"], "claude-code", &[]);
        storage.create(&data1).unwrap();
        storage.create(&data2).unwrap();

        let graph = build_graph(&storage).unwrap();

        // File node should be deduplicated
        let file_nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| n.node_type == NodeType::File)
            .collect();
        assert_eq!(file_nodes.len(), 1);

        // Agent node should be deduplicated
        let agent_nodes: Vec<_> = graph
            .nodes
            .iter()
            .filter(|n| n.node_type == NodeType::Agent)
            .collect();
        assert_eq!(agent_nodes.len(), 1);
    }
}
