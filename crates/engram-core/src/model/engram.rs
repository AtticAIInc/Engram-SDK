use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::token_economics::TokenUsage;
use crate::error::CoreError;

/// A unique identifier for an engram.
/// Generated as UUID v4 hex (no dashes), used as the ref path component.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct EngramId(pub String);

impl EngramId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().as_simple().to_string())
    }

    /// Parse and validate an ID string. Must be at least 2 characters.
    pub fn parse(s: impl Into<String>) -> Result<Self, CoreError> {
        let s = s.into();
        if s.len() < 2 {
            return Err(CoreError::InvalidId(format!(
                "ID must be at least 2 characters, got {}",
                s.len()
            )));
        }
        Ok(Self(s))
    }

    /// The 2-char prefix used for fanout in refs/engrams/<ab>/<full-id>
    pub fn fanout_prefix(&self) -> &str {
        if self.0.len() >= 2 {
            &self.0[..2]
        } else {
            "00"
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for EngramId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for EngramId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for EngramId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for EngramId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Compact metadata stored as manifest.json in the engram tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub id: EngramId,
    pub version: u32,
    pub created_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<DateTime<Utc>>,
    pub agent: AgentInfo,
    #[serde(default)]
    pub git_commits: Vec<String>,
    pub token_usage: TokenUsage,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    pub capture_mode: CaptureMode,
    /// SHA-256 of the source file used during import (for deduplication).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentInfo {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    Wrapper,
    Import,
    Sdk,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engram_id_generation() {
        let id = EngramId::new();
        assert_eq!(id.0.len(), 32); // UUID v4 hex, no dashes
        assert_eq!(id.fanout_prefix().len(), 2);
    }

    #[test]
    fn test_engram_id_display() {
        let id = EngramId("abcdef1234567890abcdef1234567890".into());
        assert_eq!(format!("{id}"), "abcdef1234567890abcdef1234567890");
        assert_eq!(id.fanout_prefix(), "ab");
    }

    #[test]
    fn test_engram_id_short_does_not_panic() {
        let short = EngramId("a".into());
        assert_eq!(short.fanout_prefix(), "00");
        let empty = EngramId("".into());
        assert_eq!(empty.fanout_prefix(), "00");
    }

    #[test]
    fn test_engram_id_parse_validation() {
        assert!(EngramId::parse("ab").is_ok());
        assert!(EngramId::parse("abcdef1234").is_ok());
        assert!(EngramId::parse("a").is_err());
        assert!(EngramId::parse("").is_err());
    }

    #[test]
    fn test_manifest_serde_roundtrip() {
        let manifest = Manifest {
            id: EngramId::new(),
            version: 1,
            created_at: Utc::now(),
            finished_at: Some(Utc::now()),
            agent: AgentInfo {
                name: "claude-code".into(),
                model: Some("claude-sonnet-4-5".into()),
                version: Some("2.1.39".into()),
            },
            git_commits: vec!["abc123".into()],
            token_usage: TokenUsage {
                input_tokens: 1000,
                output_tokens: 500,
                total_tokens: 1500,
                cost_usd: Some(0.23),
                ..Default::default()
            },
            summary: Some("Implemented OAuth2".into()),
            tags: vec!["auth".into()],
            capture_mode: CaptureMode::Wrapper,
            source_hash: None,
        };
        let json = serde_json::to_string_pretty(&manifest).unwrap();
        let parsed: Manifest = serde_json::from_str(&json).unwrap();
        assert_eq!(manifest, parsed);
    }
}
