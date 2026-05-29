//! Persistent event log for best-effort background operations.
//!
//! Git hooks and auto-capture run detached from any terminal, so failures that
//! are merely traced to stderr (at `debug` level, below the default `warn`
//! filter) are invisible to users. A repo whose auto-capture is silently broken
//! looks identical to one that simply has no reasoning to capture.
//!
//! This module appends timestamped events to `<git_dir>/engram.log` so those
//! otherwise-silent outcomes leave a breadcrumb that `engram doctor` can surface.
//! Every function here is best-effort: it never panics and never propagates
//! errors, so logging can never break the git operation that triggered it.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

/// Rotate the log once it exceeds this size, keeping one `.old` generation.
const MAX_LOG_BYTES: u64 = 256 * 1024;

/// Log file name, written inside the git directory (`.git/engram.log`).
const LOG_FILE: &str = "engram.log";
const LOG_FILE_OLD: &str = "engram.log.old";

/// Severity of a logged event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    /// A normal, successful outcome worth recording (e.g. a capture or push).
    Info,
    /// A recoverable problem the user may want to know about.
    Warn,
    /// A failure that prevented an operation from completing.
    Error,
}

impl Level {
    fn as_str(self) -> &'static str {
        match self {
            Level::Info => "INFO",
            Level::Warn => "WARN",
            Level::Error => "ERROR",
        }
    }

    fn parse(s: &str) -> Option<Level> {
        match s {
            "INFO" => Some(Level::Info),
            "WARN" => Some(Level::Warn),
            "ERROR" => Some(Level::Error),
            _ => None,
        }
    }
}

impl fmt::Display for Level {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A single parsed log entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// RFC 3339 timestamp string, exactly as written.
    pub timestamp: String,
    pub level: Level,
    pub message: String,
}

/// Append an event to the repo's event log. Best-effort: all errors are swallowed.
///
/// `git_dir` is the path to the `.git` directory (i.e. `storage.repo().path()`).
pub fn log(git_dir: &Path, level: Level, message: impl AsRef<str>) {
    let path = git_dir.join(LOG_FILE);

    // Rotate when the log grows too large so it can never grow unbounded.
    if let Ok(meta) = fs::metadata(&path) {
        if meta.len() > MAX_LOG_BYTES {
            let _ = fs::rename(&path, git_dir.join(LOG_FILE_OLD));
        }
    }

    let line = format!(
        "{} [{}] {}\n",
        chrono::Utc::now().to_rfc3339(),
        level,
        // Keep each event on a single line so the format stays parseable.
        message.as_ref().replace('\n', " ")
    );

    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}

/// Convenience wrapper for [`Level::Info`].
pub fn info(git_dir: &Path, message: impl AsRef<str>) {
    log(git_dir, Level::Info, message);
}

/// Convenience wrapper for [`Level::Warn`].
pub fn warn(git_dir: &Path, message: impl AsRef<str>) {
    log(git_dir, Level::Warn, message);
}

/// Convenience wrapper for [`Level::Error`].
pub fn error(git_dir: &Path, message: impl AsRef<str>) {
    log(git_dir, Level::Error, message);
}

/// Parse a single log line of the form `<timestamp> [<LEVEL>] <message>`.
fn parse_line(line: &str) -> Option<Entry> {
    let line = line.trim_end();
    if line.is_empty() {
        return None;
    }
    let (timestamp, rest) = line.split_once(' ')?;
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('[')?;
    let (level_str, message) = rest.split_once(']')?;
    let level = Level::parse(level_str)?;
    Some(Entry {
        timestamp: timestamp.to_string(),
        level,
        message: message.trim_start().to_string(),
    })
}

/// Read up to `max_lines` of the most recent events, oldest first.
///
/// Returns an empty vector if the log does not exist or cannot be read.
/// Unparseable lines are skipped.
pub fn read_recent(git_dir: &Path, max_lines: usize) -> Vec<Entry> {
    let path = git_dir.join(LOG_FILE);
    let contents = match fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut entries: Vec<Entry> = contents.lines().filter_map(parse_line).collect();
    if entries.len() > max_lines {
        entries.drain(0..entries.len() - max_lines);
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_log_and_read_roundtrip() {
        let tmp = TempDir::new().unwrap();
        info(tmp.path(), "captured engram abc123");
        warn(tmp.path(), "summarization skipped");
        error(tmp.path(), "store failed: disk full");

        let entries = read_recent(tmp.path(), 10);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].level, Level::Info);
        assert_eq!(entries[0].message, "captured engram abc123");
        assert_eq!(entries[1].level, Level::Warn);
        assert_eq!(entries[2].level, Level::Error);
        assert_eq!(entries[2].message, "store failed: disk full");
    }

    #[test]
    fn test_read_recent_caps_lines() {
        let tmp = TempDir::new().unwrap();
        for i in 0..20 {
            info(tmp.path(), format!("event {i}"));
        }
        let entries = read_recent(tmp.path(), 5);
        assert_eq!(entries.len(), 5);
        // Oldest-first, so the last 5 of 20 are events 15..=19.
        assert_eq!(entries[0].message, "event 15");
        assert_eq!(entries[4].message, "event 19");
    }

    #[test]
    fn test_read_recent_missing_file() {
        let tmp = TempDir::new().unwrap();
        assert!(read_recent(tmp.path(), 10).is_empty());
    }

    #[test]
    fn test_newlines_collapsed_to_single_line() {
        let tmp = TempDir::new().unwrap();
        error(tmp.path(), "line one\nline two\nline three");
        let entries = read_recent(tmp.path(), 10);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "line one line two line three");
    }

    #[test]
    fn test_rotation_keeps_log_bounded() {
        let tmp = TempDir::new().unwrap();
        // Write enough data to exceed MAX_LOG_BYTES and trigger a rotation.
        let big = "x".repeat(1024);
        for _ in 0..(MAX_LOG_BYTES / 1024 + 8) {
            info(tmp.path(), &big);
        }
        // After rotation, the live log is small again and an .old file exists.
        let live = fs::metadata(tmp.path().join(LOG_FILE)).unwrap().len();
        assert!(live <= MAX_LOG_BYTES, "live log should be bounded: {live}");
        assert!(tmp.path().join(LOG_FILE_OLD).exists());
    }

    #[test]
    fn test_parse_line_tolerates_garbage() {
        assert!(parse_line("not a valid line").is_none());
        assert!(parse_line("").is_none());
        assert!(parse_line("2024-01-01T00:00:00Z [BOGUS] hi").is_none());
        let ok = parse_line("2024-01-01T00:00:00Z [INFO] hello world").unwrap();
        assert_eq!(ok.level, Level::Info);
        assert_eq!(ok.message, "hello world");
    }
}
