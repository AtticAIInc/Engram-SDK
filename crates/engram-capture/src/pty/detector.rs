use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use sha2::{Digest, Sha256};

use engram_core::model::{FileChange, FileChangeType};

/// Snapshot the working tree: map of relative path -> SHA256 hash.
/// Respects .gitignore, .git/info/exclude, and global gitignore.
pub fn snapshot_working_tree(
    repo_root: &Path,
) -> Result<HashMap<PathBuf, Vec<u8>>, std::io::Error> {
    let mut snapshot = HashMap::new();

    let walker = WalkBuilder::new(repo_root)
        .hidden(false) // include dotfiles (.gitignore, .eslintrc, etc.)
        .git_ignore(true) // respect .gitignore
        .git_global(true) // respect global gitignore
        .git_exclude(true) // respect .git/info/exclude
        .filter_entry(|e| {
            // Skip .git directory (not filtered by gitignore since it's special)
            e.file_name().to_str() != Some(".git")
        })
        .build();

    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::debug!("Skipping walk error: {e}");
                continue;
            }
        };
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        let rel_path = entry
            .path()
            .strip_prefix(repo_root)
            .unwrap_or(entry.path())
            .to_path_buf();

        match std::fs::read(entry.path()) {
            Ok(contents) => {
                let hash = Sha256::digest(&contents).to_vec();
                snapshot.insert(rel_path, hash);
            }
            Err(e) => {
                tracing::debug!("Skipping unreadable file {:?}: {}", entry.path(), e);
            }
        }
    }

    Ok(snapshot)
}

/// Compare before/after snapshots to detect file changes.
pub fn detect_changes(
    before: &HashMap<PathBuf, Vec<u8>>,
    after: &HashMap<PathBuf, Vec<u8>>,
) -> Vec<FileChange> {
    let mut changes = Vec::new();

    // Check for created and modified files
    for (path, after_hash) in after {
        match before.get(path) {
            None => {
                changes.push(FileChange {
                    path: path.to_string_lossy().to_string(),
                    change_type: FileChangeType::Created,
                    lines_added: None,
                    lines_removed: None,
                });
            }
            Some(before_hash) if before_hash != after_hash => {
                changes.push(FileChange {
                    path: path.to_string_lossy().to_string(),
                    change_type: FileChangeType::Modified,
                    lines_added: None,
                    lines_removed: None,
                });
            }
            _ => {} // Unchanged
        }
    }

    // Check for deleted files
    for path in before.keys() {
        if !after.contains_key(path) {
            changes.push(FileChange {
                path: path.to_string_lossy().to_string(),
                change_type: FileChangeType::Deleted,
                lines_added: None,
                lines_removed: None,
            });
        }
    }

    // Sort for deterministic output
    changes.sort_by(|a, b| a.path.cmp(&b.path));
    changes
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_snapshot_and_detect_changes() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Create initial files
        std::fs::write(root.join("existing.txt"), "hello").unwrap();
        std::fs::write(root.join("to_delete.txt"), "goodbye").unwrap();
        std::fs::write(root.join("unchanged.txt"), "same").unwrap();

        let before = snapshot_working_tree(root).unwrap();
        assert_eq!(before.len(), 3);

        // Make changes
        std::fs::write(root.join("existing.txt"), "modified").unwrap();
        std::fs::remove_file(root.join("to_delete.txt")).unwrap();
        std::fs::write(root.join("new_file.txt"), "new").unwrap();

        let after = snapshot_working_tree(root).unwrap();
        let changes = detect_changes(&before, &after);

        assert_eq!(changes.len(), 3);

        let created: Vec<_> = changes
            .iter()
            .filter(|c| matches!(c.change_type, FileChangeType::Created))
            .collect();
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].path, "new_file.txt");

        let modified: Vec<_> = changes
            .iter()
            .filter(|c| matches!(c.change_type, FileChangeType::Modified))
            .collect();
        assert_eq!(modified.len(), 1);
        assert_eq!(modified[0].path, "existing.txt");

        let deleted: Vec<_> = changes
            .iter()
            .filter(|c| matches!(c.change_type, FileChangeType::Deleted))
            .collect();
        assert_eq!(deleted.len(), 1);
        assert_eq!(deleted[0].path, "to_delete.txt");
    }

    #[test]
    fn test_ignores_git_dir() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        std::fs::create_dir_all(root.join(".git/objects")).unwrap();
        std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/main").unwrap();
        std::fs::write(root.join("real_file.txt"), "content").unwrap();

        let snapshot = snapshot_working_tree(root).unwrap();
        assert_eq!(snapshot.len(), 1);
        assert!(snapshot.contains_key(Path::new("real_file.txt")));
    }

    #[test]
    fn test_respects_gitignore() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        // Initialize a git repo so .gitignore is respected
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join(".gitignore"), "*.log\nbuild/\n").unwrap();
        std::fs::write(root.join("source.rs"), "fn main() {}").unwrap();
        std::fs::write(root.join("debug.log"), "log data").unwrap();
        std::fs::create_dir_all(root.join("build")).unwrap();
        std::fs::write(root.join("build/output.bin"), "binary").unwrap();

        let snapshot = snapshot_working_tree(root).unwrap();
        // Only .gitignore and source.rs should be included (debug.log and build/ are ignored)
        assert!(snapshot.contains_key(Path::new("source.rs")));
        assert!(snapshot.contains_key(Path::new(".gitignore")));
        assert!(!snapshot.contains_key(Path::new("debug.log")));
        assert!(!snapshot.contains_key(Path::new("build/output.bin")));
    }
}
