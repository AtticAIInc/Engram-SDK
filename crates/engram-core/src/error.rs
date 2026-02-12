use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("Git operation failed: {0}")]
    Git(#[from] git2::Error),

    #[error("Engram not found: {id}")]
    NotFound { id: String },

    #[error("Invalid manifest: {0}")]
    InvalidManifest(#[from] serde_json::Error),

    #[error("Repository not initialized for engram (run `engram init`)")]
    NotInitialized,

    #[error("Config error: {0}")]
    Config(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Missing blob in engram tree: {0}")]
    MissingBlob(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Invalid engram ID: {0}")]
    InvalidId(String),
}
