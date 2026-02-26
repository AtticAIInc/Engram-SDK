//! Update notification system for the engram CLI.
//!
//! Checks GitHub for newer releases, caches the result to avoid repeated
//! network calls, and provides update information for display.

use std::path::PathBuf;
use std::time::Duration;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[cfg(feature = "update-check")]
const GITHUB_API_URL: &str = "https://api.github.com/repos/AtticAIInc/Engram-SDK/releases/latest";
const DEFAULT_TTL_SECS: u64 = 86_400; // 24 hours
#[cfg(feature = "update-check")]
const REQUEST_TIMEOUT_SECS: u64 = 5;
const CACHE_FILE: &str = "update-cache.json";
const CONFIG_DIR: &str = "engram";

/// Information about an available update.
#[derive(Debug, Clone)]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: String,
    pub release_url: String,
}

/// Cached update check result, persisted to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UpdateCache {
    latest_version: String,
    checked_at: DateTime<Utc>,
    release_url: String,
}

/// A parsed semver triple (MAJOR.MINOR.PATCH).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct VersionTriple {
    major: u64,
    minor: u64,
    patch: u64,
}

impl VersionTriple {
    /// Parse a version string like "0.2.0" or "v0.2.0".
    fn parse(s: &str) -> Option<Self> {
        let s = s.strip_prefix('v').unwrap_or(s);
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 3 {
            return None;
        }
        Some(Self {
            major: parts[0].parse().ok()?,
            minor: parts[1].parse().ok()?,
            patch: parts[2].parse().ok()?,
        })
    }
}

/// Returns the path to the update cache file.
fn cache_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join(CONFIG_DIR).join(CACHE_FILE))
}

/// Read the cached update check result from disk.
fn read_cache() -> Option<UpdateCache> {
    let path = cache_path()?;
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Write the update cache to disk (best-effort).
fn write_cache(cache: &UpdateCache) {
    if let Some(path) = cache_path() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(cache) {
            let _ = std::fs::write(&path, json);
        }
    }
}

/// Return the TTL from env or fall back to 24h.
fn ttl() -> Duration {
    std::env::var("ENGRAM_UPDATE_CHECK_INTERVAL")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(DEFAULT_TTL_SECS))
}

