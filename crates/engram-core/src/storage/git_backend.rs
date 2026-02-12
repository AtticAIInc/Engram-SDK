use std::path::Path;

use git2::Repository;

use crate::config::EngramConfig;
use crate::error::CoreError;
use crate::model::{EngramData, EngramId, Manifest};

use super::objects::create_engram_objects;
use super::read;
use super::refs;

const ENGRAM_HEAD_FILE: &str = "engram-head";

/// Options for listing engrams.
#[derive(Debug, Clone, Default)]
pub struct ListOptions {
    pub limit: Option<usize>,
    pub agent_filter: Option<String>,
}

/// The main storage interface for engram operations.
pub struct GitStorage {
    repo: Repository,
}

impl GitStorage {
    /// Open the Git repository at the given path.
    pub fn open(path: &Path) -> Result<Self, CoreError> {
        let repo = Repository::open(path)?;
        Ok(Self { repo })
    }

    /// Discover the Git repository from the current directory.
    pub fn discover() -> Result<Self, CoreError> {
        let repo = Repository::discover(".")?;
        Ok(Self { repo })
    }

    /// Check if engram has been initialized in this repo.
    pub fn is_initialized(&self) -> bool {
        self.repo
            .config()
            .ok()
            .and_then(|c| c.get_bool("engram.enabled").ok())
            .unwrap_or(false)
    }

    /// Initialize engram in this repo: set config, configure refspecs.
    /// If `remote` is Some, only configure that specific remote; otherwise configure all.
    pub fn init_with_remote(&self, remote: Option<&str>) -> Result<(), CoreError> {
        let mut config = self.repo.config().map_err(CoreError::Git)?;
        let engram_config = EngramConfig::default_init();
        engram_config.save(&mut config)?;

        // Set schema version
        config
            .set_i32("engram.version", 1)
            .map_err(CoreError::Git)?;

        // Add engram fetch/push refspecs to remotes
        self.configure_remotes_filtered(remote)?;

        Ok(())
    }

    /// Initialize engram in this repo: set config, configure refspecs on all remotes.
    pub fn init(&self) -> Result<(), CoreError> {
        self.init_with_remote(None)
    }

    /// Create a new engram and store it as Git objects.
    pub fn create(&self, data: &EngramData) -> Result<EngramId, CoreError> {
        let commit_oid = create_engram_objects(&self.repo, data)?;
        let id = data.manifest.id.clone();
        refs::create_engram_ref(&self.repo, &id, commit_oid)?;
        // Update engram-head pointer for O(1) HEAD resolution
        self.update_head_pointer(&id, &data.manifest.created_at);
        Ok(id)
    }

    /// Resolve "HEAD" to the most recent engram ID, or pass through to prefix resolution.
    pub fn resolve(&self, id_or_alias: &str) -> Result<String, CoreError> {
        if id_or_alias.eq_ignore_ascii_case("HEAD") {
            // Fast path: try engram-head pointer file
            if let Some(head_id) = self.read_head_pointer() {
                // Validate the ref still exists
                if refs::resolve_engram_ref(&self.repo, &head_id).is_ok() {
                    return Ok(head_id);
                }
            }
            // Fallback: O(n) scan
            let manifests = self.list(&ListOptions::default())?;
            if let Some(m) = manifests.first() {
                // Repair the head pointer
                self.update_head_pointer(&m.id, &m.created_at);
                Ok(m.id.as_str().to_string())
            } else {
                Err(CoreError::NotFound {
                    id: "HEAD (no engrams exist)".to_string(),
                })
            }
        } else {
            let (id, _oid) = refs::resolve_engram_ref(&self.repo, id_or_alias)?;
            Ok(id.as_str().to_string())
        }
    }

    /// Read an engram by its ID (or prefix).
    pub fn read(&self, id_or_prefix: &str) -> Result<EngramData, CoreError> {
        let (_id, oid) = refs::resolve_engram_ref(&self.repo, id_or_prefix)?;
        read::read_engram(&self.repo, oid)
    }

    /// Read only the manifest (fast path for listing).
    pub fn read_manifest(&self, id_or_prefix: &str) -> Result<Manifest, CoreError> {
        let (_id, oid) = refs::resolve_engram_ref(&self.repo, id_or_prefix)?;
        read::read_manifest(&self.repo, oid)
    }

