use serde::{Deserialize, Serialize};

/// Type of node in the context graph.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeType {
    Engram,
    File,
    Agent,
    Commit,
}

/// A node in the context graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub node_type: NodeType,
    pub label: String,
}

/// Type of edge in the context graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EdgeType {
    ModifiedBy,
    ProducedBy,
    UsedAgent,
    FollowsFrom,
    TouchedFile,
}

/// An edge in the context graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub from: String,
    pub to: String,
    pub edge_type: EdgeType,
}

/// The full context graph.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
}

impl ContextGraph {
    /// Extract a subgraph centered on a node, up to a given depth.
    pub fn subgraph(&self, center_id: &str, depth: usize) -> ContextGraph {
        use std::collections::{HashSet, VecDeque};

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((center_id.to_string(), 0));
        visited.insert(center_id.to_string());

        while let Some((current, d)) = queue.pop_front() {
            if d >= depth {
                continue;
            }
            for edge in &self.edges {
                let neighbor = if edge.from == current {
                    &edge.to
                } else if edge.to == current {
                    &edge.from
                } else {
                    continue;
                };
                if visited.insert(neighbor.clone()) {
                    queue.push_back((neighbor.clone(), d + 1));
                }
            }
        }

        let nodes: Vec<GraphNode> = self
            .nodes
            .iter()
            .filter(|n| visited.contains(&n.id))
            .cloned()
            .collect();
        let edges: Vec<GraphEdge> = self
            .edges
            .iter()
            .filter(|e| visited.contains(&e.from) && visited.contains(&e.to))
            .cloned()
            .collect();

        ContextGraph { nodes, edges }
    }

    /// Render as DOT format for Graphviz.
    pub fn to_dot(&self) -> String {
        let mut dot = String::from("digraph engram {\n  rankdir=LR;\n");

        for node in &self.nodes {
            let shape = match node.node_type {
                NodeType::Engram => "box",
                NodeType::File => "note",
                NodeType::Agent => "diamond",
                NodeType::Commit => "ellipse",
            };
            dot.push_str(&format!(
                "  \"{}\" [label=\"{}\" shape={}];\n",
                node.id, node.label, shape
            ));
        }

        for edge in &self.edges {
            let label = match edge.edge_type {
                EdgeType::ModifiedBy => "modified_by",
                EdgeType::ProducedBy => "produced_by",
                EdgeType::UsedAgent => "used_agent",
                EdgeType::FollowsFrom => "follows_from",
                EdgeType::TouchedFile => "touched_file",
            };
            dot.push_str(&format!(
                "  \"{}\" -> \"{}\" [label=\"{}\"];\n",
                edge.from, edge.to, label
            ));
        }

        dot.push_str("}\n");
        dot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_graph() -> ContextGraph {
        ContextGraph {
            nodes: vec![
                GraphNode {
                    id: "engram:abc123".into(),
                    node_type: NodeType::Engram,
                    label: "Add auth".into(),
                },
                GraphNode {
                    id: "file:src/auth.rs".into(),
                    node_type: NodeType::File,
                    label: "src/auth.rs".into(),
                },
                GraphNode {
                    id: "agent:claude-code".into(),
                    node_type: NodeType::Agent,
                    label: "claude-code".into(),
                },
                GraphNode {
                    id: "commit:def456".into(),
                    node_type: NodeType::Commit,
                    label: "def456".into(),
                },
                GraphNode {
                    id: "file:src/main.rs".into(),
                    node_type: NodeType::File,
                    label: "src/main.rs".into(),
                },
            ],
            edges: vec![
                GraphEdge {
                    from: "engram:abc123".into(),
                    to: "file:src/auth.rs".into(),
                    edge_type: EdgeType::TouchedFile,
                },
                GraphEdge {
                    from: "engram:abc123".into(),
                    to: "agent:claude-code".into(),
                    edge_type: EdgeType::UsedAgent,
                },
                GraphEdge {
                    from: "engram:abc123".into(),
                    to: "commit:def456".into(),
                    edge_type: EdgeType::ProducedBy,
                },
                GraphEdge {
                    from: "file:src/auth.rs".into(),
                    to: "engram:abc123".into(),
                    edge_type: EdgeType::ModifiedBy,
                },
            ],
        }
    }

    #[test]
    fn test_subgraph_depth_0() {
        let graph = make_test_graph();
        let sub = graph.subgraph("engram:abc123", 0);
        assert_eq!(sub.nodes.len(), 1);
        assert!(sub.edges.is_empty());
    }

    #[test]
    fn test_subgraph_depth_1() {
        let graph = make_test_graph();
        let sub = graph.subgraph("engram:abc123", 1);
        // Should include the engram + its direct neighbors
        assert!(sub.nodes.len() >= 4); // engram, file:auth, agent, commit
        assert!(!sub.edges.is_empty());
    }

    #[test]
    fn test_subgraph_nonexistent_center() {
        let graph = make_test_graph();
        let sub = graph.subgraph("nonexistent", 2);
        // Should contain only the "nonexistent" node, which doesn't actually exist in nodes
        assert!(sub.nodes.is_empty());
    }

    #[test]
    fn test_to_dot_contains_structure() {
        let graph = make_test_graph();
        let dot = graph.to_dot();

        assert!(dot.starts_with("digraph engram {"));
        assert!(dot.ends_with("}\n"));
        assert!(dot.contains("engram:abc123"));
        assert!(dot.contains("shape=box")); // Engram node
        assert!(dot.contains("shape=note")); // File node
        assert!(dot.contains("shape=diamond")); // Agent node
        assert!(dot.contains("shape=ellipse")); // Commit node
        assert!(dot.contains("touched_file"));
        assert!(dot.contains("used_agent"));
        assert!(dot.contains("produced_by"));
        assert!(dot.contains("modified_by"));
    }

    #[test]
    fn test_empty_graph_to_dot() {
        let graph = ContextGraph::default();
        let dot = graph.to_dot();
        assert!(dot.starts_with("digraph engram {"));
        assert!(dot.ends_with("}\n"));
    }

    #[test]
    fn test_subgraph_bidirectional_edges() {
        let graph = make_test_graph();
        // From file:src/auth.rs at depth 1, should reach engram:abc123
        let sub = graph.subgraph("file:src/auth.rs", 1);
        assert!(sub.nodes.iter().any(|n| n.id == "engram:abc123"));
    }
}
