use std::path::Path;

use engram_core::storage::GitStorage;

use super::writer::EngramIndexWriter;
use crate::error::QueryError;

/// Rebuild the index from scratch by reading all engrams from Git.
pub fn rebuild_index(storage: &GitStorage, index_path: &Path) -> Result<usize, QueryError> {
    // Remove existing index
    if index_path.exists() {
        std::fs::remove_dir_all(index_path).map_err(QueryError::Io)?;
    }

    let mut writer = EngramIndexWriter::open(index_path)?;
    let manifests = storage.list(&Default::default())?;

    let mut count = 0;
    for manifest in &manifests {
        match storage.read(manifest.id.as_str()) {
            Ok(data) => {
                writer.index_engram(&data)?;
                count += 1;
            }
            Err(e) => {
                tracing::warn!("Failed to read engram {}: {e}", manifest.id);
            }
        }
    }

    writer.commit()?;

    tracing::info!("Indexed {count} engrams");
    Ok(count)
}
