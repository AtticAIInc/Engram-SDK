use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::CoreError;

const CONFIG_DIR: &str = "engram";
const CONFIG_FILE: &str = "repos.toml";

/// Global engram configuration shared across repositories.
/// Stored at `~/.config/engram/repos.toml` (or `$XDG_CONFIG_HOME/engram/repos.toml`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub repos: Vec<PathBuf>,
}

impl GlobalConfig {
    /// Get the path to the global config file.
    pub fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join(CONFIG_DIR).join(CONFIG_FILE))
    }

    /// Load the global config. Returns default if file doesn't exist.
    pub fn load() -> Result<Self, CoreError> {
        let path = Self::config_path()
            .ok_or_else(|| CoreError::Config("Cannot determine config directory".to_string()))?;

        if !path.exists() {
            return Ok(Self::default());
        }

        let content = std::fs::read_to_string(&path)
            .map_err(|e| CoreError::Config(format!("Failed to read {}: {}", path.display(), e)))?;

        toml::from_str(&content)
            .map_err(|e| CoreError::Config(format!("Failed to parse {}: {}", path.display(), e)))
    }

    /// Save the global config to disk.
    pub fn save(&self) -> Result<(), CoreError> {
        let path = Self::config_path()
            .ok_or_else(|| CoreError::Config("Cannot determine config directory".to_string()))?;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                CoreError::Config(format!("Failed to create {}: {}", parent.display(), e))
            })?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| CoreError::Config(format!("Failed to serialize config: {}", e)))?;

        std::fs::write(&path, content)
            .map_err(|e| CoreError::Config(format!("Failed to write {}: {}", path.display(), e)))?;

        Ok(())
    }

    /// Add a repository path if not already present. Returns true if added.
    pub fn add_repo(&mut self, path: &Path) -> bool {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if self
            .repos
            .iter()
            .any(|p| p.canonicalize().unwrap_or_else(|_| p.clone()) == canonical)
        {
            return false;
        }
        self.repos.push(canonical);
        true
    }

    /// Remove a repository path. Returns true if removed.
    pub fn remove_repo(&mut self, path: &Path) -> bool {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let before = self.repos.len();
        self.repos
            .retain(|p| p.canonicalize().unwrap_or_else(|_| p.clone()) != canonical);
        self.repos.len() < before
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_add_repo_deduplicates() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();

        let mut config = GlobalConfig::default();
        assert!(config.add_repo(path));
        assert!(!config.add_repo(path)); // duplicate
        assert_eq!(config.repos.len(), 1);
    }

    #[test]
    fn test_remove_repo() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path();

        let mut config = GlobalConfig::default();
        config.add_repo(path);
        assert!(config.remove_repo(path));
        assert!(config.repos.is_empty());
        assert!(!config.remove_repo(path)); // already removed
    }

    #[test]
    fn test_serde_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let mut config = GlobalConfig::default();
        config.add_repo(tmp.path());

        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: GlobalConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(config.repos.len(), deserialized.repos.len());
    }
}
