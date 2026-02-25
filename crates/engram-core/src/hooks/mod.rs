pub mod claude_code;
pub mod handlers;
pub mod installer;
pub mod session;

pub use claude_code::{install_claude_code_hook, uninstall_claude_code_hook};
pub use handlers::{handle_post_commit, handle_prepare_commit_msg};
pub use installer::{install_hooks, uninstall_hooks};
pub use session::ActiveSession;
