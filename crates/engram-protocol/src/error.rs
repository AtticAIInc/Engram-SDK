use engram_core::error::CoreError;

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Git error: {0}")]
    Git(#[from] git2::Error),

    #[error("Core error: {0}")]
    Core(#[from] CoreError),

    #[error("Remote not found: {0}")]
    RemoteNotFound(String),

    #[error("Sync error: {0}")]
    Sync(String),
}
