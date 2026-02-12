use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use portable_pty::{native_pty_system, CommandBuilder, PtySize};

use engram_core::model::FileChange;

use crate::error::CaptureError;

use super::detector::{detect_changes, snapshot_working_tree};

/// Configuration for a PTY-wrapped agent session.
#[derive(Debug, Clone)]
pub struct PtyWrapperConfig {
    pub command: String,
    pub args: Vec<String>,
    pub working_dir: PathBuf,
    pub agent_name: Option<String>,
}

/// Result of a captured PTY session.
#[derive(Debug, Clone)]
pub struct CapturedSession {
    pub raw_output: Vec<u8>,
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub exit_code: Option<u32>,
    pub file_changes: Vec<FileChange>,
    pub command: String,
    pub args: Vec<String>,
}

/// A PTY session that captures agent output and detects file changes.
pub struct PtySession {
    config: PtyWrapperConfig,
    file_snapshot_before: HashMap<PathBuf, Vec<u8>>,
    start_time: DateTime<Utc>,
}

impl PtySession {
    /// Start a new PTY session: snapshot the working tree.
    pub fn start(config: PtyWrapperConfig) -> Result<Self, CaptureError> {
        let snapshot = snapshot_working_tree(&config.working_dir)
            .map_err(|e| CaptureError::Pty(format!("Failed to snapshot working tree: {e}")))?;

        Ok(Self {
            config,
            file_snapshot_before: snapshot,
            start_time: Utc::now(),
        })
    }

    /// Run the session to completion. Spawns the command in a PTY,
    /// passes stdin/stdout through, captures output.
    pub fn run(self) -> Result<CapturedSession, CaptureError> {
        let pty_system = native_pty_system();

        // Get terminal size from current terminal, fall back to defaults
        let (cols, rows) = terminal_size().unwrap_or((80, 24));

        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| CaptureError::Pty(format!("Failed to open PTY: {e}")))?;

        // Build the command
        let mut cmd = CommandBuilder::new(&self.config.command);
        cmd.args(&self.config.args);
        cmd.cwd(&self.config.working_dir);

        // Spawn the child process
        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| CaptureError::Pty(format!("Failed to spawn command: {e}")))?;

        // Drop the slave to avoid hanging
        drop(pair.slave);

        // Set up capture buffer
        let capture_buffer = Arc::new(Mutex::new(Vec::new()));

        // Get reader/writer from master
        let mut reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| CaptureError::Pty(format!("Failed to clone PTY reader: {e}")))?;
        let mut writer = pair
            .master
            .take_writer()
            .map_err(|e| CaptureError::Pty(format!("Failed to take PTY writer: {e}")))?;

        // Reader thread: PTY output -> stdout + capture buffer
        let buf_clone = Arc::clone(&capture_buffer);
        let reader_handle = std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        // Pass through to stdout
                        let _ = std::io::stdout().write_all(&buf[..n]);
                        let _ = std::io::stdout().flush();
                        // Capture
                        if let Ok(mut capture) = buf_clone.lock() {
                            capture.extend_from_slice(&buf[..n]);
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Shutdown flag so we can signal the writer thread to stop
        let shutdown = Arc::new(AtomicBool::new(false));
        let shutdown_writer = Arc::clone(&shutdown);

        // Writer thread: stdin -> PTY
        let writer_handle = std::thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                if shutdown_writer.load(Ordering::Relaxed) {
                    break;
                }
                match std::io::stdin().read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        if writer.write_all(&buf[..n]).is_err() {
                            break;
                        }
                    }
                    Err(_) => break,
                }
            }
        });

        // Wait for child to exit
        let status = child
            .wait()
            .map_err(|e| CaptureError::Pty(format!("Failed to wait for child: {e}")))?;

        // Wait for reader to finish
        let _ = reader_handle.join();

        // Signal writer to stop and give it a brief moment to exit.
        // The writer thread may be blocked on stdin.read(), which we cannot
        // interrupt portably. We accept that it may linger until the process exits.
        shutdown.store(true, Ordering::Relaxed);
        let _ = writer_handle.join();

        let end_time = Utc::now();
        let exit_code = Some(status.exit_code());

        // Detect file changes
        let snapshot_after = snapshot_working_tree(&self.config.working_dir)
            .map_err(|e| CaptureError::Pty(format!("Failed to snapshot working tree: {e}")))?;
        let file_changes = detect_changes(&self.file_snapshot_before, &snapshot_after);

        // Collect captured output
        let raw_output = capture_buffer
            .lock()
            .map(|buf| buf.clone())
            .unwrap_or_default();

        Ok(CapturedSession {
            raw_output,
            start_time: self.start_time,
            end_time,
            exit_code,
            file_changes,
            command: self.config.command,
            args: self.config.args,
        })
    }
}

/// Try to get the current terminal size from environment variables.
fn terminal_size() -> Option<(u16, u16)> {
    // Try COLUMNS and LINES env vars (set by many terminals)
    let cols = std::env::var("COLUMNS")
        .ok()
        .and_then(|v| v.parse::<u16>().ok());
    let rows = std::env::var("LINES")
        .ok()
        .and_then(|v| v.parse::<u16>().ok());

    match (cols, rows) {
        (Some(c), Some(r)) if c > 0 && r > 0 => Some((c, r)),
        _ => None,
    }
}
