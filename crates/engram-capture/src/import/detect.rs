use std::path::{Path, PathBuf};

use crate::error::CaptureError;
use crate::import::aider::AiderImporter;
use crate::import::claude_code::ClaudeCodeImporter;

/// A discovered import source.
#[derive(Debug, Clone)]
pub enum ImportSource {
    ClaudeCode { session_path: PathBuf },
    Aider { history_path: PathBuf },
}

impl ImportSource {
    pub fn description(&self) -> String {
        match self {
            Self::ClaudeCode { session_path } => {
                format!("Claude Code session: {}", session_path.display())
            }
            Self::Aider { history_path } => {
                format!("Aider history: {}", history_path.display())
            }
        }
    }

    pub fn format_name(&self) -> &'static str {
        match self {
            Self::ClaudeCode { .. } => "claude-code",
            Self::Aider { .. } => "aider",
        }
    }
}

/// Auto-detect importable session sources for the given repo root.
pub fn detect_sources(repo_root: &Path) -> Result<Vec<ImportSource>, CaptureError> {
    let mut sources = Vec::new();

    // Check for Claude Code sessions
    if let Ok(sessions) = ClaudeCodeImporter::discover_sessions(repo_root) {
        for path in sessions {
            sources.push(ImportSource::ClaudeCode { session_path: path });
        }
    }

    // Check for Aider history
    if let Ok(histories) = AiderImporter::discover(repo_root) {
        for path in histories {
            sources.push(ImportSource::Aider { history_path: path });
        }
    }

    Ok(sources)
}
