use engram_core::error::CoreError;

#[derive(Debug, thiserror::Error)]
pub enum QueryError {
    #[error("Index error: {0}")]
    Index(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Core error: {0}")]
    Core(#[from] CoreError),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

impl From<tantivy::TantivyError> for QueryError {
    fn from(e: tantivy::TantivyError) -> Self {
        QueryError::Index(e.to_string())
    }
}
