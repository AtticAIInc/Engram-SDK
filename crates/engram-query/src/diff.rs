use std::collections::HashSet;

use engram_core::model::{EngramData, EngramId};
use engram_core::storage::GitStorage;

use crate::error::QueryError;

/// Differences between two engrams.
#[derive(Debug)]
pub struct EngramDiff {
    pub id_a: EngramId,
    pub id_b: EngramId,
    pub common_files: Vec<String>,
    pub only_a_files: Vec<String>,
    pub only_b_files: Vec<String>,
    pub token_delta: i64,
    pub cost_delta: Option<f64>,
}

/// Compare two engrams.
pub fn diff_engrams(
    storage: &GitStorage,
    id_a: &EngramId,
    id_b: &EngramId,
) -> Result<EngramDiff, QueryError> {
    let data_a = storage.read(id_a.as_str())?;
    let data_b = storage.read(id_b.as_str())?;

    compute_diff(id_a, id_b, &data_a, &data_b)
}

fn compute_diff(
    id_a: &EngramId,
    id_b: &EngramId,
    data_a: &EngramData,
    data_b: &EngramData,
) -> Result<EngramDiff, QueryError> {
    let files_a: HashSet<&str> = data_a
        .operations
        .file_changes
        .iter()
        .map(|f| f.path.as_str())
        .collect();
    let files_b: HashSet<&str> = data_b
        .operations
        .file_changes
        .iter()
        .map(|f| f.path.as_str())
        .collect();

    let common: Vec<String> = files_a
        .intersection(&files_b)
        .map(|s| s.to_string())
        .collect();
    let only_a: Vec<String> = files_a
        .difference(&files_b)
        .map(|s| s.to_string())
        .collect();
    let only_b: Vec<String> = files_b
        .difference(&files_a)
        .map(|s| s.to_string())
        .collect();

    let token_delta = data_b.manifest.token_usage.total_tokens as i64
        - data_a.manifest.token_usage.total_tokens as i64;

    let cost_delta = match (
        data_a.manifest.token_usage.cost_usd,
        data_b.manifest.token_usage.cost_usd,
    ) {
        (Some(a), Some(b)) => Some(b - a),
        _ => None,
    };

    Ok(EngramDiff {
        id_a: id_a.clone(),
        id_b: id_b.clone(),
        common_files: common,
        only_a_files: only_a,
        only_b_files: only_b,
        token_delta,
        cost_delta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_core::model::*;

    fn make_test_data(files: &[&str], tokens: u64, cost: Option<f64>) -> EngramData {
        EngramData {
            manifest: Manifest {
                id: EngramId::new(),
                version: 1,
                created_at: chrono::Utc::now(),
                finished_at: None,
                agent: AgentInfo {
                    name: "test".into(),
                    model: None,
                    version: None,
                },
                git_commits: Vec::new(),
                token_usage: TokenUsage {
                    total_tokens: tokens,
                    cost_usd: cost,
                    ..Default::default()
                },
                summary: None,
                tags: Vec::new(),
                capture_mode: CaptureMode::Import,
                source_hash: None,
            },
            intent: Intent {
                original_request: "test".into(),
                interpreted_goal: None,
                summary: None,
                dead_ends: Vec::new(),
                decisions: Vec::new(),
            },
            transcript: Transcript {
                entries: Vec::new(),
            },
            operations: Operations {
                tool_calls: Vec::new(),
                file_changes: files
                    .iter()
                    .map(|f| FileChange {
                        path: f.to_string(),
                        change_type: FileChangeType::Modified,
                        lines_added: None,
                        lines_removed: None,
                    })
                    .collect(),
                shell_commands: Vec::new(),
            },
            lineage: Lineage::default(),
        }
    }

    #[test]
    fn test_diff_engrams() {
        let id_a = EngramId::new();
        let id_b = EngramId::new();
        let data_a = make_test_data(&["src/main.rs", "src/lib.rs"], 1000, Some(0.01));
        let data_b = make_test_data(&["src/main.rs", "tests/test.rs"], 2000, Some(0.03));

        let diff = compute_diff(&id_a, &id_b, &data_a, &data_b).unwrap();

        assert_eq!(diff.common_files, vec!["src/main.rs"]);
        assert!(diff.only_a_files.contains(&"src/lib.rs".to_string()));
        assert!(diff.only_b_files.contains(&"tests/test.rs".to_string()));
        assert_eq!(diff.token_delta, 1000);
        let cost = diff.cost_delta.unwrap();
        assert!((cost - 0.02).abs() < 1e-10, "cost_delta was {cost}");
    }
}
