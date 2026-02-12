use git2::{Oid, Repository};

use crate::error::CoreError;
use crate::model::EngramId;

/// The ref prefix for all engram refs.
pub const ENGRAM_REF_PREFIX: &str = "refs/engrams/";

/// Build the full ref name for an engram: refs/engrams/<ab>/<full-id>
pub fn engram_ref_name(id: &EngramId) -> String {
    format!("refs/engrams/{}/{}", id.fanout_prefix(), id.as_str())
}

/// Create or update the ref for an engram.
pub fn create_engram_ref(
    repo: &Repository,
    id: &EngramId,
    commit_oid: Oid,
) -> Result<(), CoreError> {
    let ref_name = engram_ref_name(id);
    repo.reference(&ref_name, commit_oid, true, "engram: create")?;
    Ok(())
}

/// Delete the ref for an engram.
pub fn delete_engram_ref(repo: &Repository, id: &EngramId) -> Result<(), CoreError> {
    let ref_name = engram_ref_name(id);
    let mut reference = repo.find_reference(&ref_name)?;
    reference.delete()?;
    Ok(())
}

/// List all engram ref names using glob. Returns (EngramId, commit Oid) pairs.
pub fn list_engram_refs(repo: &Repository) -> Result<Vec<(EngramId, Oid)>, CoreError> {
    let mut results = Vec::new();
    let pattern = format!("{ENGRAM_REF_PREFIX}*/*");
    let refs = repo.references_glob(&pattern)?;
    for reference in refs {
        let reference = reference?;
        if let (Some(name), Some(oid)) = (reference.name(), reference.target()) {
            // Extract the ID from refs/engrams/ab/full-id
            if let Some(id_part) = name.strip_prefix(ENGRAM_REF_PREFIX) {
                // id_part is "ab/full-id"
                if let Some((_prefix, full_id)) = id_part.split_once('/') {
                    results.push((EngramId(full_id.to_string()), oid));
                }
            }
        }
    }
    Ok(results)
}

/// Resolve an engram ID (or prefix) to its full ID and commit Oid.
pub fn resolve_engram_ref(
    repo: &Repository,
    id_or_prefix: &str,
) -> Result<(EngramId, Oid), CoreError> {
    // First try exact match
    let exact_id = EngramId(id_or_prefix.to_string());
    let ref_name = engram_ref_name(&exact_id);
    if let Ok(reference) = repo.find_reference(&ref_name) {
        if let Some(oid) = reference.target() {
            return Ok((exact_id, oid));
        }
    }

    // Try prefix match
    let all_refs = list_engram_refs(repo)?;
    let matches: Vec<_> = all_refs
        .iter()
        .filter(|(id, _)| id.as_str().starts_with(id_or_prefix))
        .collect();

    match matches.len() {
        0 => Err(CoreError::NotFound {
            id: id_or_prefix.to_string(),
        }),
        1 => Ok(matches[0].clone()),
        _ => Err(CoreError::Parse(format!(
            "Ambiguous engram ID prefix '{}': {} matches",
            id_or_prefix,
            matches.len()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_engram_ref_name() {
        let id = EngramId("abcdef1234567890abcdef1234567890".into());
        assert_eq!(
            engram_ref_name(&id),
            "refs/engrams/ab/abcdef1234567890abcdef1234567890"
        );
    }

    #[test]
    fn test_create_and_list_refs() {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();

        // Create a dummy blob + tree + commit to point refs at
        let blob_oid = repo.blob(b"test").unwrap();
        let mut tb = repo.treebuilder(None).unwrap();
        tb.insert("test", blob_oid, 0o100644).unwrap();
        let tree_oid = tb.write().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = git2::Signature::now("test", "test@test").unwrap();
        let commit_oid = repo.commit(None, &sig, &sig, "test", &tree, &[]).unwrap();

        // Create engram refs
        let id1 = EngramId("abcdef1234567890abcdef1234567890".into());
        let id2 = EngramId("123456abcdef7890123456abcdef7890".into());
        create_engram_ref(&repo, &id1, commit_oid).unwrap();
        create_engram_ref(&repo, &id2, commit_oid).unwrap();

        // List
        let refs = list_engram_refs(&repo).unwrap();
        assert_eq!(refs.len(), 2);

        // Delete
        delete_engram_ref(&repo, &id1).unwrap();
        let refs = list_engram_refs(&repo).unwrap();
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].0, id2);
    }

    #[test]
    fn test_resolve_prefix() {
        let tmp = TempDir::new().unwrap();
        let repo = Repository::init(tmp.path()).unwrap();

        let blob_oid = repo.blob(b"test").unwrap();
        let mut tb = repo.treebuilder(None).unwrap();
        tb.insert("test", blob_oid, 0o100644).unwrap();
        let tree_oid = tb.write().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        let sig = git2::Signature::now("test", "test@test").unwrap();
        let commit_oid = repo.commit(None, &sig, &sig, "test", &tree, &[]).unwrap();

        let id = EngramId("abcdef1234567890abcdef1234567890".into());
        create_engram_ref(&repo, &id, commit_oid).unwrap();

        // Full match
        let (resolved, _) = resolve_engram_ref(&repo, "abcdef1234567890abcdef1234567890").unwrap();
        assert_eq!(resolved, id);

        // Prefix match
        let (resolved, _) = resolve_engram_ref(&repo, "abcdef").unwrap();
        assert_eq!(resolved, id);

        // Not found
        assert!(resolve_engram_ref(&repo, "zzzzz").is_err());
    }
}
