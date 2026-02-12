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
