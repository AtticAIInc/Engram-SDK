use std::path::Path;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::Value;
use tantivy::{Index, ReloadPolicy};

use engram_core::model::Manifest;

use super::schema::EngramSchema;
use crate::error::QueryError;

/// Result of a search query.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub manifest: Manifest,
    pub score: f32,
    pub snippet: Option<String>,
}

/// Searches the engram index.
pub struct EngramSearcher {
    schema: EngramSchema,
    index: Index,
}

impl EngramSearcher {
    /// Open an existing index for reading.
    pub fn open(path: &Path) -> Result<Self, QueryError> {
        let schema = EngramSchema::new();
        let index = Index::open_in_dir(path)?;
        Ok(Self { schema, index })
    }

    /// Search engrams with a free-text query.
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<SearchResult>, QueryError> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        let searcher = reader.searcher();

        let query_parser = QueryParser::for_index(
            &self.index,
            vec![
                self.schema.intent_request,
                self.schema.intent_summary,
                self.schema.transcript_text,
                self.schema.dead_ends,
                self.schema.file_paths,
            ],
        );

        let query = query_parser
            .parse_query(query_str)
            .map_err(|e| QueryError::Search(e.to_string()))?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::with_capacity(top_docs.len());
        for (score, doc_address) in top_docs {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;

            // Extract manifest JSON
            let manifest_json = doc
                .get_first(self.schema.manifest_json)
                .and_then(|v| v.as_str())
                .unwrap_or("{}");

            let manifest: Manifest = serde_json::from_str(manifest_json)?;

            // Extract snippet from intent summary
            let snippet = doc
                .get_first(self.schema.intent_summary)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            results.push(SearchResult {
                manifest,
                score,
                snippet,
            });
        }

        Ok(results)
    }

    /// Search for engrams that modified a specific file path.
    pub fn search_by_file(
        &self,
        file_path: &str,
        limit: usize,
    ) -> Result<Vec<SearchResult>, QueryError> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;
        let searcher = reader.searcher();

        let query_parser = QueryParser::for_index(&self.index, vec![self.schema.file_paths]);

        let query = query_parser
            .parse_query(file_path)
            .map_err(|e| QueryError::Search(e.to_string()))?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let mut results = Vec::with_capacity(top_docs.len());
        for (score, doc_address) in top_docs {
            let doc: tantivy::TantivyDocument = searcher.doc(doc_address)?;
            let manifest_json = doc
                .get_first(self.schema.manifest_json)
                .and_then(|v| v.as_str())
                .unwrap_or("{}");
            let manifest: Manifest = serde_json::from_str(manifest_json)?;

            let snippet = doc
                .get_first(self.schema.intent_summary)
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());

            results.push(SearchResult {
                manifest,
                score,
                snippet,
            });
        }

        Ok(results)
    }
}
