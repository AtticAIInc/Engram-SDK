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

    if opts.dry_run {
        // Count refs that would be pushed
        let refs = engram_core::storage::refs::list_engram_refs(repo)?;
        return Ok(PushResult {
            remote: remote_name.into(),
            refs_pushed: refs.len(),
        });
    }

    let mut remote = repo
        .find_remote(remote_name)
        .map_err(|_| ProtocolError::RemoteNotFound(remote_name.into()))?;

    let refspec_strs: Vec<&str> = refspecs.iter().map(|s| s.as_str()).collect();

    remote
        .push(&refspec_strs, None)
        .map_err(|e| ProtocolError::Sync(format!("Push failed: {e}")))?;

    // Count refs (approximate)
    let refs = engram_core::storage::refs::list_engram_refs(repo)?;

    Ok(PushResult {
        remote: remote_name.into(),
        refs_pushed: refs.len(),
    })
}

/// Fetch engram refs from a remote.
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
