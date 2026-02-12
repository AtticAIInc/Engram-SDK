use std::path::Path;

use chrono::Utc;
use sha2::{Digest, Sha256};

use engram_core::model::*;

use crate::error::CaptureError;

/// Import Aider chat history from .aider.chat.history.md
pub struct AiderImporter;

impl AiderImporter {
    /// Discover aider history files in a repo.
    pub fn discover(repo_root: &Path) -> Result<Vec<std::path::PathBuf>, CaptureError> {
        let history_path = repo_root.join(".aider.chat.history.md");
        if history_path.exists() {
            Ok(vec![history_path])
        } else {
            Ok(Vec::new())
        }
    }

    /// Import from .aider.chat.history.md.
    /// Returns one EngramData per session (delimited by "# aider chat started at").
    pub fn import_history(path: &Path) -> Result<Vec<EngramData>, CaptureError> {
        let content = std::fs::read_to_string(path).map_err(CaptureError::Io)?;
        let file_hash = format!("{:x}", Sha256::digest(content.as_bytes()));
        let mut engrams = parse_aider_history(&content)?;
        // Each session in the file gets a unique source hash based on the file + session index
        for (i, engram) in engrams.iter_mut().enumerate() {
            let session_hash = format!("{:x}", Sha256::digest(format!("{file_hash}:{i}")));
            engram.manifest.source_hash = Some(session_hash);
        }
        Ok(engrams)
    }
}

fn parse_aider_history(content: &str) -> Result<Vec<EngramData>, CaptureError> {
    let mut engrams = Vec::new();

    // Split by session headers
    let sessions: Vec<&str> = content.split("# aider chat started at").collect();

    for session_text in sessions.iter().skip(1) {
        // Skip the first empty split
        if let Some(engram) = parse_aider_session(session_text)? {
            engrams.push(engram);
        }
    }

    // If no session headers found, treat the whole file as one session
    if engrams.is_empty() && !content.trim().is_empty() {
        if let Some(engram) = parse_aider_session(content)? {
            engrams.push(engram);
        }
    }

    Ok(engrams)
}

fn parse_aider_session(session_text: &str) -> Result<Option<EngramData>, CaptureError> {
    let mut transcript_entries = Vec::new();
    let mut original_request = String::new();
    let mut total_tokens_sent: u64 = 0;
    let mut total_tokens_received: u64 = 0;
    let mut total_cost: f64 = 0.0;

    let mut current_role: Option<Role> = None;
    let mut current_text = String::new();
    let now = Utc::now();

    for line in session_text.lines() {
        // User message starts with ####
        if let Some(user_msg) = line.strip_prefix("#### ") {
            // Save any previous accumulated text
            flush_entry(&mut transcript_entries, &current_role, &current_text, now);

            let msg = user_msg.trim().to_string();
            if original_request.is_empty() && !msg.is_empty() {
                original_request = msg.clone();
            }

            if !msg.is_empty() {
                transcript_entries.push(TranscriptEntry {
                    timestamp: now,
                    role: Role::User,
                    content: TranscriptContent::Text { text: msg },
                    token_count: None,
                });
            }
            current_role = None;
            current_text.clear();
            continue;
        }

        // Tool output / system messages start with >
        if line.starts_with("> ") {
            let tool_text = line.strip_prefix("> ").unwrap_or(line);

            // Parse token/cost lines: "Tokens: 3.2k sent, 245 received. Cost: $0.01"
            if tool_text.starts_with("Tokens:") {
                parse_token_line(
                    tool_text,
                    &mut total_tokens_sent,
                    &mut total_tokens_received,
                    &mut total_cost,
                );
            }
            continue;
        }

        // Unprefixed text = assistant response
        if !line.is_empty() {
            if current_role.is_none() {
                current_role = Some(Role::Assistant);
            }
            if !current_text.is_empty() {
                current_text.push('\n');
            }
            current_text.push_str(line);
        }
    }

    // Flush last entry
    flush_entry(&mut transcript_entries, &current_role, &current_text, now);

    if transcript_entries.is_empty() {
        return Ok(None);
    }

    let id = EngramId::new();
    let token_usage = TokenUsage {
        input_tokens: total_tokens_sent,
        output_tokens: total_tokens_received,
        total_tokens: total_tokens_sent + total_tokens_received,
        cost_usd: if total_cost > 0.0 {
            Some(total_cost)
        } else {
            None
        },
        ..Default::default()
    };

    let manifest = Manifest {
        id,
        version: 1,
        created_at: now,
        finished_at: Some(now),
        agent: AgentInfo {
            name: "aider".into(),
            model: None,
            version: None,
        },
        git_commits: Vec::new(),
        token_usage,
        summary: if original_request.len() > 100 {
            Some(format!("{}...", &original_request[..100]))
        } else if original_request.is_empty() {
            Some("Imported Aider session".into())
        } else {
            Some(original_request.clone())
        },
        tags: Vec::new(),
        capture_mode: CaptureMode::Import,
        source_hash: None,
    };

    let intent = Intent {
        original_request: if original_request.is_empty() {
            "Imported Aider session".into()
        } else {
            original_request
        },
        interpreted_goal: None,
        summary: manifest.summary.clone(),
        dead_ends: Vec::new(),
        decisions: Vec::new(),
    };

    Ok(Some(EngramData {
        manifest,
        intent,
        transcript: Transcript {
            entries: transcript_entries,
        },
        operations: Operations::default(),
        lineage: Lineage::default(),
    }))
}

