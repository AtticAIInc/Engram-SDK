use std::path::PathBuf;

use engram_core::model::EngramData;
use engram_core::storage::GitStorage;

use crate::error::QueryError;
use crate::index::{rebuild_index, EngramIndexWriter, EngramSearcher, SearchResult};

/// High-level search engine that manages index lifecycle.
pub struct SearchEngine {
    index_path: PathBuf,
}

impl SearchEngine {
    /// Open a search engine for a repository. Index is stored at `.git/engram-index/`.
    pub fn open(storage: &GitStorage) -> Result<Self, QueryError> {
        let git_dir = storage.repo().path();
        let index_path = git_dir.join("engram-index");
        Ok(Self { index_path })
    }

    /// Ensure the index exists, creating it if needed.
    pub fn ensure_index(&self, storage: &GitStorage) -> Result<(), QueryError> {
        if !self.index_path.exists() || !self.index_path.join("meta.json").exists() {
            rebuild_index(storage, &self.index_path)?;
        }
        Ok(())
    }

    /// Search engrams by free-text query.
    pub fn search(
        &self,
        storage: &GitStorage,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, QueryError> {
        self.ensure_index(storage)?;
        let searcher = EngramSearcher::open(&self.index_path)?;
        searcher.search(query, limit)
    }

    /// Search for engrams that touched a file.
    pub fn search_by_file(
        &self,
        storage: &GitStorage,
        file_path: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, QueryError> {
        self.ensure_index(storage)?;
        let searcher = EngramSearcher::open(&self.index_path)?;
        searcher.search_by_file(file_path, limit)
    }

    /// Index a single new engram (incremental update).
    ///
    /// If the index directory doesn't exist yet, this is a no-op (the index
    /// will be created on the first search via `ensure_index`).
    pub fn index_engram(&self, data: &EngramData) -> Result<(), QueryError> {
        if !self.index_path.exists() {
            tracing::debug!(
                "Search index not yet created at {}, skipping incremental update",
                self.index_path.display()
            );
            return Ok(());
        }
        let mut writer = EngramIndexWriter::open(&self.index_path)?;
        writer.index_engram(data)?;
        writer.commit()?;
        Ok(())
    }

    /// Rebuild the index from scratch.
    pub fn rebuild(&self, storage: &GitStorage) -> Result<usize, QueryError> {
        rebuild_index(storage, &self.index_path)
    }

    /// Return the index path.
    pub fn index_path(&self) -> &PathBuf {
        &self.index_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use engram_core::model::*;
    use tempfile::TempDir;

    fn make_test_data(request: &str, files: &[&str]) -> EngramData {
        EngramData {
            manifest: Manifest {
                id: EngramId::new(),
                version: 1,
                created_at: chrono::Utc::now(),
                finished_at: None,
                agent: AgentInfo {
                    name: "test-agent".into(),
                    model: Some("test-model".into()),
                    version: None,
                },
                git_commits: vec![],
                token_usage: TokenUsage::default(),
                summary: Some(request.into()),
                tags: vec![],
                capture_mode: CaptureMode::Sdk,
                source_hash: None,
            },
            intent: Intent {
                original_request: request.into(),
                interpreted_goal: None,
                summary: Some(request.into()),
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

    #[test]
    fn test_search_engine_open() {
        let tmp = TempDir::new().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        let storage = GitStorage::open(tmp.path()).unwrap();
        storage.init().unwrap();

        let engine = SearchEngine::open(&storage).unwrap();
        assert!(engine.index_path().ends_with("engram-index"));
    }

    #[test]
    fn test_search_empty_index() {
        let tmp = TempDir::new().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        let storage = GitStorage::open(tmp.path()).unwrap();
        storage.init().unwrap();

        let engine = SearchEngine::open(&storage).unwrap();
        let results = engine.search(&storage, "anything", 10).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_finds_engram() {
        let tmp = TempDir::new().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        let storage = GitStorage::open(tmp.path()).unwrap();
        storage.init().unwrap();

        let data = make_test_data("Add OAuth2 authentication", &["src/auth.rs"]);
        storage.create(&data).unwrap();

        let engine = SearchEngine::open(&storage).unwrap();
        engine.rebuild(&storage).unwrap();

        let results = engine.search(&storage, "OAuth2", 10).unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].manifest.id, data.manifest.id);
    }

    #[test]
    fn test_search_by_file() {
        let tmp = TempDir::new().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        let storage = GitStorage::open(tmp.path()).unwrap();
        storage.init().unwrap();

        let data = make_test_data("Add auth", &["src/auth.rs", "src/main.rs"]);
        storage.create(&data).unwrap();

        let engine = SearchEngine::open(&storage).unwrap();
        engine.rebuild(&storage).unwrap();

        let results = engine.search_by_file(&storage, "auth", 10).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_index_engram_incremental() {
        let tmp = TempDir::new().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        let storage = GitStorage::open(tmp.path()).unwrap();
        storage.init().unwrap();

        let engine = SearchEngine::open(&storage).unwrap();
        engine.rebuild(&storage).unwrap(); // Create empty index

        let data = make_test_data("Fix database connection pooling", &["src/db.rs"]);
        storage.create(&data).unwrap();
        engine.index_engram(&data).unwrap();

        let results = engine.search(&storage, "database", 10).unwrap();
        assert!(!results.is_empty());
    }

    #[test]
    fn test_rebuild_twice_swaps_index_in_place() {
        let tmp = TempDir::new().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        let storage = GitStorage::open(tmp.path()).unwrap();
        storage.init().unwrap();

        let data1 = make_test_data("Add OAuth2 authentication", &["src/auth.rs"]);
        storage.create(&data1).unwrap();

        let engine = SearchEngine::open(&storage).unwrap();
        assert_eq!(engine.rebuild(&storage).unwrap(), 1);
        assert!(!engine.search(&storage, "OAuth2", 10).unwrap().is_empty());

        // A second engram plus a rebuild must replace the existing on-disk index
        // (exercises the "move live aside, rename temp into place" swap path).
        let data2 = make_test_data("Fix rate limiting bug", &["src/rate.rs"]);
        storage.create(&data2).unwrap();
        assert_eq!(engine.rebuild(&storage).unwrap(), 2);

        // Both old and new content are searchable after the swap.
        assert!(!engine.search(&storage, "OAuth2", 10).unwrap().is_empty());
        assert!(!engine.search(&storage, "limiting", 10).unwrap().is_empty());

        // No scratch directories are left behind next to the live index.
        let parent = engine.index_path().parent().unwrap();
        for entry in std::fs::read_dir(parent).unwrap().flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            assert!(
                !name.contains("rebuild") && !name.contains(".old"),
                "leftover scratch dir: {name}"
            );
        }
    }

    #[test]
    fn test_rebuild_recovers_from_stale_scratch_dir() {
        let tmp = TempDir::new().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        let storage = GitStorage::open(tmp.path()).unwrap();
        storage.init().unwrap();
        storage
            .create(&make_test_data("Add auth", &["src/auth.rs"]))
            .unwrap();

        let engine = SearchEngine::open(&storage).unwrap();

        // Simulate a crashed prior rebuild by leaving a stale scratch dir with
        // the exact name this process would use.
        let pid = std::process::id();
        let parent = engine.index_path().parent().unwrap();
        let stale = parent.join(format!(".engram-index.rebuild.{pid}"));
        std::fs::create_dir_all(stale.join("garbage")).unwrap();

        // Rebuild must clean it up and still succeed.
        assert_eq!(engine.rebuild(&storage).unwrap(), 1);
        assert!(!stale.exists());
        assert!(!engine.search(&storage, "auth", 10).unwrap().is_empty());
    }

    #[test]
    fn test_rebuild_reindexes_all() {
        let tmp = TempDir::new().unwrap();
        git2::Repository::init(tmp.path()).unwrap();
        let storage = GitStorage::open(tmp.path()).unwrap();
        storage.init().unwrap();

        let data1 = make_test_data("Add user authentication", &["src/auth.rs"]);
        let data2 = make_test_data("Fix rate limiting bug", &["src/rate_limiter.rs"]);
        storage.create(&data1).unwrap();
        storage.create(&data2).unwrap();

        let engine = SearchEngine::open(&storage).unwrap();
        let count = engine.rebuild(&storage).unwrap();
        assert_eq!(count, 2);

        let results = engine.search(&storage, "authentication", 10).unwrap();
        assert!(!results.is_empty());
    }
}
