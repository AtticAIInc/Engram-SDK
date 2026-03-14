use engram_core::model::{DeadEnd, Decision};

/// Best-effort extraction of reasoning insights from raw PTY output.
///
/// Uses heuristic pattern matching to find rejected approaches (dead ends)
/// and architectural decisions from agent output text.
pub fn extract_insights(raw_output: &[u8]) -> ExtractedInsights {
    let text = String::from_utf8_lossy(raw_output);
    let mut dead_ends = Vec::new();
    let mut decisions = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.len() < 10 {
            continue;
        }
        // Skip lines that look like markdown formatting, code, or structured content
        if trimmed.starts_with('#')
            || trimmed.starts_with('|')
            || trimmed.starts_with("```")
            || trimmed.starts_with("- **")
            || trimmed.starts_with("**")
            || trimmed.contains("](")
        {
            continue;
        }
        let lower = trimmed.to_lowercase();

        // Dead end patterns
        if let Some(de) = try_extract_dead_end(&lower, trimmed) {
            dead_ends.push(de);
        }

        // Decision patterns
        if let Some(d) = try_extract_decision(&lower, trimmed) {
            decisions.push(d);
        }
    }

    // Deduplicate by approach/description (sort first so dedup_by catches all)
    dead_ends.sort_by(|a, b| a.approach.cmp(&b.approach));
    dead_ends.dedup_by(|a, b| a.approach == b.approach);
    decisions.sort_by(|a, b| a.description.cmp(&b.description));
    decisions.dedup_by(|a, b| a.description == b.description);

    ExtractedInsights {
        dead_ends,
        decisions,
    }
}

pub struct ExtractedInsights {
    pub dead_ends: Vec<DeadEnd>,
    pub decisions: Vec<Decision>,
}

/// Extract the portion after a keyword, up to sentence-ending punctuation or end of string.
fn extract_after(text: &str, pos: usize, keyword_len: usize) -> &str {
    let start = pos + keyword_len;
    if start >= text.len() {
        return "";
    }
    text[start..].trim()
}

