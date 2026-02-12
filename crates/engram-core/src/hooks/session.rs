use std::fs;
use std::io::{Read as _, Seek, SeekFrom, Write as _};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::CoreError;
use crate::model::{AgentInfo, EngramId, TokenUsage};

const SESSION_FILE: &str = "engram-session";

/// Tracks an active recording session. Stored as JSON at `.git/engram-session`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveSession {
    pub engram_id: EngramId,
    pub agent: AgentInfo,
    pub started_at: DateTime<Utc>,
    pub commits: Vec<String>,
    pub token_usage: TokenUsage,
}

impl ActiveSession {
    /// Create a new active session.
    pub fn new(engram_id: EngramId, agent: AgentInfo) -> Self {
        Self {
            engram_id,
            agent,
            started_at: Utc::now(),
            commits: Vec::new(),
            token_usage: TokenUsage::default(),
        }
    }

    /// Path to the session file inside the .git directory.
    fn session_path(git_dir: &Path) -> PathBuf {
        git_dir.join(SESSION_FILE)
    }

    /// Save the session to disk with an exclusive file lock.
    pub fn save(&self, git_dir: &Path) -> Result<(), CoreError> {
        let path = Self::session_path(git_dir);
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| CoreError::Config(format!("Failed to serialize session: {e}")))?;
        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)?;
        fs2::FileExt::lock_exclusive(&file).map_err(CoreError::Io)?;
        (&file).write_all(json.as_bytes())?;
        fs2::FileExt::unlock(&file).map_err(CoreError::Io)?;
        Ok(())
    }

    /// Load an active session if one exists, using a shared file lock.
    pub fn load(git_dir: &Path) -> Option<Self> {
        let path = Self::session_path(git_dir);
        let file = fs::OpenOptions::new().read(true).open(&path).ok()?;
        fs2::FileExt::lock_shared(&file).ok()?;
        let mut data = String::new();
        (&file).read_to_string(&mut data).ok()?;
        fs2::FileExt::unlock(&file).ok();
        serde_json::from_str(&data).ok()
    }

    /// Remove the session file.
    pub fn cleanup(git_dir: &Path) {
        let path = Self::session_path(git_dir);
        let _ = fs::remove_file(path);
    }

    /// Add a commit SHA to the session atomically with an exclusive lock.
    pub fn add_commit(&mut self, sha: &str, git_dir: &Path) -> Result<(), CoreError> {
        let path = Self::session_path(git_dir);
        let file = fs::OpenOptions::new().read(true).write(true).open(&path)?;
        fs2::FileExt::lock_exclusive(&file).map_err(CoreError::Io)?;

        // Re-read under lock to get latest state
        let mut data = String::new();
        (&file).read_to_string(&mut data)?;
        let mut current: ActiveSession = serde_json::from_str(&data)
            .map_err(|e| CoreError::Config(format!("Session parse: {e}")))?;
        current.commits.push(sha.to_string());

        // Write back
        let json = serde_json::to_string_pretty(&current)
            .map_err(|e| CoreError::Config(format!("Session serialize: {e}")))?;
        file.set_len(0)?;
        (&file).seek(SeekFrom::Start(0))?;
        (&file).write_all(json.as_bytes())?;

        // Update self
        self.commits = current.commits;
        fs2::FileExt::unlock(&file).map_err(CoreError::Io)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_session_save_load_cleanup() {
        let tmp = TempDir::new().unwrap();
        let git_dir = tmp.path();

        let session = ActiveSession::new(
            EngramId::new(),
            AgentInfo {
                name: "test-agent".into(),
                model: Some("gpt-4".into()),
                version: None,
            },
        );

        // Save
        session.save(git_dir).unwrap();

        // Load
        let loaded = ActiveSession::load(git_dir).unwrap();
        assert_eq!(loaded.engram_id, session.engram_id);
        assert_eq!(loaded.agent.name, "test-agent");
        assert!(loaded.commits.is_empty());

        // Add commit
        let mut loaded = loaded;
        loaded.add_commit("abc123", git_dir).unwrap();
        let reloaded = ActiveSession::load(git_dir).unwrap();
        assert_eq!(reloaded.commits, vec!["abc123"]);

        // Cleanup
        ActiveSession::cleanup(git_dir);
        assert!(ActiveSession::load(git_dir).is_none());
    }
}
