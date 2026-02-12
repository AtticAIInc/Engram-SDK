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
    pub fn index_engram(&self, data: &EngramData) -> Result<(), QueryError> {
        if !self.index_path.exists() {
            return Ok(()); // Index doesn't exist yet, skip
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
