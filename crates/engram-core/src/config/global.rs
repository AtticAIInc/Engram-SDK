use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::CoreError;

const CONFIG_DIR: &str = "engram";
const CONFIG_FILE: &str = "repos.toml";

/// Global settings for engram (API keys, model overrides).
/// Stored in the `[settings]` section of the global config file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Settings {
    /// Anthropic API key for LLM-powered summarization.
    /// Env var `ANTHROPIC_API_KEY` takes precedence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anthropic_api_key: Option<String>,

    /// Model to use for summarization (default: claude-haiku-4-5-20251001).
    /// Env var `ENGRAM_SUMMARIZE_MODEL` takes precedence.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summarize_model: Option<String>,
}

/// Global engram configuration shared across repositories.
/// Stored at `~/.config/engram/repos.toml` (or `$XDG_CONFIG_HOME/engram/repos.toml`).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalConfig {
    #[serde(default)]
    pub repos: Vec<PathBuf>,

    #[serde(default)]
    pub settings: Settings,
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

        std::fs::write(&path, &content)
            .map_err(|e| CoreError::Config(format!("Failed to write {}: {}", path.display(), e)))?;

        // Restrict permissions since the file may contain API keys
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = std::fs::Permissions::from_mode(0o600);
            let _ = std::fs::set_permissions(&path, perms);
        }

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

    /// Get the Anthropic API key: env var takes precedence, then config file.
    pub fn anthropic_api_key(&self) -> Option<String> {
        match std::env::var("ANTHROPIC_API_KEY") {
            Ok(k) if !k.is_empty() => Some(k),
            _ => self.settings.anthropic_api_key.clone(),
        }
    }

    /// Get the summarize model: env var takes precedence, then config file.
    pub fn summarize_model(&self) -> Option<String> {
        match std::env::var("ENGRAM_SUMMARIZE_MODEL") {
            Ok(m) if !m.is_empty() => Some(m),
            _ => self.settings.summarize_model.clone(),
        }
    }

    /// Set a config value by key name. Returns Err if the key is unknown.
    pub fn set(&mut self, key: &str, value: &str) -> Result<(), CoreError> {
        match key {
            "anthropic_api_key" | "settings.anthropic_api_key" => {
                self.settings.anthropic_api_key = Some(value.to_string());
                Ok(())
            }
            "summarize_model" | "settings.summarize_model" => {
                self.settings.summarize_model = Some(value.to_string());
                Ok(())
            }
            _ => Err(CoreError::Config(format!("Unknown config key: {key}"))),
        }
    }

    /// Get a config value by key name.
    pub fn get(&self, key: &str) -> Option<String> {
        match key {
            "anthropic_api_key" | "settings.anthropic_api_key" => {
                self.settings.anthropic_api_key.clone()
            }
            "summarize_model" | "settings.summarize_model" => self.settings.summarize_model.clone(),
            _ => None,
        }
    }

    /// List all config keys and their current values.
    pub fn list(&self) -> Vec<(&str, Option<String>)> {
        vec![
            ("anthropic_api_key", self.settings.anthropic_api_key.clone()),
            ("summarize_model", self.settings.summarize_model.clone()),
        ]
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

    #[test]
    fn test_settings_roundtrip() {
        let mut config = GlobalConfig::default();
        config.settings.anthropic_api_key = Some("sk-ant-test123".to_string());
        config.settings.summarize_model = Some("claude-sonnet-4-20250514".to_string());

        let serialized = toml::to_string_pretty(&config).unwrap();
        let deserialized: GlobalConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(
            deserialized.settings.anthropic_api_key.as_deref(),
            Some("sk-ant-test123")
        );
        assert_eq!(
            deserialized.settings.summarize_model.as_deref(),
            Some("claude-sonnet-4-20250514")
        );
    }

    #[test]
    fn test_backward_compat_no_settings() {
        // Old-format config with only repos should still parse
        let toml_str = "repos = [\"/tmp/some-repo\"]";
        let config: GlobalConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.repos.len(), 1);
        assert!(config.settings.anthropic_api_key.is_none());
        assert!(config.settings.summarize_model.is_none());
    }

    #[test]
    fn test_set_get_known_keys() {
        let mut config = GlobalConfig::default();
        config.set("anthropic_api_key", "test-key").unwrap();
        assert_eq!(config.get("anthropic_api_key").as_deref(), Some("test-key"));

        config.set("summarize_model", "test-model").unwrap();
        assert_eq!(config.get("summarize_model").as_deref(), Some("test-model"));
    }

    #[test]
    fn test_set_unknown_key_errors() {
        let mut config = GlobalConfig::default();
        assert!(config.set("unknown_key", "value").is_err());
    }

    #[test]
    fn test_list_returns_all_keys() {
        let config = GlobalConfig::default();
        let entries = config.list();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].0, "anthropic_api_key");
        assert_eq!(entries[1].0, "summarize_model");
    }
}
