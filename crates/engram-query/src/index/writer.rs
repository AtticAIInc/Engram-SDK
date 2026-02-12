use std::path::Path;

use tantivy::doc;
use tantivy::{Index, IndexWriter};

use engram_core::model::{EngramData, TranscriptContent};

use super::schema::EngramSchema;
use crate::error::QueryError;

/// Writes engrams to the Tantivy index.
pub struct EngramIndexWriter {
    schema: EngramSchema,
    index: Index,
    writer: IndexWriter,
}

impl EngramIndexWriter {
    /// Open or create an index at the given path.
    pub fn open(path: &Path) -> Result<Self, QueryError> {
        let schema = EngramSchema::new();
        let index = if path.exists() && path.join("meta.json").exists() {
            Index::open_in_dir(path)?
        } else {
            std::fs::create_dir_all(path).map_err(QueryError::Io)?;
            Index::create_in_dir(path, schema.schema.clone())?
        };

        // 50MB heap for indexing
        let writer = index.writer(50_000_000)?;

        Ok(Self {
            schema,
            index,
            writer,
        })
    }

    /// Index a single engram.
    pub fn index_engram(&mut self, data: &EngramData) -> Result<(), QueryError> {
        let s = &self.schema;

        // Concatenate transcript text entries
        let transcript_text: String = data
            .transcript
            .entries
            .iter()
            .filter_map(|e| match &e.content {
                TranscriptContent::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Concatenate file paths
        let file_paths: String = data
            .operations
            .file_changes
            .iter()
            .map(|fc| fc.path.clone())
            .collect::<Vec<_>>()
            .join("\n");

        // Concatenate dead ends
        let dead_ends: String = data
            .intent
            .dead_ends
            .iter()
            .map(|de| format!("{}: {}", de.approach, de.reason))
            .collect::<Vec<_>>()
            .join("\n");

        // Convert chrono to tantivy datetime
        let created_at =
            tantivy::DateTime::from_timestamp_secs(data.manifest.created_at.timestamp());

        let manifest_json = serde_json::to_string(&data.manifest)?;

        self.writer.add_document(doc!(
            s.id => data.manifest.id.as_str(),
            s.intent_request => data.intent.original_request.as_str(),
            s.intent_summary => data.intent.summary.as_deref().unwrap_or(""),
            s.transcript_text => transcript_text,
            s.agent_name => data.manifest.agent.name.as_str(),
            s.agent_model => data.manifest.agent.model.as_deref().unwrap_or(""),
            s.created_at => created_at,
            s.file_paths => file_paths,
            s.dead_ends => dead_ends,
            s.cost_usd => data.manifest.token_usage.cost_usd.unwrap_or(0.0),
            s.total_tokens => data.manifest.token_usage.total_tokens,
            s.manifest_json => manifest_json,
        ))?;

        Ok(())
    }

    /// Commit all pending changes.
    pub fn commit(&mut self) -> Result<(), QueryError> {
        self.writer.commit()?;
        Ok(())
    }

    /// Delete an engram from the index by its ID.
    pub fn delete_engram(&mut self, id: &str) -> Result<(), QueryError> {
        let term = tantivy::Term::from_field_text(self.schema.id, id);
        self.writer.delete_term(term);
        Ok(())
    }

    /// Get a reference to the underlying index.
    pub fn index(&self) -> &Index {
        &self.index
    }
}
