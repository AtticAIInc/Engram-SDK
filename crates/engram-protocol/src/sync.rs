use git2::Repository;

use crate::error::ProtocolError;
use crate::refspec::{ensure_refspecs, ENGRAM_FETCH_REFSPEC, ENGRAM_PUSH_REFSPEC};

/// Options for push/fetch operations.
#[derive(Debug, Default)]
pub struct SyncOptions {
    /// Only sync these specific engram ref patterns (empty = all).
    pub refspecs: Vec<String>,
    /// Dry run — don't actually transfer data.
    pub dry_run: bool,
}

/// Result of a push operation.
#[derive(Debug)]
pub struct PushResult {
    pub remote: String,
    pub refs_pushed: usize,
}

/// Result of a fetch operation.
#[derive(Debug)]
pub struct FetchResult {
    pub remote: String,
    pub refs_fetched: usize,
}

/// Push engram refs to a remote.
pub fn push_engrams(
    repo: &Repository,
    remote_name: &str,
    opts: &SyncOptions,
) -> Result<PushResult, ProtocolError> {
    ensure_refspecs(repo, remote_name)?;

    let refspecs = if opts.refspecs.is_empty() {
        vec![ENGRAM_PUSH_REFSPEC.to_string()]
    } else {
        opts.refspecs.clone()
    };

    let refs_before = engram_core::storage::refs::list_engram_refs(repo)?;

    if opts.dry_run {
        // In dry-run, report how many refs exist that would be pushed
        return Ok(PushResult {
            remote: remote_name.into(),
            refs_pushed: refs_before.len(),
        });
    }

    let mut remote = repo
        .find_remote(remote_name)
        .map_err(|_| ProtocolError::RemoteNotFound(remote_name.into()))?;

    let refspec_strs: Vec<&str> = refspecs.iter().map(|s| s.as_str()).collect();

    remote
        .push(&refspec_strs, None)
        .map_err(|e| ProtocolError::Sync(format!("Push failed: {e}")))?;

    // Count how many refs were actually pushed (delta)
    let refs_after = engram_core::storage::refs::list_engram_refs(repo)?;
    let new_refs = refs_after.len().saturating_sub(refs_before.len());
    // If no new refs were created locally during push, report the count of refs
    // that existed before (all were pushed/synced)
    let pushed = if new_refs == 0 {
        refs_before.len()
    } else {
        new_refs
    };

    Ok(PushResult {
        remote: remote_name.into(),
        refs_pushed: pushed,
    })
}

