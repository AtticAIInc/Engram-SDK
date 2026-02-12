use thiserror::Error;

#[derive(Error, Debug)]
pub enum CaptureError {
    #[error("PTY error: {0}")]
    Pty(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Core error: {0}")]
    Core(#[from] engram_core::error::CoreError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Import error: {0}")]
    Import(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Process exited with code {0}")]
    ProcessFailed(i32),
}
