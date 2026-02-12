mod detector;
mod wrapper;

pub use detector::{detect_changes, snapshot_working_tree};
pub use wrapper::{CapturedSession, PtySession, PtyWrapperConfig};
