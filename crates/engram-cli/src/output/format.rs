use engram_core::model::{EngramData, Manifest};

use super::OutputFormat;

pub fn format_manifest_list(manifests: &[Manifest], show_cost: bool, fmt: OutputFormat) -> String {
    match fmt {
        OutputFormat::Json => serde_json::to_string_pretty(manifests).unwrap_or_default(),
        OutputFormat::Text => format_manifest_list_text(manifests, show_cost),
    }
}

fn format_manifest_list_text(manifests: &[Manifest], show_cost: bool) -> String {
    if manifests.is_empty() {
        return "No engrams found.".to_string();
    }

    let mut out = String::new();
    for m in manifests {
        let short_id = &m.id.as_str()[..8.min(m.id.as_str().len())];
        let summary = m.summary.as_deref().unwrap_or("(no summary)");
        let agent = &m.agent.name;
        let model = m.agent.model.as_deref().unwrap_or("");
        let time = m.created_at.format("%Y-%m-%d %H:%M");

        if show_cost {
            let tokens = m.token_usage.total_tokens;
            let cost = m
                .token_usage
                .cost_usd
                .map(|c| format!("${c:.2}"))
                .unwrap_or_else(|| "-".to_string());
            out.push_str(&format!(
                "\u{25c6} {short_id} {summary} [{agent}/{model}] {cost} {tokens}tok  {time}\n"
            ));
        } else {
            out.push_str(&format!(
                "\u{25c6} {short_id} {summary} [{agent}/{model}]  {time}\n"
            ));
        }
    }
    out
}

pub fn format_engram_full(data: &EngramData, fmt: OutputFormat) -> String {
    match fmt {
        OutputFormat::Json => serde_json::to_string_pretty(&data.manifest).unwrap_or_default(),
        OutputFormat::Text => format_engram_full_text(data),
    }
}

fn format_engram_full_text(data: &EngramData) -> String {
    let m = &data.manifest;
    let mut out = String::new();

    out.push_str(&format!("Engram: {}\n", m.id));
    out.push_str(&format!(
        "Agent:  {}{}\n",
        m.agent.name,
        m.agent
            .model
            .as_ref()
            .map(|m| format!(" ({m})"))
            .unwrap_or_default()
    ));
    out.push_str(&format!(
        "Date:   {}\n",
        m.created_at.format("%Y-%m-%d %H:%M:%S UTC")
    ));
    if let Some(summary) = &m.summary {
        out.push_str(&format!("Summary: {summary}\n"));
    }

    // Token usage
    let tu = &m.token_usage;
    if tu.total_tokens > 0 {
        out.push_str(&format!(
            "Tokens: {} total ({} in, {} out)",
            tu.total_tokens, tu.input_tokens, tu.output_tokens
        ));
        if let Some(cost) = tu.cost_usd {
            out.push_str(&format!("  Cost: ${cost:.4}"));
        }
        out.push('\n');
    }

    if !m.git_commits.is_empty() {
        out.push_str(&format!("Commits: {}\n", m.git_commits.join(", ")));
    }

    if !m.tags.is_empty() {
        out.push_str(&format!("Tags: {}\n", m.tags.join(", ")));
    }

    // Intent
    out.push_str("\n--- Intent ---\n");
    out.push_str(&data.intent.to_markdown());

    // Operations summary
    if !data.operations.file_changes.is_empty() {
        out.push_str("\n--- File Changes ---\n");
        for fc in &data.operations.file_changes {
            let symbol = match &fc.change_type {
                engram_core::model::FileChangeType::Created => "+",
                engram_core::model::FileChangeType::Modified => "~",
                engram_core::model::FileChangeType::Deleted => "-",
                engram_core::model::FileChangeType::Renamed { from } => {
                    out.push_str(&format!("  {from} -> {}\n", fc.path));
                    continue;
                }
            };
            out.push_str(&format!("  {symbol} {}\n", fc.path));
        }
    }

    if !data.operations.tool_calls.is_empty() {
        out.push_str(&format!(
            "\n--- Tool Calls ({}) ---\n",
            data.operations.tool_calls.len()
        ));
        for tc in &data.operations.tool_calls {
            let err_marker = if tc.is_error { " [ERROR]" } else { "" };
            out.push_str(&format!("  {}{err_marker}\n", tc.tool_name));
        }
    }

    // Transcript summary
    out.push_str(&format!(
        "\n--- Transcript ({} entries) ---\n",
        data.transcript.entries.len()
    ));

    out
}

pub fn format_intent(data: &EngramData, fmt: OutputFormat) -> String {
    match fmt {
        OutputFormat::Json => serde_json::to_string_pretty(&data.intent).unwrap_or_default(),
        OutputFormat::Text => data.intent.to_markdown(),
    }
}
