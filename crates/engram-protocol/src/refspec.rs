use git2::Repository;

use crate::error::ProtocolError;

/// Refspec for fetching engram refs from remotes.
pub const ENGRAM_FETCH_REFSPEC: &str = "+refs/engrams/*:refs/engrams/*";

/// Refspec for pushing engram refs to remotes (force-push, since engrams may be
/// updated in-place when commit SHAs are appended).
pub const ENGRAM_PUSH_REFSPEC: &str = "+refs/engrams/*:refs/engrams/*";

/// Refspec for fetching engram notes from remotes.
pub const NOTES_FETCH_REFSPEC: &str = "+refs/notes/engram:refs/notes/engram";

/// Refspec for pushing engram notes to remotes (force-push).
pub const NOTES_PUSH_REFSPEC: &str = "+refs/notes/engram:refs/notes/engram";

/// Ensure the engram fetch refspecs are configured for a remote.
///
/// Only fetch refspecs are persisted on the remote config (so `git fetch` pulls engram
/// refs automatically). Push refspecs are NOT persisted — they would cause every
/// `git push` to implicitly push engram refs, conflicting with the pre-push hook's
/// explicit push logic. Instead, push refspecs are passed directly to `remote.push()`.
pub fn ensure_refspecs(repo: &Repository, remote_name: &str) -> Result<bool, ProtocolError> {
    let remote = repo
        .find_remote(remote_name)
        .map_err(|_| ProtocolError::RemoteNotFound(remote_name.into()))?;

    let mut needs_fetch = true;
    let mut needs_notes_fetch = true;

    // Check existing fetch refspecs
    if let Ok(refspecs) = remote.fetch_refspecs() {
        for i in 0..refspecs.len() {
            if let Some(spec) = refspecs.get(i) {
                if spec == ENGRAM_FETCH_REFSPEC {
                    needs_fetch = false;
                }
                if spec == NOTES_FETCH_REFSPEC {
                    needs_notes_fetch = false;
                }
            }
        }
    }

    // Clean up any stale push refspecs that were previously persisted.
    // These cause `git push` to implicitly push engram refs, which conflicts
    // with the pre-push hook's explicit auto-push mechanism.
    let stale_push_refspecs: Vec<String> = {
        let mut stale = Vec::new();
        if let Ok(refspecs) = remote.push_refspecs() {
            for i in 0..refspecs.len() {
                if let Some(spec) = refspecs.get(i) {
                    if spec.contains("refs/engrams/") || spec.contains("refs/notes/engram") {
                        stale.push(spec.to_string());
                    }
                }
            }
        }
        stale
    };

    drop(remote);

    let mut changed = false;

    if needs_fetch {
        repo.remote_add_fetch(remote_name, ENGRAM_FETCH_REFSPEC)?;
        changed = true;
    }
    if needs_notes_fetch {
        repo.remote_add_fetch(remote_name, NOTES_FETCH_REFSPEC)?;
        changed = true;
    }

    // Remove stale push refspecs from the git config
    if !stale_push_refspecs.is_empty() {
        if let Ok(mut config) = repo.config() {
            let key = format!("remote.{remote_name}.push");
            for spec in &stale_push_refspecs {
                let _ = config.remove_multivar(&key, &regex_escape(spec));
            }
        }
        changed = true;
    }

    Ok(changed)
}

/// Escape a string for use as a regex pattern (for git config multivar removal).
fn regex_escape(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        match c {
            '.' | '*' | '+' | '?' | '(' | ')' | '[' | ']' | '{' | '}' | '\\' | '^' | '$' | '|' => {
                escaped.push('\\');
                escaped.push(c);
            }
            _ => escaped.push(c),
        }
    }
    escaped
}

/// Ensure refspecs for all remotes in the repository.
pub fn ensure_all_refspecs(repo: &Repository) -> Result<Vec<String>, ProtocolError> {
    let remotes = repo.remotes()?;
    let mut configured = Vec::new();

    for i in 0..remotes.len() {
        if let Some(name) = remotes.get(i) {
            if ensure_refspecs(repo, name)? {
                configured.push(name.to_string());
            }
        }
    }

    Ok(configured)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

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
    fn test_ensure_refspecs_adds_fetch_only() {
        let (_local, _remote, repo) = make_repo_with_remote();

        let changed = ensure_refspecs(&repo, "origin").unwrap();
        assert!(changed, "Should report changes on first call");

        // Verify fetch refspec was added
        let remote = repo.find_remote("origin").unwrap();
        let fetch_specs = remote.fetch_refspecs().unwrap();
        let has_engram_fetch =
            (0..fetch_specs.len()).any(|i| fetch_specs.get(i) == Some(ENGRAM_FETCH_REFSPEC));
        assert!(has_engram_fetch, "Fetch refspec should be configured");

        // Push refspecs should NOT be persisted on the remote config
        let push_specs = remote.push_refspecs().unwrap();
        let has_engram_push =
            (0..push_specs.len()).any(|i| push_specs.get(i) == Some(ENGRAM_PUSH_REFSPEC));
        assert!(
            !has_engram_push,
            "Push refspec should NOT be persisted on remote"
        );
    }

    #[test]
    fn test_ensure_refspecs_idempotent() {
        let (_local, _remote, repo) = make_repo_with_remote();

        let first = ensure_refspecs(&repo, "origin").unwrap();
        assert!(first);

        let second = ensure_refspecs(&repo, "origin").unwrap();
        assert!(!second, "Second call should report no changes");
    }

    #[test]
    fn test_ensure_refspecs_remote_not_found() {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();

        let result = ensure_refspecs(&repo, "nonexistent");
        assert!(result.is_err());
        match result.unwrap_err() {
            ProtocolError::RemoteNotFound(name) => assert_eq!(name, "nonexistent"),
            other => panic!("Expected RemoteNotFound, got: {other:?}"),
        }
    }

    #[test]
    fn test_ensure_all_refspecs_multiple_remotes() {
        let remote1 = TempDir::new().unwrap();
        Repository::init_bare(remote1.path()).unwrap();
        let remote2 = TempDir::new().unwrap();
        Repository::init_bare(remote2.path()).unwrap();

        let local = TempDir::new().unwrap();
        let repo = Repository::init(local.path()).unwrap();
        repo.remote("origin", remote1.path().to_str().unwrap())
            .unwrap();
        repo.remote("upstream", remote2.path().to_str().unwrap())
            .unwrap();

        let configured = ensure_all_refspecs(&repo).unwrap();
        assert_eq!(configured.len(), 2);
        assert!(configured.contains(&"origin".to_string()));
        assert!(configured.contains(&"upstream".to_string()));
    }

    #[test]
    fn test_ensure_all_refspecs_no_remotes() {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();

        let configured = ensure_all_refspecs(&repo).unwrap();
        assert!(configured.is_empty());
    }
}
