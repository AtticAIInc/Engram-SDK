use git2::{Oid, Repository};

use crate::error::CoreError;
use crate::model::{EngramData, Intent, Lineage, Manifest, Operations, Transcript};

/// Read an engram's data from its commit Oid.
pub fn read_engram(repo: &Repository, commit_oid: Oid) -> Result<EngramData, CoreError> {
    let commit = repo.find_commit(commit_oid)?;
    let tree = commit.tree()?;

    let manifest = read_blob_json::<Manifest>(repo, &tree, "manifest.json")?;
    let intent = Intent::from_markdown(&read_blob_string(repo, &tree, "intent.md")?)?;
    let transcript = Transcript::from_jsonl(&read_blob_bytes(repo, &tree, "transcript.jsonl")?)?;
    let operations = read_blob_json::<Operations>(repo, &tree, "operations.json")?;
    let lineage = read_blob_json::<Lineage>(repo, &tree, "lineage.json")?;

    Ok(EngramData {
        manifest,
        intent,
        transcript,
        operations,
        lineage,
    })
}

/// Read only the manifest (fast path for listing).
pub fn read_manifest(repo: &Repository, commit_oid: Oid) -> Result<Manifest, CoreError> {
    let commit = repo.find_commit(commit_oid)?;
    let tree = commit.tree()?;
    read_blob_json::<Manifest>(repo, &tree, "manifest.json")
}

fn read_blob_bytes(repo: &Repository, tree: &git2::Tree, name: &str) -> Result<Vec<u8>, CoreError> {
    let entry = tree
        .get_name(name)
        .ok_or_else(|| CoreError::MissingBlob(name.to_string()))?;
    let blob = repo.find_blob(entry.id())?;
    Ok(blob.content().to_vec())
}

fn read_blob_string(repo: &Repository, tree: &git2::Tree, name: &str) -> Result<String, CoreError> {
    let bytes = read_blob_bytes(repo, tree, name)?;
    String::from_utf8(bytes).map_err(CoreError::Utf8)
}

fn read_blob_json<T: serde::de::DeserializeOwned>(
    repo: &Repository,
    tree: &git2::Tree,
    name: &str,
) -> Result<T, CoreError> {
    let bytes = read_blob_bytes(repo, tree, name)?;
    serde_json::from_slice(&bytes).map_err(CoreError::InvalidManifest)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use crate::storage::objects::create_engram_objects;
    use chrono::Utc;
    use tempfile::TempDir;

    #[test]
    fn test_read_engram_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();

        let original = EngramData {
            manifest: Manifest {
                id: EngramId::new(),
                version: 1,
                created_at: Utc::now(),
                finished_at: None,
                agent: AgentInfo {
                    name: "claude-code".into(),
                    model: Some("claude-sonnet-4-5".into()),
                    version: None,
                },
                git_commits: vec!["abc123".into()],
                token_usage: TokenUsage {
                    input_tokens: 1000,
                    output_tokens: 500,
                    total_tokens: 1500,
                    cost_usd: Some(0.23),
                    ..Default::default()
                },
                summary: Some("Implemented auth".into()),
                tags: vec!["auth".into()],
                capture_mode: CaptureMode::Wrapper,
                source_hash: None,
            },
            intent: Intent {
                original_request: "Add OAuth2 authentication".into(),
                interpreted_goal: Some("Implement OAuth2 with PKCE".into()),
                summary: Some("Done".into()),
                dead_ends: vec![DeadEnd {
                    approach: "passport.js".into(),
                    reason: "Conflict".into(),
                }],
                decisions: vec![Decision {
                    description: "Custom middleware".into(),
                    rationale: "Full control".into(),
                }],
            },
            transcript: Transcript {
                entries: vec![TranscriptEntry {
                    timestamp: Utc::now(),
                    role: Role::User,
                    content: TranscriptContent::Text {
                        text: "Add OAuth2".into(),
                    },
                    token_count: None,
                }],
            },
            operations: Operations {
                file_changes: vec![FileChange {
                    path: "src/auth.rs".into(),
                    change_type: FileChangeType::Created,
                    lines_added: Some(50),
                    lines_removed: None,
                }],
                ..Default::default()
            },
            lineage: Lineage {
                git_commits: vec!["abc123".into()],
                branch: Some("main".into()),
                ..Default::default()
            },
        };

        // Store
        let commit_oid = create_engram_objects(&repo, &original).unwrap();

        // Read back
        let loaded = read_engram(&repo, commit_oid).unwrap();

        // Verify key fields
        assert_eq!(original.manifest.id, loaded.manifest.id);
        assert_eq!(original.manifest.agent.name, loaded.manifest.agent.name);
        assert_eq!(
            original.manifest.token_usage.input_tokens,
            loaded.manifest.token_usage.input_tokens
        );
        assert_eq!(
            original.intent.original_request,
            loaded.intent.original_request
        );
        assert_eq!(
            original.intent.dead_ends.len(),
            loaded.intent.dead_ends.len()
        );
        assert_eq!(
            original.transcript.entries.len(),
            loaded.transcript.entries.len()
        );
        assert_eq!(
            original.operations.file_changes.len(),
            loaded.operations.file_changes.len()
        );
        assert_eq!(original.lineage.branch, loaded.lineage.branch);
    }

    #[test]
    fn test_read_manifest_only() {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();

        let data = EngramData {
            manifest: Manifest {
                id: EngramId::new(),
                version: 1,
                created_at: Utc::now(),
                finished_at: None,
                agent: AgentInfo {
                    name: "test".into(),
                    model: None,
                    version: None,
                },
                git_commits: vec![],
                token_usage: TokenUsage::default(),
                summary: Some("Quick test".into()),
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
            operations: Operations::default(),
            lineage: Lineage::default(),
        };

        let commit_oid = create_engram_objects(&repo, &data).unwrap();
        let manifest = read_manifest(&repo, commit_oid).unwrap();
        assert_eq!(data.manifest.id, manifest.id);
        assert_eq!(data.manifest.summary, manifest.summary);
    }
}