/// Fetch engram refs from a remote.
///
/// Fetches engram refs from the named remote. In dry-run mode, returns immediately
/// with zero refs fetched.
pub fn fetch_engrams(
    repo: &Repository,
    remote_name: &str,
    opts: &SyncOptions,
) -> Result<FetchResult, ProtocolError> {
    ensure_refspecs(repo, remote_name)?;

    let refspecs = if opts.refspecs.is_empty() {
        vec![ENGRAM_FETCH_REFSPEC.to_string()]
    } else {
        opts.refspecs.clone()
    };

    if opts.dry_run {
        return Ok(FetchResult {
            remote: remote_name.into(),
            refs_fetched: 0,
        });
    }

    let refs_before = engram_core::storage::refs::list_engram_refs(repo)?;

    let mut remote = repo
        .find_remote(remote_name)
        .map_err(|_| ProtocolError::RemoteNotFound(remote_name.into()))?;

    let refspec_strs: Vec<&str> = refspecs.iter().map(|s| s.as_str()).collect();

    remote
        .fetch(&refspec_strs, None, None)
        .map_err(|e| ProtocolError::Sync(format!("Fetch failed: {e}")))?;

    let refs_after = engram_core::storage::refs::list_engram_refs(repo)?;
    let new_refs = refs_after.len().saturating_sub(refs_before.len());

    Ok(FetchResult {
        remote: remote_name.into(),
        refs_fetched: new_refs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_core::model::*;
    use engram_core::storage::GitStorage;
    use tempfile::TempDir;

    fn make_test_data() -> EngramData {
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
            operations: Operations::default(),
            lineage: Lineage::default(),
        }
    }

    fn make_repo_with_remote() -> (TempDir, TempDir, Repository) {
        let remote_tmp = TempDir::new().unwrap();
        Repository::init_bare(remote_tmp.path()).unwrap();

        let local_tmp = TempDir::new().unwrap();
        let repo = Repository::init(local_tmp.path()).unwrap();
        repo.remote("origin", remote_tmp.path().to_str().unwrap())
            .unwrap();
        (local_tmp, remote_tmp, repo)
    }

    #[test]
    fn test_push_dry_run_counts_refs() {
        let (local_tmp, _remote_tmp, _repo) = make_repo_with_remote();
        let storage = GitStorage::open(local_tmp.path()).unwrap();
        storage.init().unwrap();

        // Create an engram
        storage.create(&make_test_data()).unwrap();

        let repo = storage.repo();
        let opts = SyncOptions {
            dry_run: true,
            ..Default::default()
        };
        let result = push_engrams(repo, "origin", &opts).unwrap();
        assert_eq!(result.remote, "origin");
        assert_eq!(result.refs_pushed, 1);
    }

    #[test]
    fn test_push_dry_run_zero_refs() {
        let (local_tmp, _remote_tmp, _repo) = make_repo_with_remote();
        let storage = GitStorage::open(local_tmp.path()).unwrap();
        storage.init().unwrap();

        let repo = storage.repo();
        let opts = SyncOptions {
            dry_run: true,
            ..Default::default()
        };
        let result = push_engrams(repo, "origin", &opts).unwrap();
        assert_eq!(result.refs_pushed, 0);
    }

    #[test]
    fn test_fetch_dry_run() {
        let (local_tmp, _remote_tmp, _repo) = make_repo_with_remote();
        let storage = GitStorage::open(local_tmp.path()).unwrap();
        storage.init().unwrap();

        let repo = storage.repo();
        let opts = SyncOptions {
            dry_run: true,
            ..Default::default()
        };
        let result = fetch_engrams(repo, "origin", &opts).unwrap();
        assert_eq!(result.remote, "origin");
        assert_eq!(result.refs_fetched, 0);
    }

    #[test]
    fn test_push_remote_not_found() {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();
        // No remote configured

        let opts = SyncOptions::default();
        let result = push_engrams(&repo, "nonexistent", &opts);
        assert!(result.is_err());
    }

    #[test]
    fn test_fetch_remote_not_found() {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();

        let opts = SyncOptions::default();
        let result = fetch_engrams(&repo, "nonexistent", &opts);
        assert!(result.is_err());
    }

    #[test]
    fn test_push_and_fetch_local() {
        let remote_tmp = TempDir::new().unwrap();
        Repository::init_bare(remote_tmp.path()).unwrap();

        // Create local repo with a remote
        let local_tmp = TempDir::new().unwrap();
        let local_repo = Repository::init(local_tmp.path()).unwrap();
        local_repo
            .remote("origin", remote_tmp.path().to_str().unwrap())
            .unwrap();

        let storage = GitStorage::open(local_tmp.path()).unwrap();
        storage.init().unwrap();

        // Create an engram
        let data = make_test_data();
        let id = data.manifest.id.clone();
        storage.create(&data).unwrap();

        // Push to remote using explicit ref (git2 doesn't expand globs with no existing refs on remote)
        let ref_name = format!("refs/engrams/{}/{}", id.fanout_prefix(), id.as_str());
        let push_refspec = format!("{ref_name}:{ref_name}");
        let push_opts = SyncOptions {
            refspecs: vec![push_refspec],
            ..Default::default()
        };
        let push_result = push_engrams(storage.repo(), "origin", &push_opts).unwrap();
        assert_eq!(push_result.refs_pushed, 1);

        // Create a second local repo to fetch into
        let fetch_tmp = TempDir::new().unwrap();
        let fetch_repo = Repository::init(fetch_tmp.path()).unwrap();
        fetch_repo
            .remote("origin", remote_tmp.path().to_str().unwrap())
            .unwrap();

        let fetch_opts = SyncOptions::default();
        let fetch_result = fetch_engrams(&fetch_repo, "origin", &fetch_opts).unwrap();
        assert_eq!(fetch_result.refs_fetched, 1);

        // Verify the fetched ref exists
        let fetch_storage = GitStorage::open(fetch_tmp.path()).unwrap();
        let loaded = fetch_storage.read(id.as_str()).unwrap();
        assert_eq!(loaded.manifest.id, id);
    }
}