fn flush_entry(
    entries: &mut Vec<TranscriptEntry>,
    role: &Option<Role>,
    text: &str,
    ts: chrono::DateTime<Utc>,
) {
    if let Some(role) = role {
        let trimmed = text.trim();
        if !trimmed.is_empty() {
            entries.push(TranscriptEntry {
                timestamp: ts,
                role: role.clone(),
                content: TranscriptContent::Text {
                    text: trimmed.to_string(),
                },
                token_count: None,
            });
        }
    }
}

fn parse_token_line(line: &str, sent: &mut u64, received: &mut u64, cost: &mut f64) {
    // Format: "Tokens: 3.2k sent, 245 received. Cost: $0.01"
    // or: "Tokens: 3200 sent, 245 received."
    // Split on ',' only (not '.') to preserve decimal numbers like 3.2k and $0.01
    for part in line.split(',') {
        let part = part.trim();
        if part.contains("sent") {
            if let Some(num) = extract_token_count(part) {
                *sent += num;
            }
        }
        if part.contains("received") {
            if let Some(num) = extract_token_count(part) {
                *received += num;
            }
        }
        if part.contains('$') {
            if let Some(c) = extract_cost(part) {
                *cost += c;
            }
        }
    }
}

fn extract_token_count(s: &str) -> Option<u64> {
    // Find a number (possibly with k/m suffix)
    for word in s.split_whitespace() {
        let word = word.trim_matches(|c: char| {
            !c.is_ascii_digit() && c != '.' && c != 'k' && c != 'K' && c != 'm' && c != 'M'
        });
        if word.is_empty() {
            continue;
        }
        if let Some(num_str) = word.strip_suffix('k').or_else(|| word.strip_suffix('K')) {
            if let Ok(n) = num_str.parse::<f64>() {
                return Some((n * 1000.0) as u64);
            }
        } else if let Some(num_str) = word.strip_suffix('m').or_else(|| word.strip_suffix('M')) {
            if let Ok(n) = num_str.parse::<f64>() {
                return Some((n * 1_000_000.0) as u64);
            }
        } else if let Ok(n) = word.parse::<u64>() {
            return Some(n);
        }
    }
    None
}

fn extract_cost(s: &str) -> Option<f64> {
    for word in s.split_whitespace() {
        if let Some(num_str) = word.strip_prefix('$') {
            if let Ok(c) = num_str.parse::<f64>() {
                return Some(c);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_aider_history() {
        let content = r#"# aider chat started at 2025-01-15 14:30:22

#### Add a fibonacci function to math_utils.py

I'll add a fibonacci function to `math_utils.py`.

```python
def fibonacci(n):
    if n <= 1:
        return n
    return fibonacci(n-1) + fibonacci(n-2)
```

> Tokens: 3.2k sent, 245 received. Cost: $0.01
> Applied edit to math_utils.py
> Commit abc1234 feat: add fibonacci function

#### Fix the off-by-one error

Let me fix that.

> Tokens: 1.5k sent, 100 received. Cost: $0.005
"#;

        let engrams = parse_aider_history(content).unwrap();
        assert_eq!(engrams.len(), 1);

        let e = &engrams[0];
        assert_eq!(e.manifest.agent.name, "aider");
        assert_eq!(
            e.intent.original_request,
            "Add a fibonacci function to math_utils.py"
        );
        // Should have user + assistant + user + assistant entries
        assert!(e.transcript.entries.len() >= 4);
        assert_eq!(e.manifest.token_usage.input_tokens, 4700); // 3.2k + 1.5k
        assert_eq!(e.manifest.token_usage.output_tokens, 345); // 245 + 100
    }

    #[test]
    fn test_extract_token_count() {
        assert_eq!(extract_token_count("3.2k sent"), Some(3200));
        assert_eq!(extract_token_count("245 received"), Some(245));
        assert_eq!(extract_token_count("1.5k sent"), Some(1500));
        assert_eq!(extract_token_count("no numbers"), None);
    }

    #[test]
    fn test_extract_cost() {
        assert_eq!(extract_cost("Cost: $0.01"), Some(0.01));
        assert_eq!(extract_cost("$0.005"), Some(0.005));
        assert_eq!(extract_cost("no cost"), None);
    }
}
