use engram_core::model::{DeadEnd, Decision};

/// Best-effort extraction of reasoning insights from raw PTY output.
///
/// This is heuristic: it looks for common phrases that indicate rejected
/// approaches or architectural decisions. Returns empty vecs if no patterns found.
pub fn extract_insights(raw_output: &[u8]) -> ExtractedInsights {
    let text = String::from_utf8_lossy(raw_output);
    let mut dead_ends = Vec::new();
    let mut decisions = Vec::new();

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.len() < 10 {
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

    // Deduplicate by approach/description
    dead_ends.dedup_by(|a, b| a.approach == b.approach);
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

fn try_extract_dead_end(lower: &str, original: &str) -> Option<DeadEnd> {
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

    // Pattern: "X didn't work because Y"
    if let Some(pos) = lower.find(" didn't work") {
        let approach = &original[..pos];
        let reason = lower
            .get((pos + " didn't work".len())..)
            .and_then(|r| r.strip_prefix(" because ").or(r.strip_prefix(": ")))
            .unwrap_or("did not work as expected");
        if !approach.is_empty() && approach.len() < 80 {
            return Some(DeadEnd {
                approach: approach.trim().to_string(),
                reason: reason.trim().to_string(),
            });
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

    None
}

fn try_extract_decision(lower: &str, original: &str) -> Option<Decision> {
    // Pattern: "decided to X because Y"
    if let Some(rest) = lower.strip_prefix("decided to ") {
        if let Some((desc, rationale)) = rest.split_once(" because ") {
            return Some(Decision {
                description: desc.trim().to_string(),
                rationale: rationale.trim().to_string(),
            });
        }
    }

    // Pattern: "chose X over Y" (short lines only)
    if lower.starts_with("chose ") && original.len() < 120 {
        if let Some((desc, _)) = lower.strip_prefix("chose ").and_then(|r| r.split_once(" over "))
        {
            return Some(Decision {
                description: desc.trim().to_string(),
                rationale: original.to_string(),
            });
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
}
