//! Fluent Rust SDK for capturing agent reasoning as Engram data in Git.
//!
//! # Example
//! ```no_run
//! use engram_sdk::EngramSession;
//!
//! let mut session = EngramSession::begin("my-agent", Some("gpt-4"));
//! session.log_message("user", "Add authentication to the API");
//! session.log_message("assistant", "I'll add JWT-based authentication.");
//! session.log_tool_call("write_file", r#"{"path":"src/auth.rs"}"#, Some("Created auth module"));
//! session.log_file_change("src/auth.rs", "created");
//! session.log_rejection("Session-based auth", "Too much server-side state for a stateless API");
//! session.add_tokens(1500, 800, Some(0.02));
//! let id = session.commit(Some("abc123"), Some("Add JWT auth")).unwrap();
//! println!("Engram stored: {id}");
//! ```

mod session;

pub use session::EngramSession;

// Re-export core types that SDK users may need
pub use engram_core::model::{
    AgentInfo, CaptureMode, EngramData, EngramId, FileChange, FileChangeType, Manifest, TokenUsage,
};
pub use engram_core::storage::GitStorage;