    /// List all engrams, optionally filtered.
    pub fn list(&self, opts: &ListOptions) -> Result<Vec<Manifest>, CoreError> {
        let all_refs = refs::list_engram_refs(&self.repo)?;
        let mut manifests = Vec::with_capacity(all_refs.len());

        for (_id, oid) in &all_refs {
            match read::read_manifest(&self.repo, *oid) {
                Ok(manifest) => {
                    // Apply agent filter
                    if let Some(agent) = &opts.agent_filter {
                        if !manifest.agent.name.contains(agent.as_str()) {
                            continue;
                        }
                    }
                    manifests.push(manifest);
                }
                Err(e) => {
                    tracing::warn!("Skipping unreadable engram: {e}");
                }
            }
        }

        // Sort by created_at descending (most recent first)
        manifests.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Apply limit
        if let Some(limit) = opts.limit {
            manifests.truncate(limit);
        }

        Ok(manifests)
    }

    /// Check if an engram with the given source hash already exists.
    /// Used for import deduplication.
    pub fn find_by_source_hash(&self, hash: &str) -> Option<EngramId> {
        let all_refs = refs::list_engram_refs(&self.repo).ok()?;
        for (id, oid) in &all_refs {
            if let Ok(manifest) = read::read_manifest(&self.repo, *oid) {
                if manifest.source_hash.as_deref() == Some(hash) {
                    return Some(id.clone());
                }
            }
        }
        None
    }

    /// Delete an engram by removing its ref.
    pub fn delete(&self, id_or_prefix: &str) -> Result<(), CoreError> {
        let (id, _oid) = refs::resolve_engram_ref(&self.repo, id_or_prefix)?;
        refs::delete_engram_ref(&self.repo, &id)
    }

    /// Get the underlying git2::Repository reference.
    pub fn repo(&self) -> &Repository {
        &self.repo
    }

    /// Get the repo working directory path.
    pub fn workdir(&self) -> Option<&Path> {
        self.repo.workdir()
    }

    /// Update the engram-head pointer file. Only updates if this engram is newer.
    /// Best-effort — failures are silently ignored.
    fn update_head_pointer(&self, id: &EngramId, created_at: &chrono::DateTime<chrono::Utc>) {
        // repo.path() returns the .git dir (or the repo dir for bare repos)
        let head_path = self.repo.path().join(ENGRAM_HEAD_FILE);

        // Read existing pointer to check timestamp
        if let Ok(existing) = std::fs::read_to_string(&head_path) {
            // Format: "<id> <rfc3339-timestamp>"
            if let Some(ts_str) = existing.split_whitespace().nth(1) {
                if let Ok(existing_ts) = ts_str.parse::<chrono::DateTime<chrono::Utc>>() {
                    if existing_ts >= *created_at {
                        return; // Existing head is newer or same
                    }
                }
            }
        }

        let content = format!("{} {}", id.as_str(), created_at.to_rfc3339());
        let _ = std::fs::write(&head_path, content);
    }

    /// Read the engram-head pointer file. Returns the ID if valid.
    fn read_head_pointer(&self) -> Option<String> {
        let head_path = self.repo.path().join(ENGRAM_HEAD_FILE);
        let content = std::fs::read_to_string(&head_path).ok()?;
        content.split_whitespace().next().map(String::from)
    }

