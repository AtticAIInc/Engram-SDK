use engram_core::model::Manifest;
use engram_core::storage::GitStorage;

use crate::error::QueryError;

/// A single engram found during a branch review.
#[derive(Debug, Clone)]
pub struct ReviewEntry {
    pub manifest: Manifest,
    pub commit_sha: String,
}

/// Result of reviewing a branch range.
#[derive(Debug)]
pub struct BranchReview {
    pub range: String,
    pub engrams: Vec<ReviewEntry>,
    pub total_commits: usize,
    pub total_tokens: u64,
    pub total_cost: Option<f64>,
    pub files_changed: Vec<String>,
}

/// Review a branch by walking git log for `base..head`, finding commits
/// with `Engram-Id` trailers, and collecting referenced engrams.
pub fn review_branch(
    storage: &GitStorage,
    base: &str,
    head: &str,
) -> Result<BranchReview, QueryError> {
    let repo = storage.repo();
    let range = format!("{base}..{head}");

    // Resolve base and head
    let head_obj = repo
        .revparse_single(head)
        .map_err(|e| QueryError::Search(format!("Cannot resolve '{head}': {e}")))?;
    let base_obj = repo
        .revparse_single(base)
        .map_err(|e| QueryError::Search(format!("Cannot resolve '{base}': {e}")))?;

    // Walk from head to base
    let mut revwalk = repo
        .revwalk()
        .map_err(|e| QueryError::Search(format!("Cannot create revwalk: {e}")))?;
    revwalk
        .push(head_obj.id())
        .map_err(|e| QueryError::Search(format!("Cannot push head: {e}")))?;
    revwalk
        .hide(base_obj.id())
        .map_err(|e| QueryError::Search(format!("Cannot hide base: {e}")))?;

    let mut engrams = Vec::new();
    let mut total_commits = 0;
    let mut seen_engram_ids = std::collections::HashSet::new();
    let mut all_files = std::collections::HashSet::new();

    for oid_result in revwalk {
        let oid = oid_result.map_err(|e| QueryError::Search(format!("Revwalk error: {e}")))?;
        let commit = repo
            .find_commit(oid)
            .map_err(|e| QueryError::Search(format!("Cannot find commit {oid}: {e}")))?;

        total_commits += 1;
        let sha = oid.to_string();

        // Check commit message for Engram-Id trailer
        if let Some(message) = commit.message() {
            for line in message.lines() {
                if let Some(engram_id) = line.strip_prefix("Engram-Id: ") {
                    let engram_id = engram_id.trim();
                    if seen_engram_ids.insert(engram_id.to_string()) {
                        // Try to read the engram
                        if let Ok(data) = storage.read(engram_id) {
                            // Collect files
                            for fc in &data.operations.file_changes {
                                all_files.insert(fc.path.clone());
                            }
                            engrams.push(ReviewEntry {
                                manifest: data.manifest,
                                commit_sha: sha.clone(),
                            });
                        }
                    }
                }
            }
        }
    }

    // Compute totals
    let total_tokens: u64 = engrams
        .iter()
        .map(|e| e.manifest.token_usage.total_tokens)
        .sum();

    let total_cost: Option<f64> = {
        let costs: Vec<f64> = engrams
            .iter()
            .filter_map(|e| {
                e.manifest
                    .token_usage
                    .effective_cost(e.manifest.agent.model.as_deref())
            })
            .collect();
        if costs.is_empty() {
            None
        } else {
            Some(costs.iter().sum())
        }
    };

    Ok(BranchReview {
        range,
        engrams,
        total_commits,
        total_tokens,
        total_cost,
        files_changed: all_files.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_core::model::*;
    use engram_core::storage::GitStorage;
    use git2::{Repository, Signature};
    use tempfile::TempDir;

    fn make_test_data(request: &str, files: &[&str], tokens: u64, cost: Option<f64>) -> EngramData {
        EngramData {
            manifest: Manifest {
                id: EngramId::new(),
                version: 1,
                created_at: chrono::Utc::now(),
                finished_at: None,
                agent: AgentInfo {
                    name: "test-agent".into(),
                    model: None,
                    version: None,
                },
                git_commits: vec![],
                token_usage: TokenUsage {
                    total_tokens: tokens,
                    cost_usd: cost,
                    ..Default::default()
                },
                summary: Some(request.into()),
                tags: vec![],
                capture_mode: CaptureMode::Sdk,
                source_hash: None,
            },
            intent: Intent {
                original_request: request.into(),
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
            lineage: Lineage::default(),
        }
    }

    /// Create a commit in the repo with an optional Engram-Id trailer.
    fn make_commit(repo: &Repository, message: &str, parent: Option<git2::Oid>) -> git2::Oid {
        let sig = Signature::now("Test", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        let tree = repo.find_tree(tree_id).unwrap();

        if let Some(parent_oid) = parent {
            let parent_commit = repo.find_commit(parent_oid).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent_commit])
                .unwrap()
        } else {
            repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
                .unwrap()
        }
    }

    #[test]
    fn test_review_branch_empty() {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }

        let storage = GitStorage::open(tmp.path()).unwrap();
        storage.init().unwrap();

        // Create base and head commit with no trailer
        let base = make_commit(&repo, "base commit", None);
        let head = make_commit(&repo, "head commit", Some(base));

        let review = review_branch(&storage, &base.to_string(), &head.to_string()).unwrap();

        assert_eq!(review.total_commits, 1);
        assert!(review.engrams.is_empty());
        assert_eq!(review.total_tokens, 0);
    }

    #[test]
    fn test_review_branch_with_engram() {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }

        let storage = GitStorage::open(tmp.path()).unwrap();
        storage.init().unwrap();

        // Create an engram
        let data = make_test_data("Add auth", &["src/auth.rs"], 5000, Some(0.10));
        let id = storage.create(&data).unwrap();

        // Create commits: base, then one with an Engram-Id trailer
        let base = make_commit(&repo, "base commit", None);
        let msg = format!("Add authentication\n\nEngram-Id: {}", id.as_str());
        let head = make_commit(&repo, &msg, Some(base));

        let review = review_branch(&storage, &base.to_string(), &head.to_string()).unwrap();

        assert_eq!(review.total_commits, 1);
        assert_eq!(review.engrams.len(), 1);
        assert_eq!(review.total_tokens, 5000);
        let cost = review.total_cost.unwrap();
        assert!((cost - 0.10).abs() < 1e-10);
        assert!(review.files_changed.contains(&"src/auth.rs".to_string()));
    }

    #[test]
    fn test_review_deduplicates_engram_ids() {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }

        let storage = GitStorage::open(tmp.path()).unwrap();
        storage.init().unwrap();

        let data = make_test_data("Add auth", &["src/auth.rs"], 1000, None);
        let id = storage.create(&data).unwrap();

        let base = make_commit(&repo, "base commit", None);
        let msg1 = format!("commit 1\n\nEngram-Id: {}", id.as_str());
        let mid = make_commit(&repo, &msg1, Some(base));
        let msg2 = format!("commit 2\n\nEngram-Id: {}", id.as_str());
        let head = make_commit(&repo, &msg2, Some(mid));

        let review = review_branch(&storage, &base.to_string(), &head.to_string()).unwrap();

        // Same engram ID should only appear once
        assert_eq!(review.engrams.len(), 1);
        assert_eq!(review.total_commits, 2);
    }
}
