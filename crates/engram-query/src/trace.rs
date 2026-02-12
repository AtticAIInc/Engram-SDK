use engram_core::model::Manifest;
use engram_core::storage::GitStorage;

use crate::error::QueryError;
use crate::search::SearchEngine;

/// An entry in a file's reasoning trace.
#[derive(Debug, Clone)]
pub struct TraceEntry {
    pub manifest: Manifest,
    pub change_type: String,
}

/// Trace all engrams that touched a file, ordered by time.
pub fn trace_file(
    storage: &GitStorage,
    search: &SearchEngine,
    file_path: &str,
) -> Result<Vec<TraceEntry>, QueryError> {
    let results = search.search_by_file(storage, file_path, 100)?;

    let mut entries: Vec<TraceEntry> = results
        .into_iter()
        .map(|r| {
            // Fetch the full engram to get actual change type for this file
            let change_type = storage
                .read(r.manifest.id.as_str())
                .ok()
                .and_then(|data| {
                    data.operations
                        .file_changes
                        .iter()
                        .find(|fc| fc.path == file_path)
                        .map(|fc| match &fc.change_type {
                            engram_core::model::FileChangeType::Created => "created".to_string(),
                            engram_core::model::FileChangeType::Modified => "modified".to_string(),
                            engram_core::model::FileChangeType::Deleted => "deleted".to_string(),
                            engram_core::model::FileChangeType::Renamed { from } => {
                                format!("renamed from {from}")
                            }
                        })
                })
                .unwrap_or_else(|| "modified".to_string());
            TraceEntry {
                manifest: r.manifest,
                change_type,
            }
        })
        .collect();

    // Sort by creation time (oldest first for a trace)
    entries.sort_by_key(|e| e.manifest.created_at);

    Ok(entries)
}