    /// Configure fetch/push refspecs for engram refs on remotes.
    /// If `filter` is Some, only configure that specific remote.
    fn configure_remotes_filtered(&self, filter: Option<&str>) -> Result<(), CoreError> {
        let remotes = self.repo.remotes().map_err(CoreError::Git)?;
        let mut config = self.repo.config().map_err(CoreError::Git)?;

        for remote_name in remotes.iter().flatten() {
            if let Some(target) = filter {
                if remote_name != target {
                    continue;
                }
            }
            let fetch_key = format!("remote.{remote_name}.fetch");
            let push_key = format!("remote.{remote_name}.push");
            let fetch_refspec = "+refs/engrams/*:refs/engrams/*";
            let push_refspec = "refs/engrams/*:refs/engrams/*";

            // Check if already configured by iterating existing values
            let fetch_exists = config
                .entries(Some(&fetch_key))
                .ok()
                .map(|mut entries| {
                    let mut found = false;
                    while let Some(Ok(entry)) = entries.next() {
                        if entry.value() == Some(fetch_refspec) {
                            found = true;
                            break;
                        }
                    }
                    found
                })
                .unwrap_or(false);

            if !fetch_exists {
                config
                    .set_multivar(&fetch_key, "^$", fetch_refspec)
                    .or_else(|_| {
                        // If set_multivar fails (no existing entry), try adding
                        self.repo
                            .remote_add_fetch(remote_name, fetch_refspec)
                            .map(|_| ())
                    })
                    .map_err(CoreError::Git)?;
            }

            let push_exists = config
                .entries(Some(&push_key))
                .ok()
                .map(|mut entries| {
                    let mut found = false;
                    while let Some(Ok(entry)) = entries.next() {
                        if entry.value() == Some(push_refspec) {
                            found = true;
                            break;
                        }
                    }
                    found
                })
                .unwrap_or(false);

            if !push_exists {
                config
                    .set_multivar(&push_key, "^$", push_refspec)
                    .or_else(|_| {
                        self.repo
                            .remote_add_push(remote_name, push_refspec)
                            .map(|_| ())
                    })
                    .map_err(CoreError::Git)?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;
    use chrono::Utc;
    use tempfile::TempDir;

    fn make_test_data() -> EngramData {
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
                summary: None,
                dead_ends: vec![],
                decisions: vec![],
            },
            transcript: Transcript::default(),
            operations: Operations::default(),
            lineage: Lineage::default(),
        }
    }

    #[test]
    fn test_full_lifecycle() {
        let tmp = TempDir::new().unwrap();
        Repository::init(tmp.path()).unwrap();

        let storage = GitStorage::open(tmp.path()).unwrap();

        // Not initialized yet
        assert!(!storage.is_initialized());

        // Init
        storage.init().unwrap();
        assert!(storage.is_initialized());

        // Empty list
        let manifests = storage.list(&ListOptions::default()).unwrap();
        assert!(manifests.is_empty());

        // Create
        let data = make_test_data();
        let id = data.manifest.id.clone();
        let created_id = storage.create(&data).unwrap();
        assert_eq!(created_id, id);

        // List
        let manifests = storage.list(&ListOptions::default()).unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].id, id);

        // Read
        let loaded = storage.read(id.as_str()).unwrap();
        assert_eq!(loaded.manifest.id, id);
        assert_eq!(loaded.intent.original_request, "Test request");

        // Read manifest only
        let manifest = storage.read_manifest(id.as_str()).unwrap();
        assert_eq!(manifest.summary, Some("Test engram".into()));

        // Delete
        storage.delete(id.as_str()).unwrap();
        let manifests = storage.list(&ListOptions::default()).unwrap();
        assert!(manifests.is_empty());
    }

    #[test]
    fn test_list_with_filter() {
        let tmp = TempDir::new().unwrap();
        Repository::init(tmp.path()).unwrap();
        let storage = GitStorage::open(tmp.path()).unwrap();
        storage.init().unwrap();

        // Create two engrams with different agents
        let mut data1 = make_test_data();
        data1.manifest.agent.name = "claude-code".into();
        storage.create(&data1).unwrap();

        let mut data2 = make_test_data();
        data2.manifest.agent.name = "aider".into();
        storage.create(&data2).unwrap();

        // Filter by agent
        let opts = ListOptions {
            agent_filter: Some("claude".into()),
            ..Default::default()
        };
        let manifests = storage.list(&opts).unwrap();
        assert_eq!(manifests.len(), 1);
        assert_eq!(manifests[0].agent.name, "claude-code");

        // No filter
        let all = storage.list(&ListOptions::default()).unwrap();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_list_with_limit() {
        let tmp = TempDir::new().unwrap();
        Repository::init(tmp.path()).unwrap();
        let storage = GitStorage::open(tmp.path()).unwrap();
        storage.init().unwrap();

        for _ in 0..5 {
            storage.create(&make_test_data()).unwrap();
        }

        let opts = ListOptions {
            limit: Some(3),
            ..Default::default()
        };
        let manifests = storage.list(&opts).unwrap();
        assert_eq!(manifests.len(), 3);
    }
}
