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
            .filter_map(|e| e.manifest.token_usage.cost_usd)
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
