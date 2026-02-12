use serde::{Deserialize, Serialize};

use super::engram::EngramId;

/// Relationships between this engram and other entities.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Lineage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_engram: Option<EngramId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub child_engrams: Vec<EngramId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_engrams: Vec<Relationship>,
    #[serde(default)]
    pub git_commits: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Relationship {
    pub engram_id: EngramId,
    pub relation_type: RelationType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RelationType {
    FollowsFrom,
    Motivates,
    DependsOn,
    Supersedes,
    ConflictsWith,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lineage_serde_roundtrip() {
        let lineage = Lineage {
            parent_engram: Some(EngramId("parent123".into())),
            child_engrams: vec![EngramId("child456".into())],
            related_engrams: vec![Relationship {
                engram_id: EngramId("related789".into()),
                relation_type: RelationType::FollowsFrom,
                description: Some("Previous auth attempt".into()),
            }],
            git_commits: vec!["abc123".into(), "def456".into()],
            branch: Some("feature/auth".into()),
        };
        let json = serde_json::to_string_pretty(&lineage).unwrap();
        let parsed: Lineage = serde_json::from_str(&json).unwrap();
        assert_eq!(lineage, parsed);
    }

    #[test]
    fn test_default_lineage() {
        let lineage = Lineage::default();
        let json = serde_json::to_string(&lineage).unwrap();
        // Default should produce minimal JSON
        assert!(!json.contains("parent_engram"));
        assert!(!json.contains("child_engrams"));
    }
}
