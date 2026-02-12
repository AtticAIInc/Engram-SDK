use git2::Repository;

use crate::error::ProtocolError;

/// Refspec for fetching engram refs from remotes.
pub const ENGRAM_FETCH_REFSPEC: &str = "+refs/engrams/*:refs/engrams/*";

/// Refspec for pushing engram refs to remotes.
pub const ENGRAM_PUSH_REFSPEC: &str = "refs/engrams/*:refs/engrams/*";

/// Ensure the engram refspecs are configured for a remote.
pub fn ensure_refspecs(repo: &Repository, remote_name: &str) -> Result<bool, ProtocolError> {
    let remote = repo
        .find_remote(remote_name)
        .map_err(|_| ProtocolError::RemoteNotFound(remote_name.into()))?;

    let mut needs_fetch = true;
    let mut needs_push = true;

    // Check existing fetch refspecs
    if let Ok(refspecs) = remote.fetch_refspecs() {
        for i in 0..refspecs.len() {
            if let Some(spec) = refspecs.get(i) {
                if spec == ENGRAM_FETCH_REFSPEC {
                    needs_fetch = false;
                }
            }
        }
    }

    // Check existing push refspecs
    if let Ok(refspecs) = remote.push_refspecs() {
        for i in 0..refspecs.len() {
            if let Some(spec) = refspecs.get(i) {
                if spec == ENGRAM_PUSH_REFSPEC {
                    needs_push = false;
                }
            }
        }
    }

    drop(remote);

    let mut changed = false;

    if needs_fetch {
        repo.remote_add_fetch(remote_name, ENGRAM_FETCH_REFSPEC)?;
        changed = true;
    }

    if needs_push {
        repo.remote_add_push(remote_name, ENGRAM_PUSH_REFSPEC)?;
        changed = true;
    }

    Ok(changed)
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