/// Check whether updates are disabled via env or config.
pub fn is_update_check_disabled() -> bool {
    // Env var override
    if std::env::var("ENGRAM_NO_UPDATE_CHECK")
        .ok()
        .filter(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .is_some()
    {
        return true;
    }
    // Config override (best-effort load)
    if let Ok(config) = crate::config::GlobalConfig::load() {
        if let Some(ref enabled) = config.settings.update_check_enabled {
            if enabled.eq_ignore_ascii_case("false") || enabled == "0" {
                return true;
            }
        }
    }
    false
}

/// Fetch the latest release tag from GitHub.
/// Returns (tag_name, html_url) or None on any failure.
#[cfg(feature = "update-check")]
fn fetch_latest_release() -> Option<(String, String)> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .user_agent(format!("engram-cli/{}", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;

    let resp = client
        .get(GITHUB_API_URL)
        .header("Accept", "application/vnd.github+json")
        .send()
        .ok()?;

    if !resp.status().is_success() {
        tracing::debug!("GitHub API returned status: {}", resp.status());
        return None;
    }

    let json: serde_json::Value = resp.json().ok()?;
    let tag = json["tag_name"].as_str()?.to_string();
    let url = json["html_url"]
        .as_str()
        .unwrap_or("https://github.com/AtticAIInc/Engram-SDK/releases")
        .to_string();
    Some((tag, url))
}

#[cfg(not(feature = "update-check"))]
fn fetch_latest_release() -> Option<(String, String)> {
    None
}

/// Perform the update check, respecting cache TTL.
/// `force` = true ignores the TTL (used by `engram version`).
/// Returns `Some(UpdateInfo)` if a newer version is available.
pub fn check_for_update(current_version: &str, force: bool) -> Option<UpdateInfo> {
    if !force && is_update_check_disabled() {
        return None;
    }

    let current = VersionTriple::parse(current_version)?;

    // Check cache
    if !force {
        if let Some(cache) = read_cache() {
            let age = Utc::now().signed_duration_since(cache.checked_at);
            if age.num_seconds() >= 0 && age.to_std().ok().map(|d| d < ttl()).unwrap_or(false) {
                // Cache is fresh -- use it
                let cached = VersionTriple::parse(&cache.latest_version)?;
                if cached > current {
                    return Some(UpdateInfo {
                        current_version: current_version.to_string(),
                        latest_version: cache.latest_version,
                        release_url: cache.release_url,
                    });
                }
                return None;
            }
        }
    }

    // Fetch from GitHub
    let (tag, url) = fetch_latest_release()?;
    let latest = VersionTriple::parse(&tag)?;

    // Update cache
    let cache = UpdateCache {
        latest_version: tag.strip_prefix('v').unwrap_or(&tag).to_string(),
        checked_at: Utc::now(),
        release_url: url.clone(),
    };
    write_cache(&cache);

    if latest > current {
        Some(UpdateInfo {
            current_version: current_version.to_string(),
            latest_version: cache.latest_version,
            release_url: url,
        })
    } else {
        None
    }
}

/// Format the update notification for display on stderr.
pub fn format_update_notice(info: &UpdateInfo) -> String {
    format!(
        "\n---\nA new version of engram is available: {} -> {}\nUpdate: {}\n---",
        info.current_version, info.latest_version, info.release_url
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parse() {
        assert_eq!(
            VersionTriple::parse("0.2.0"),
            Some(VersionTriple {
                major: 0,
                minor: 2,
                patch: 0
            })
        );
        assert_eq!(
            VersionTriple::parse("v1.3.7"),
            Some(VersionTriple {
                major: 1,
                minor: 3,
                patch: 7
            })
        );
        assert_eq!(VersionTriple::parse("invalid"), None);
        assert_eq!(VersionTriple::parse("1.2"), None);
    }

    #[test]
    fn test_version_ordering() {
        let v020 = VersionTriple::parse("0.2.0").unwrap();
        let v030 = VersionTriple::parse("0.3.0").unwrap();
        let v021 = VersionTriple::parse("0.2.1").unwrap();
        let v100 = VersionTriple::parse("1.0.0").unwrap();
        assert!(v030 > v020);
        assert!(v021 > v020);
        assert!(v100 > v030);
        assert_eq!(v020, VersionTriple::parse("v0.2.0").unwrap());
    }

    #[test]
    fn test_format_update_notice() {
        let info = UpdateInfo {
            current_version: "0.2.0".to_string(),
            latest_version: "0.3.0".to_string(),
            release_url: "https://github.com/AtticAIInc/Engram-SDK/releases/tag/v0.3.0".to_string(),
        };
        let notice = format_update_notice(&info);
        assert!(notice.contains("0.2.0 -> 0.3.0"));
        assert!(notice.contains("https://"));
    }

    #[test]
    fn test_disabled_via_env() {
        // Save and restore to avoid test interference
        let prev = std::env::var("ENGRAM_NO_UPDATE_CHECK").ok();
        std::env::set_var("ENGRAM_NO_UPDATE_CHECK", "1");
        assert!(is_update_check_disabled());
        match prev {
            Some(v) => std::env::set_var("ENGRAM_NO_UPDATE_CHECK", v),
            None => std::env::remove_var("ENGRAM_NO_UPDATE_CHECK"),
        }
    }

    #[test]
    fn test_cache_roundtrip() {
        let tmp = tempfile::TempDir::new().unwrap();
        // Write/read cache directly to a temp path instead of using
        // the global cache_path() which varies by platform.
        let path = tmp.path().join("engram").join("update-cache.json");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        let cache = UpdateCache {
            latest_version: "0.3.0".to_string(),
            checked_at: Utc::now(),
            release_url: "https://example.com".to_string(),
        };
        let json = serde_json::to_string_pretty(&cache).unwrap();
        std::fs::write(&path, &json).unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let loaded: UpdateCache = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.latest_version, "0.3.0");
        assert_eq!(loaded.release_url, "https://example.com");
    }
}