fn try_extract_dead_end(lower: &str, original: &str) -> Option<DeadEnd> {
    // Only process reasonably short lines to avoid matching prose paragraphs
    if original.len() > 200 {
        return None;
    }

    // Pattern: "tried X but Y"
    if let Some(rest) = lower.strip_prefix("tried ") {
        if let Some((approach, reason)) = rest.split_once(" but ") {
            return Some(DeadEnd {
                approach: approach.trim().to_string(),
                reason: reason.trim().to_string(),
            });
        }
    }

    // Pattern: "rejected X because Y" or "rejected X: Y"
    if let Some(rest) = lower.strip_prefix("rejected ") {
        if let Some((approach, reason)) = rest
            .split_once(" because ")
            .or_else(|| rest.split_once(": "))
        {
            return Some(DeadEnd {
                approach: approach.trim().to_string(),
                reason: reason.trim().to_string(),
            });
        }
    }

    // Pattern: "X didn't work because Y" / "X doesn't work because Y"
    for needle in &[" didn't work", " doesn't work", " won't work", " isn't working"] {
        if let Some(pos) = lower.find(needle) {
            let approach = &original[..pos];
            let after_pos = pos + needle.len();
            let reason = lower
                .get(after_pos..)
                .and_then(|r| {
                    r.strip_prefix(" because ")
                        .or(r.strip_prefix(": "))
                        .or(r.strip_prefix(" — "))
                        .or(r.strip_prefix(" - "))
                })
                .unwrap_or("did not work as expected");
            if !approach.is_empty() && approach.len() < 80 {
                return Some(DeadEnd {
                    approach: approach.trim().to_string(),
                    reason: reason.trim().to_string(),
                });
            }
        }
    }

    // Pattern: "instead of X" (looser, only if line is short enough to be a summary)
    if lower.contains("instead of ") && original.len() < 120 {
        if let Some(pos) = lower.find("instead of ") {
            let approach = &original[(pos + "instead of ".len())..];
            let reason = &original[..pos];
            if !approach.is_empty() && approach.len() < 80 && !reason.is_empty() {
                return Some(DeadEnd {
                    approach: approach.trim().trim_end_matches('.').to_string(),
                    reason: reason.trim().to_string(),
                });
            }
        }
    }

    // Pattern: "X rather than Y" / "X over Y"
    if original.len() < 150 {
        for separator in &[" rather than ", " over "] {
            if let Some(pos) = lower.find(separator) {
                // Only if preceded by "using" or "went with" or "chose" or "use"
                let before = &lower[..pos];
                if before.contains("using ")
                    || before.contains("went with ")
                    || before.contains("use ")
                    || before.contains("chose ")
                    || before.contains("going with ")
                {
                    let approach = extract_after(original, pos, separator.len());
                    if !approach.is_empty() && approach.len() < 80 {
                        return Some(DeadEnd {
                            approach: approach.trim().trim_end_matches('.').to_string(),
                            reason: original[..pos].trim().to_string(),
                        });
                    }
                }
            }
        }
    }

    // Pattern: "X failed" / "X broke" / "X caused issues"
    for needle in &[" failed", " broke", " caused issues", " caused errors", " caused problems"] {
        if let Some(pos) = lower.find(needle) {
            let approach = &original[..pos];
            let reason_start = pos + needle.len();
            let reason = if reason_start < lower.len() {
                let rest = lower[reason_start..].trim();
                if rest.is_empty() || rest == "." {
                    needle.trim().to_string()
                } else {
                    rest.trim_start_matches([',', ':', ' '])
                        .to_string()
                }
            } else {
                needle.trim().to_string()
            };
            if !approach.is_empty() && approach.trim().len() < 80 && approach.trim().len() > 2 {
                return Some(DeadEnd {
                    approach: approach.trim().to_string(),
                    reason,
                });
            }
        }
    }

    // Pattern: "I considered X but" / "I thought about X but"
    for prefix in &["i considered ", "i thought about ", "i looked into ", "i explored "] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            if let Some((approach, reason)) = rest
                .split_once(" but ")
                .or_else(|| rest.split_once(" however "))
            {
                return Some(DeadEnd {
                    approach: approach.trim().to_string(),
                    reason: reason.trim().to_string(),
                });
            }
        }
    }

    // Pattern: "the issue with X is Y" / "the problem with X is Y"
    for prefix in &["the issue with ", "the problem with "] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            if let Some((approach, reason)) = rest.split_once(" is ") {
                return Some(DeadEnd {
                    approach: approach.trim().to_string(),
                    reason: reason.trim().to_string(),
                });
            }
        }
    }

    // Pattern: "initially tried X" / "first tried X"
    for prefix in &["initially tried ", "first tried ", "originally tried "] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            if let Some((approach, reason)) = rest
                .split_once(" but ")
                .or_else(|| rest.split_once(" however "))
            {
                return Some(DeadEnd {
                    approach: approach.trim().to_string(),
                    reason: reason.trim().to_string(),
                });
            }
        }
    }

    // Pattern: "switched from X to Y" / "moved from X to Y" / "switched to Y from X"
    for prefix in &["switched from ", "moved from ", "changed from ", "migrated from "] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            if let Some((approach, _)) = rest.split_once(" to ") {
                return Some(DeadEnd {
                    approach: approach.trim().to_string(),
                    reason: original.to_string(),
                });
            }
        }
    }

    // Pattern: "abandoned X" / "gave up on X" / "dropped X"
    for prefix in &["abandoned ", "gave up on ", "dropped "] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            let (approach, reason) = if let Some(split) = rest
                .split_once(" because ")
                .or_else(|| rest.split_once(" due to "))
                .or_else(|| rest.split_once(": "))
            {
                split
            } else {
                (rest.trim(), "approach was abandoned")
            };
            if !approach.is_empty() && approach.len() < 80 {
                return Some(DeadEnd {
                    approach: approach.trim().trim_end_matches('.').to_string(),
                    reason: reason.trim().to_string(),
                });
            }
        }
    }

    None
}

