use std::path::{Path, PathBuf};

use engram_core::storage::GitStorage;

use super::writer::EngramIndexWriter;
use crate::error::QueryError;

/// Rebuild the index from scratch by reading all engrams from Git.
///
/// The new index is built in a sibling temporary directory and only swapped
/// into place once it is fully committed. This avoids the race where a
/// concurrent search opens `index_path` after it has been deleted but before
/// the rebuild finishes (the old delete-then-build approach left the index
/// directory missing for the entire duration of the rebuild). The live index
/// is replaced with two `rename` calls, so the window in which it is absent is
/// microseconds rather than the full rebuild time.
pub fn rebuild_index(storage: &GitStorage, index_path: &Path) -> Result<usize, QueryError> {
    let (tmp_path, old_path) = scratch_paths(index_path);

    // Clean up any leftovers from a previously-crashed rebuild.
    if tmp_path.exists() {
        std::fs::remove_dir_all(&tmp_path).map_err(QueryError::Io)?;
    }
    if old_path.exists() {
        let _ = std::fs::remove_dir_all(&old_path);
    }

    // Build the full index into the temp directory.
    let mut writer = EngramIndexWriter::open(&tmp_path)?;
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
    // Drop the writer (and its lock on the temp dir) before swapping.
    drop(writer);

    swap_into_place(&tmp_path, index_path, &old_path)?;

    tracing::info!("Indexed {count} engrams");
    Ok(count)
}

/// Compute sibling scratch paths next to `index_path`, made unique per-process
/// so two concurrent rebuilds don't trample each other's temp directories.
fn scratch_paths(index_path: &Path) -> (PathBuf, PathBuf) {
    let pid = std::process::id();
    let name = index_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("engram-index");
    let parent = index_path.parent().unwrap_or_else(|| Path::new("."));
    (
        parent.join(format!(".{name}.rebuild.{pid}")),
        parent.join(format!(".{name}.old.{pid}")),
    )
}

/// Atomically replace `index_path` with the freshly-built `tmp_path`.
fn swap_into_place(tmp_path: &Path, index_path: &Path, old_path: &Path) -> Result<(), QueryError> {
    // Move the live index aside first: `rename` cannot replace a non-empty
    // directory, so the destination must not exist when we move the new one in.
    if index_path.exists() {
        std::fs::rename(index_path, old_path).map_err(QueryError::Io)?;
    }

    if let Err(e) = std::fs::rename(tmp_path, index_path) {
        // Swap failed — restore the previous index so we don't end up with no
        // index at all, then surface the error.
        if old_path.exists() {
            let _ = std::fs::rename(old_path, index_path);
        }
        return Err(QueryError::Io(e));
    }

    // The new index is live; discard the old one (best-effort).
    if old_path.exists() {
        let _ = std::fs::remove_dir_all(old_path);
    }
    Ok(())
}
