use serde::{Deserialize, Serialize};

use crate::error::CoreError;

/// Structured intent data, stored as intent.md (Markdown) in the engram tree.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Intent {
    pub original_request: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interpreted_goal: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dead_ends: Vec<DeadEnd>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub decisions: Vec<Decision>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeadEnd {
    pub approach: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Decision {
    pub description: String,
    pub rationale: String,
}

impl Intent {
    /// Render as Markdown for storage as intent.md
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# Intent\n\n");
        md.push_str(&self.original_request);
        md.push('\n');

        if let Some(goal) = &self.interpreted_goal {
            md.push_str("\n## Interpreted Goal\n\n");
            md.push_str(goal);
            md.push('\n');
        }

        if let Some(summary) = &self.summary {
            md.push_str("\n## Summary\n\n");
            md.push_str(summary);
            md.push('\n');
        }

        if !self.dead_ends.is_empty() {
            md.push_str("\n## Dead Ends\n\n");
            for de in &self.dead_ends {
                md.push_str(&format!("- **{}**: {}\n", de.approach, de.reason));
            }
        }

        if !self.decisions.is_empty() {
            md.push_str("\n## Decisions\n\n");
            for d in &self.decisions {
                md.push_str(&format!("- **{}**: {}\n", d.description, d.rationale));
            }
        }

        md
    }

    /// Parse from Markdown
    pub fn from_markdown(md: &str) -> Result<Self, CoreError> {
        let mut original_request = String::new();
        let mut interpreted_goal = None;
        let mut summary = None;
        let mut dead_ends = Vec::new();
        let mut decisions = Vec::new();

        let mut current_section = "intent";
        let mut current_content = String::new();

        for line in md.lines() {
            if line.starts_with("# Intent") {
                current_section = "intent";
                current_content.clear();
                continue;
            } else if line.starts_with("## Original Request") {
                // Backward compat: treat as intent section (some SDKs used this heading)
                Self::save_section(
                    current_section,
                    &current_content,
                    &mut original_request,
                    &mut interpreted_goal,
                    &mut summary,
                );
                current_section = "intent";
                current_content.clear();
                continue;
            } else if line.starts_with("## Interpreted Goal") {
                // Save previous section
                Self::save_section(
                    current_section,
                    &current_content,
                    &mut original_request,
                    &mut interpreted_goal,
                    &mut summary,
                );
                current_section = "goal";
                current_content.clear();
                continue;
            } else if line.starts_with("## Summary") {
                Self::save_section(
                    current_section,
                    &current_content,
                    &mut original_request,
                    &mut interpreted_goal,
                    &mut summary,
                );
                current_section = "summary";
                current_content.clear();
                continue;
            } else if line.starts_with("## Dead Ends") {
                Self::save_section(
                    current_section,
                    &current_content,
                    &mut original_request,
                    &mut interpreted_goal,
                    &mut summary,
                );
                current_section = "dead_ends";
                current_content.clear();
                continue;
            } else if line.starts_with("## Decisions") {
                Self::save_section(
                    current_section,
                    &current_content,
                    &mut original_request,
                    &mut interpreted_goal,
                    &mut summary,
                );
                current_section = "decisions";
                current_content.clear();
                continue;
            }

            match current_section {
                "dead_ends" => {
                    if let Some(entry) = line.strip_prefix("- **") {
                        if let Some((approach, reason)) = entry.split_once("**: ") {
                            dead_ends.push(DeadEnd {
                                approach: approach.to_string(),
                                reason: reason.to_string(),
                            });
                        }
                    }
                }
                "decisions" => {
                    if let Some(entry) = line.strip_prefix("- **") {
                        if let Some((desc, rationale)) = entry.split_once("**: ") {
                            decisions.push(Decision {
                                description: desc.to_string(),
                                rationale: rationale.to_string(),
                            });
                        }
                    }
                }
                _ => {
                    if !current_content.is_empty() || !line.is_empty() {
                        if !current_content.is_empty() {
                            current_content.push('\n');
                        }
                        current_content.push_str(line);
                    }
                }
            }
        }

        // Save last section
        Self::save_section(
            current_section,
            &current_content,
            &mut original_request,
            &mut interpreted_goal,
            &mut summary,
        );

        Ok(Intent {
            original_request,
            interpreted_goal,
            summary,
            dead_ends,
            decisions,
        })
    }

    fn save_section(
        section: &str,
        content: &str,
        original_request: &mut String,
        interpreted_goal: &mut Option<String>,
        summary: &mut Option<String>,
    ) {
        let trimmed = content.trim().to_string();
        if trimmed.is_empty() {
            return;
        }
        match section {
            "intent" => *original_request = trimmed,
            "goal" => *interpreted_goal = Some(trimmed),
            "summary" => *summary = Some(trimmed),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_roundtrip() {
        let intent = Intent {
            original_request: "Add OAuth2 authentication".into(),
            interpreted_goal: Some("Implement OAuth2 with PKCE for the SPA".into()),
            summary: Some("Implemented OAuth2 with custom middleware".into()),
            dead_ends: vec![
                DeadEnd {
                    approach: "passport.js".into(),
                    reason: "Middleware conflict with existing stack".into(),
                },
                DeadEnd {
                    approach: "Auth0 SDK".into(),
                    reason: "Added 2MB to bundle".into(),
                },
            ],
            decisions: vec![Decision {
                description: "Custom middleware".into(),
                rationale: "Full control over auth flow".into(),
            }],
        };

        let md = intent.to_markdown();
        let parsed = Intent::from_markdown(&md).unwrap();

        assert_eq!(intent.original_request, parsed.original_request);
        assert_eq!(intent.interpreted_goal, parsed.interpreted_goal);
        assert_eq!(intent.summary, parsed.summary);
        assert_eq!(intent.dead_ends.len(), parsed.dead_ends.len());
        assert_eq!(intent.dead_ends[0].approach, parsed.dead_ends[0].approach);
        assert_eq!(intent.decisions.len(), parsed.decisions.len());
    }

    #[test]
    fn test_minimal_intent() {
        let intent = Intent {
            original_request: "Fix the bug".into(),
            interpreted_goal: None,
            summary: None,
            dead_ends: vec![],
            decisions: vec![],
        };
        let md = intent.to_markdown();
        let parsed = Intent::from_markdown(&md).unwrap();
        assert_eq!(intent.original_request, parsed.original_request);
        assert!(parsed.interpreted_goal.is_none());
        assert!(parsed.dead_ends.is_empty());
    }
}
