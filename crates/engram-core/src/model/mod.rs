pub mod engram;
pub mod intent;
pub mod lineage;
pub mod operations;
pub mod token_economics;
pub mod transcript;

pub use engram::{AgentInfo, CaptureMode, EngramId, Manifest};
pub use intent::{DeadEnd, Decision, Intent};
pub use lineage::{Lineage, RelationType, Relationship};
pub use operations::{FileChange, FileChangeType, Operations, ShellCommand, ToolCall};
pub use token_economics::TokenUsage;
pub use transcript::{Role, Transcript, TranscriptContent, TranscriptEntry};

/// All data for a single engram, ready to be stored or returned.
#[derive(Debug, Clone)]
pub struct EngramData {
    pub manifest: Manifest,
    pub intent: Intent,
    pub transcript: Transcript,
    pub operations: Operations,
    pub lineage: Lineage,
}