fn try_extract_decision(lower: &str, original: &str) -> Option<Decision> {
    // Only process reasonably short lines
    if original.len() > 200 {
        return None;
    }

    // Pattern: "decided to X because Y"
    if let Some(rest) = lower.strip_prefix("decided to ") {
        if let Some((desc, rationale)) = rest.split_once(" because ") {
            return Some(Decision {
                description: desc.trim().to_string(),
                rationale: rationale.trim().to_string(),
            });
        }
    }

    // Pattern: "chose X over Y"
    if lower.starts_with("chose ") {
        if let Some((desc, _)) = lower
            .strip_prefix("chose ")
            .and_then(|r| r.split_once(" over "))
        {
            return Some(Decision {
                description: desc.trim().to_string(),
                rationale: original.to_string(),
            });
        }
    }

    // Pattern: "going with X because Y" / "went with X because Y"
    for prefix in &["going with ", "went with ", "opting for ", "opted for "] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            if let Some((desc, rationale)) = rest
                .split_once(" because ")
                .or_else(|| rest.split_once(" since "))
                .or_else(|| rest.split_once(" as it "))
                .or_else(|| rest.split_once(" for "))
            {
                return Some(Decision {
                    description: desc.trim().to_string(),
                    rationale: rationale.trim().to_string(),
                });
            }
        }
    }

    // Pattern: "I'll use X because Y" / "using X because Y" / "let's use X because Y"
    for prefix in &["i'll use ", "using ", "let's use ", "we'll use ", "i will use "] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            if let Some((desc, rationale)) = rest
                .split_once(" because ")
                .or_else(|| rest.split_once(" since "))
                .or_else(|| rest.split_once(" as it "))
            {
                if desc.len() < 80 {
                    return Some(Decision {
                        description: desc.trim().to_string(),
                        rationale: rationale.trim().to_string(),
                    });
                }
            }
        }
    }

    // Pattern: "the best approach is X because Y"
    for prefix in &[
        "the best approach is ",
        "the right approach is ",
        "the better approach is ",
    ] {
        if let Some(rest) = lower.strip_prefix(prefix) {
            if let Some((desc, rationale)) = rest
                .split_once(" because ")
                .or_else(|| rest.split_once(" since "))
            {
                return Some(Decision {
                    description: desc.trim().to_string(),
                    rationale: rationale.trim().to_string(),
                });
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_dead_end_tried_but() {
        let output = b"tried passport.js but middleware conflict with existing stack\n";
        let insights = extract_insights(output);
        assert_eq!(insights.dead_ends.len(), 1);
        assert_eq!(insights.dead_ends[0].approach, "passport.js");
        assert!(insights.dead_ends[0].reason.contains("middleware conflict"));
    }

    #[test]
    fn test_extract_dead_end_rejected() {
        let output = b"rejected Auth0 SDK because it added 2MB to the bundle\n";
        let insights = extract_insights(output);
        assert_eq!(insights.dead_ends.len(), 1);
        assert_eq!(insights.dead_ends[0].approach, "auth0 sdk");
    }

    #[test]
    fn test_extract_decision() {
        let output = b"decided to use custom middleware because full control over auth flow\n";
        let insights = extract_insights(output);
        assert_eq!(insights.decisions.len(), 1);
        assert_eq!(insights.decisions[0].description, "use custom middleware");
    }

    #[test]
    fn test_no_false_positives_on_code() {
        let output = b"fn main() {\n    println!(\"Hello, world!\");\n}\n";
        let insights = extract_insights(output);
        assert!(insights.dead_ends.is_empty());
        assert!(insights.decisions.is_empty());
    }

    #[test]
    fn test_empty_output() {
        let insights = extract_insights(b"");
        assert!(insights.dead_ends.is_empty());
        assert!(insights.decisions.is_empty());
    }

    #[test]
    fn test_extract_dead_end_rejected_with_colon() {
        let output = b"rejected SQLite: not suitable for concurrent writes\n";
        let insights = extract_insights(output);
        assert_eq!(insights.dead_ends.len(), 1);
        assert_eq!(insights.dead_ends[0].approach, "sqlite");
        assert!(insights.dead_ends[0].reason.contains("concurrent writes"));
    }

    #[test]
    fn test_extract_dead_end_didnt_work() {
        let output = b"Using raw SQL didn't work because of injection risks\n";
        let insights = extract_insights(output);
        assert_eq!(insights.dead_ends.len(), 1);
        assert_eq!(insights.dead_ends[0].approach, "Using raw SQL");
        assert!(insights.dead_ends[0].reason.contains("injection risks"));
    }

    #[test]
    fn test_extract_dead_end_instead_of() {
        let output = b"Used connection pooling instead of raw connections\n";
        let insights = extract_insights(output);
        assert_eq!(insights.dead_ends.len(), 1);
        assert_eq!(insights.dead_ends[0].approach, "raw connections");
    }

    #[test]
    fn test_extract_decision_chose_over() {
        let output = b"chose tokio over async-std for better ecosystem support\n";
        let insights = extract_insights(output);
        assert_eq!(insights.decisions.len(), 1);
        assert_eq!(insights.decisions[0].description, "tokio");
    }

    #[test]
    fn test_skips_markdown_lines() {
        let output = b"# tried something but it failed\n\
            | tried table but row issue |\n\
            ```tried code but block issue```\n\
            - **tried bold but format issue**\n\
            **tried bold line but issue**\n\
            Check [this link](https://example.com) tried link but issue\n";
        let insights = extract_insights(output);
        assert!(insights.dead_ends.is_empty());
    }

    #[test]
    fn test_skips_short_lines() {
        let output = b"short\nab\n\n";
        let insights = extract_insights(output);
        assert!(insights.dead_ends.is_empty());
        assert!(insights.decisions.is_empty());
    }

    #[test]
    fn test_didnt_work_skips_long_lines() {
        // Lines > 120 chars with "didn't work" should be skipped
        let long_line = format!("{} didn't work because reasons", "A".repeat(120));
        let insights = extract_insights(long_line.as_bytes());
        assert!(insights.dead_ends.is_empty());
    }

    #[test]
    fn test_deduplication() {
        let output = b"tried X but Y\ntried X but Z\n";
        let insights = extract_insights(output);
        // Should deduplicate by approach (dedup_by is only on adjacent items)
        assert_eq!(insights.dead_ends.len(), 1);
    }

    #[test]
    fn test_multiple_insights() {
        let output = b"tried plan A but too complex\nrejected plan B because too slow\ndecided to use plan C because simple and fast\n";
        let insights = extract_insights(output);
        assert_eq!(insights.dead_ends.len(), 2);
        assert_eq!(insights.decisions.len(), 1);
    }

    // --- Tests for new extraction patterns ---

    #[test]
    fn test_extract_doesnt_work() {
        let output = b"The regex approach doesn't work because it can't handle nested structures\n";
        let insights = extract_insights(output);
        assert_eq!(insights.dead_ends.len(), 1);
        assert_eq!(insights.dead_ends[0].approach, "The regex approach");
    }

    #[test]
    fn test_extract_wont_work() {
        let output = b"Global state won't work because of thread safety issues\n";
        let insights = extract_insights(output);
        assert_eq!(insights.dead_ends.len(), 1);
        assert_eq!(insights.dead_ends[0].approach, "Global state");
    }

    #[test]
    fn test_extract_considered_but() {
        let output = b"I considered using Redis but the added dependency isn't justified\n";
        let insights = extract_insights(output);
        assert_eq!(insights.dead_ends.len(), 1);
        assert!(insights.dead_ends[0].approach.contains("redis"));
    }

    #[test]
    fn test_extract_switched_from() {
        let output = b"switched from SQLite to PostgreSQL for concurrent writes\n";
        let insights = extract_insights(output);
        assert_eq!(insights.dead_ends.len(), 1);
        assert_eq!(insights.dead_ends[0].approach, "sqlite");
    }

    #[test]
    fn test_extract_abandoned() {
        let output = b"abandoned the ORM approach because raw SQL was simpler\n";
        let insights = extract_insights(output);
        assert_eq!(insights.dead_ends.len(), 1);
        assert!(insights.dead_ends[0].approach.contains("orm"));
    }

    #[test]
    fn test_extract_problem_with() {
        let output = b"the problem with polling is it wastes CPU cycles\n";
        let insights = extract_insights(output);
        assert_eq!(insights.dead_ends.len(), 1);
        assert_eq!(insights.dead_ends[0].approach, "polling");
    }

    #[test]
    fn test_extract_going_with_because() {
        let output = b"going with axum because it has better ergonomics than actix\n";
        let insights = extract_insights(output);
        assert_eq!(insights.decisions.len(), 1);
        assert_eq!(insights.decisions[0].description, "axum");
    }

    #[test]
    fn test_extract_using_because() {
        let output = b"using tokio because it has the best async runtime ecosystem\n";
        let insights = extract_insights(output);
        assert_eq!(insights.decisions.len(), 1);
        assert_eq!(insights.decisions[0].description, "tokio");
    }

    #[test]
    fn test_extract_ill_use_because() {
        let output = b"I'll use serde because it's the standard for Rust serialization\n";
        let insights = extract_insights(output);
        assert_eq!(insights.decisions.len(), 1);
        assert_eq!(insights.decisions[0].description, "serde");
    }

    #[test]
    fn test_skips_long_lines_for_new_patterns() {
        // Lines > 200 chars should be skipped by new patterns too
        let long_line = format!(
            "I considered {} but it was too complex for our needs",
            "A".repeat(180)
        );
        let insights = extract_insights(long_line.as_bytes());
        assert!(insights.dead_ends.is_empty());
    }
}
