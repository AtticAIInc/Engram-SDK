use git2::{Oid, Repository, Signature};

use crate::error::CoreError;
use crate::model::EngramData;

/// Build the engram tree object from EngramData.
///
/// Creates blobs for each file, inserts them into a TreeBuilder, writes the tree,
/// then creates a commit pointing to that tree. Returns the commit Oid.
///
/// Object layout:
///   commit (message = "engram: {id}")
///     -> tree
///        -> blob "manifest.json"
///        -> blob "intent.md"
///        -> blob "transcript.jsonl"
///        -> blob "operations.json"
///        -> blob "lineage.json"
pub fn create_engram_objects(repo: &Repository, data: &EngramData) -> Result<Oid, CoreError> {
    // 1. Serialize each component to bytes
    let manifest_bytes = serde_json::to_vec_pretty(&data.manifest)?;
    let intent_bytes = data.intent.to_markdown().into_bytes();
    let transcript_bytes = data.transcript.to_jsonl()?;
    let operations_bytes = serde_json::to_vec_pretty(&data.operations)?;
    let lineage_bytes = serde_json::to_vec_pretty(&data.lineage)?;

    // 2. Create blobs
    let manifest_oid = repo.blob(&manifest_bytes)?;
    let intent_oid = repo.blob(&intent_bytes)?;
    let transcript_oid = repo.blob(&transcript_bytes)?;
    let operations_oid = repo.blob(&operations_bytes)?;
    let lineage_oid = repo.blob(&lineage_bytes)?;

    // 3. Build tree
    let mut builder = repo.treebuilder(None)?;
    builder.insert("manifest.json", manifest_oid, 0o100644)?;
    builder.insert("intent.md", intent_oid, 0o100644)?;
    builder.insert("transcript.jsonl", transcript_oid, 0o100644)?;
    builder.insert("operations.json", operations_oid, 0o100644)?;
    builder.insert("lineage.json", lineage_oid, 0o100644)?;
    let tree_oid = builder.write()?;

    // 4. Create commit (no parent — standalone orphan)
    let tree = repo.find_tree(tree_oid)?;
    let sig = Signature::now("engram", "engram@local")?;
    let message = format!("engram: {}", data.manifest.id);
    let commit_oid = repo.commit(None, &sig, &sig, &message, &tree, &[])?;

    Ok(commit_oid)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use chrono::Utc;
    use tempfile::TempDir;

    fn make_test_engram_data() -> EngramData {
        EngramData {
            manifest: Manifest {
                id: EngramId::new(),
                version: 1,
                created_at: Utc::now(),
                finished_at: None,
                agent: AgentInfo {
                    name: "test-agent".into(),
                    model: Some("test-model".into()),
                    version: None,
                },
                git_commits: vec![],
                token_usage: TokenUsage::default(),
                summary: Some("Test engram".into()),
                tags: vec![],
                capture_mode: CaptureMode::Sdk,
                source_hash: None,
            },
            intent: Intent {
                original_request: "Test request".into(),
                interpreted_goal: None,
                summary: Some("Test summary".into()),
                dead_ends: vec![],
                decisions: vec![],
            },
            transcript: Transcript::default(),
            operations: Operations::default(),
            lineage: Lineage::default(),
        }
    }

    #[test]
    fn test_create_engram_objects() {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();
        let data = make_test_engram_data();

        let commit_oid = create_engram_objects(&repo, &data).unwrap();

        // Verify the commit exists
        let commit = repo.find_commit(commit_oid).unwrap();
        assert!(commit.message().unwrap().contains("engram:"));

        // Verify the tree has 5 entries
        let tree = commit.tree().unwrap();
        assert_eq!(tree.len(), 5);
        assert!(tree.get_name("manifest.json").is_some());
        assert!(tree.get_name("intent.md").is_some());
        assert!(tree.get_name("transcript.jsonl").is_some());
        assert!(tree.get_name("operations.json").is_some());
        assert!(tree.get_name("lineage.json").is_some());
    }
}
