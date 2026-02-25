use crate::model::{EngramData, FileChangeType};

/// Notes ref namespace used for engram annotations on commits.
pub const ENGRAM_NOTES_REF: &str = "refs/notes/engram";

/// Format an engram as a git note string for attaching to a commit.
pub fn format_note(data: &EngramData) -> String {
    let m = &data.manifest;
    let agent = &m.agent.name;
    let model = m.agent.model.as_deref().unwrap_or("unknown");
    let tokens = m.token_usage.total_tokens;
    let cost = m
        .token_usage
        .effective_cost(m.agent.model.as_deref())
        .map(|c| format!("${c:.2}"))
        .unwrap_or_else(|| "-".to_string());

    let mut out = format!("[{agent}/{model}] {cost} {tokens}tok\n");

    // Intent
    let request = &data.intent.original_request;
    // Truncate long requests for the note
    let display_request = if request.len() > 120 {
        format!("{}...", &request[..117])
    } else {
        request.clone()
    };
    out.push_str(&format!("Intent: \"{display_request}\"\n"));

    // Summary
    if let Some(summary) = data.intent.summary.as_deref().or(m.summary.as_deref()) {
        let display_summary = if summary.len() > 200 {
            format!("{}...", &summary[..197])
        } else {
            summary.to_string()
        };
        out.push_str(&format!("Summary: {display_summary}\n"));
    }

    // Dead ends
    if !data.intent.dead_ends.is_empty() {
        out.push_str("Dead ends:\n");
        for de in &data.intent.dead_ends {
            out.push_str(&format!("  - {}: {}\n", de.approach, de.reason));
        }
    }

    // Decisions
    if !data.intent.decisions.is_empty() {
        out.push_str("Decisions:\n");
        for d in &data.intent.decisions {
            out.push_str(&format!("  - {}: {}\n", d.description, d.rationale));
        }
    }

    // File changes
    if !data.operations.file_changes.is_empty() {
        let files: Vec<String> = data
            .operations
            .file_changes
            .iter()
            .map(|fc| {
                let prefix = match &fc.change_type {
                    FileChangeType::Created => "+",
                    FileChangeType::Modified => "~",
                    FileChangeType::Deleted => "-",
                    FileChangeType::Renamed { .. } => "→",
                };
                // Show just the filename for brevity
                let name = fc.path.rsplit('/').next().unwrap_or(&fc.path);
                format!("{prefix}{name}")
            })
            .collect();
        out.push_str(&format!("Files: {}\n", files.join(" ")));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::*;

    fn make_test_data() -> EngramData {
        EngramData {
            manifest: Manifest {
                id: EngramId::new(),
                version: 1,
                created_at: chrono::Utc::now(),
                finished_at: None,
                agent: AgentInfo {
                    name: "claude-code".into(),
                    model: Some("claude-sonnet-4-5".into()),
                    version: None,
                },
                git_commits: vec!["abc123".into()],
                token_usage: TokenUsage {
                    input_tokens: 1_000_000,
                    output_tokens: 100_000,
                    cache_read_tokens: 0,
                    cache_write_tokens: 0,
                    total_tokens: 1_100_000,
                    cost_usd: None,
                },
                summary: Some("Implemented OAuth2 with PKCE".into()),
                tags: vec![],
                capture_mode: CaptureMode::Import,
                source_hash: None,
            },
            intent: Intent {
                original_request: "Add OAuth2 authentication with PKCE for our SPA".into(),
                interpreted_goal: None,
                summary: Some("Implemented OAuth2 with PKCE using custom middleware".into()),
                dead_ends: vec![DeadEnd {
                    approach: "passport.js".into(),
                    reason: "Middleware conflict with existing stack".into(),
                }],
                decisions: vec![Decision {
                    description: "Custom middleware over Auth0 SDK".into(),
                    rationale: "Auth0 added 2MB to bundle".into(),
                }],
            },
            transcript: Transcript::default(),
            operations: Operations {
                tool_calls: vec![],
                file_changes: vec![
                    FileChange {
                        path: "src/auth.rs".into(),
                        change_type: FileChangeType::Created,
                        lines_added: Some(150),
                        lines_removed: None,
                    },
                    FileChange {
                        path: "src/middleware/oauth.rs".into(),
                        change_type: FileChangeType::Created,
                        lines_added: Some(80),
                        lines_removed: None,
                    },
                    FileChange {
                        path: "src/routes/api.rs".into(),
                        change_type: FileChangeType::Modified,
                        lines_added: Some(10),
                        lines_removed: Some(2),
                    },
                ],
                shell_commands: vec![],
            },
            lineage: Lineage::default(),
        }
    }

    #[test]
    fn test_format_note_basic() {
        let data = make_test_data();
        let note = format_note(&data);

        assert!(note.contains("[claude-code/claude-sonnet-4-5]"));
        assert!(note.contains("$4.50")); // estimated from pricing
        assert!(note.contains("1100000tok"));
        assert!(note.contains("Intent: \"Add OAuth2 authentication with PKCE for our SPA\""));
        assert!(note.contains("Summary: Implemented OAuth2 with PKCE using custom middleware"));
        assert!(note.contains("passport.js: Middleware conflict"));
        assert!(note.contains("Custom middleware over Auth0 SDK: Auth0 added 2MB"));
        assert!(note.contains("+auth.rs"));
        assert!(note.contains("+oauth.rs"));
        assert!(note.contains("~api.rs"));
    }

    #[test]
    fn test_format_note_truncates_long_request() {
        let mut data = make_test_data();
        data.intent.original_request = "x".repeat(200);
        let note = format_note(&data);
        assert!(note.contains("..."));
        // The intent line should be truncated
        let intent_line = note.lines().find(|l| l.starts_with("Intent:")).unwrap();
        assert!(intent_line.len() < 200);
    }

    #[test]
    fn test_format_note_minimal() {
        let data = EngramData {
            manifest: Manifest {
                id: EngramId::new(),
                version: 1,
                created_at: chrono::Utc::now(),
                finished_at: None,
                agent: AgentInfo {
                    name: "test".into(),
                    model: None,
                    version: None,
                },
                git_commits: vec![],
                token_usage: TokenUsage::default(),
                summary: None,
                tags: vec![],
                capture_mode: CaptureMode::Sdk,
                source_hash: None,
            },
            intent: Intent {
                original_request: "do something".into(),
                interpreted_goal: None,
                summary: None,
                dead_ends: vec![],
                decisions: vec![],
            },
            transcript: Transcript::default(),
            operations: Operations::default(),
            lineage: Lineage::default(),
        };
        let note = format_note(&data);
        assert!(note.contains("[test/unknown]"));
        assert!(note.contains("Intent: \"do something\""));
        // Should NOT contain dead ends, decisions, or files sections
        assert!(!note.contains("Dead ends:"));
        assert!(!note.contains("Decisions:"));
        assert!(!note.contains("Files:"));
    }
}
