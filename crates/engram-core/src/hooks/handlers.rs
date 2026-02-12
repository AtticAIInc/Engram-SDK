use std::fs;
use std::path::Path;

use crate::error::CoreError;

use super::session::ActiveSession;

/// Handle the `prepare-commit-msg` hook.
///
/// If an active engram session exists, appends Engram trailers to the commit message.
pub fn handle_prepare_commit_msg(msg_file: &Path, git_dir: &Path) -> Result<(), CoreError> {
    let session = match ActiveSession::load(git_dir) {
        Some(s) => s,
        None => return Ok(()), // No active session, nothing to do
    };

    let mut msg = fs::read_to_string(msg_file)?;

    // Don't add trailers if they're already there
    if msg.contains("Engram-Id:") {
        return Ok(());
    }

    // Ensure blank line before trailers
    if !msg.ends_with('\n') {
        msg.push('\n');
    }
    msg.push('\n');

    msg.push_str(&format!("Engram-Id: {}\n", session.engram_id.as_str()));
    msg.push_str(&format!(
        "Engram-Agent: {}{}\n",
        session.agent.name,
        session
            .agent
            .model
            .as_ref()
            .map(|m| format!("/{m}"))
            .unwrap_or_default()
    ));
    msg.push_str(&format!(
        "Engram-Tokens: {}\n",
        session.token_usage.total_tokens
    ));
    if let Some(cost) = session.token_usage.cost_usd {
        msg.push_str(&format!("Engram-Cost: ${cost:.2}\n"));
    }

    fs::write(msg_file, msg)?;
    Ok(())
}

/// Handle the `post-commit` hook.
///
/// If an active engram session exists, records the new commit SHA.
pub fn handle_post_commit(git_dir: &Path) -> Result<(), CoreError> {
    let mut session = match ActiveSession::load(git_dir) {
        Some(s) => s,
        None => return Ok(()),
    };

    // Read HEAD to get the new commit SHA
    let repo = git2::Repository::open(git_dir.parent().unwrap_or(git_dir))?;
    let head = repo.head()?;
    let sha = head.target().map(|oid| oid.to_string()).unwrap_or_default();

    if !sha.is_empty() {
        session.add_commit(&sha, git_dir)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{AgentInfo, EngramId, TokenUsage};
    use tempfile::TempDir;

    fn make_session() -> ActiveSession {
        let mut session = ActiveSession::new(
            EngramId::new(),
            AgentInfo {
                name: "claude-code".into(),
                model: Some("claude-sonnet-4-5".into()),
                version: None,
            },
        );
        session.token_usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 500,
            total_tokens: 1500,
            cost_usd: Some(0.02),
            ..Default::default()
        };
        session
    }

    #[test]
    fn test_prepare_commit_msg_no_session() {
        let tmp = TempDir::new().unwrap();
        let msg_file = tmp.path().join("COMMIT_EDITMSG");
        fs::write(&msg_file, "Initial commit\n").unwrap();

        // No session file — should be a no-op
        handle_prepare_commit_msg(&msg_file, tmp.path()).unwrap();

        let content = fs::read_to_string(&msg_file).unwrap();
        assert_eq!(content, "Initial commit\n");
    }

    #[test]
    fn test_prepare_commit_msg_with_session() {
        let tmp = TempDir::new().unwrap();
        let msg_file = tmp.path().join("COMMIT_EDITMSG");
        fs::write(&msg_file, "Add auth feature\n").unwrap();

        let session = make_session();
        session.save(tmp.path()).unwrap();

        handle_prepare_commit_msg(&msg_file, tmp.path()).unwrap();

        let content = fs::read_to_string(&msg_file).unwrap();
        assert!(content.contains("Engram-Id:"));
        assert!(content.contains("Engram-Agent: claude-code/claude-sonnet-4-5"));
        assert!(content.contains("Engram-Tokens: 1500"));
        assert!(content.contains("Engram-Cost: $0.02"));
    }

    #[test]
    fn test_prepare_commit_msg_idempotent() {
        let tmp = TempDir::new().unwrap();
        let msg_file = tmp.path().join("COMMIT_EDITMSG");
        fs::write(&msg_file, "Add auth\n\nEngram-Id: existing\n").unwrap();

        let session = make_session();
        session.save(tmp.path()).unwrap();

        handle_prepare_commit_msg(&msg_file, tmp.path()).unwrap();

        let content = fs::read_to_string(&msg_file).unwrap();
        // Should not duplicate trailers
        assert_eq!(content.matches("Engram-Id:").count(), 1);
    }
}
