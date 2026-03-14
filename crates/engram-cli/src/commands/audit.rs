use std::collections::HashMap;
use std::io::Write;

use anyhow::{Context, Result};
use clap::Args;
use engram_core::storage::{GitStorage, ListOptions};

use crate::output::OutputFormat;

#[derive(Args)]
pub struct AuditArgs {
    /// Git range to audit (e.g., "main..HEAD"). If omitted, audits all commits on current branch.
    pub range: Option<String>,

    /// Write report to a file instead of stdout
    #[arg(short, long)]
    pub output: Option<String>,

    /// Report format: json, markdown, csv
    #[arg(long, default_value = "markdown")]
    pub report: ReportFormat,
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum ReportFormat {
    Json,
    Markdown,
    Csv,
}

struct AuditEntry {
    commit_sha: String,
    commit_date: String,
    commit_author: String,
    commit_message: String,
    engram_id: Option<String>,
    agent: Option<String>,
    model: Option<String>,
    tokens: Option<u64>,
    cost: Option<f64>,
    dead_ends: Vec<String>,
    decisions: Vec<String>,
}

pub fn run(args: &AuditArgs, _format: OutputFormat) -> Result<()> {
    let storage = GitStorage::discover().context("Not inside a Git repository")?;

    if !storage.is_initialized() {
        anyhow::bail!("Engram is not initialized. Run `engram init` first.");
    }

    let repo = storage.repo();

    // Build commit SHA -> engram ID map from all manifests
    let manifests = storage
        .list(&ListOptions::default())
        .context("Failed to list engrams")?;

    let mut commit_to_engram: HashMap<String, String> = HashMap::new();
    for m in &manifests {
        for sha in &m.git_commits {
            commit_to_engram
                .entry(sha.clone())
                .or_insert_with(|| m.id.as_str().to_string());
        }
    }

    // Also check for Engram-Id trailers in commit messages
    let mut trailer_to_engram: HashMap<String, String> = HashMap::new();

    // Walk commits
    let mut revwalk = repo.revwalk().context("Cannot create revwalk")?;

    if let Some(range) = &args.range {
        let parts: Vec<&str> = range.split("..").collect();
        if parts.len() != 2 {
            anyhow::bail!("Invalid range format. Expected 'base..head' (e.g., 'main..HEAD').");
        }
        let head_obj = repo
            .revparse_single(parts[1])
            .context(format!("Cannot resolve '{}'", parts[1]))?;
        let base_obj = repo
            .revparse_single(parts[0])
            .context(format!("Cannot resolve '{}'", parts[0]))?;
        revwalk
            .push(head_obj.id())
            .context("Cannot push head to revwalk")?;
        revwalk
            .hide(base_obj.id())
            .context("Cannot hide base in revwalk")?;
    } else {
        // Default: walk from HEAD
        revwalk.push_head().context("Cannot push HEAD to revwalk")?;
    }

    revwalk.set_sorting(git2::Sort::TOPOLOGICAL | git2::Sort::TIME)?;

    let mut entries = Vec::new();

    for oid_result in revwalk {
        let oid = oid_result.context("Revwalk error")?;
        let commit = repo.find_commit(oid).context("Cannot find commit")?;
        let sha = oid.to_string();

        let message = commit.message().unwrap_or("").to_string();
        let summary = commit.summary().unwrap_or("").to_string();
        let author = commit.author().name().unwrap_or("unknown").to_string();
        let time = commit.time();
        let date = chrono::DateTime::from_timestamp(time.seconds(), 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_default();

        // Find engram: first try direct commit linkage, then trailer
        let engram_id = commit_to_engram.get(&sha).cloned().or_else(|| {
            // Extract Engram-Id trailer from commit message
            for line in message.lines() {
                if let Some(id) = line.strip_prefix("Engram-Id: ") {
                    let id = id.trim().to_string();
                    trailer_to_engram
                        .entry(sha.clone())
                        .or_insert_with(|| id.clone());
                    return Some(id);
                }
            }
            None
        });

        let mut entry = AuditEntry {
            commit_sha: sha,
            commit_date: date,
            commit_author: author,
            commit_message: summary,
            engram_id: engram_id.clone(),
            agent: None,
            model: None,
            tokens: None,
            cost: None,
            dead_ends: Vec::new(),
            decisions: Vec::new(),
        };

        // Enrich with engram data if available
        if let Some(eid) = &engram_id {
            if let Ok(data) = storage.read(eid) {
                let m = &data.manifest;
                entry.agent = Some(m.agent.name.clone());
                entry.model = m.agent.model.clone();
                entry.tokens = Some(m.token_usage.total_tokens);
                entry.cost = Some(
                    m.token_usage
                        .effective_cost(m.agent.model.as_deref())
                        .unwrap_or(0.0),
                );
                entry.dead_ends = data
                    .intent
                    .dead_ends
                    .iter()
                    .map(|de| format!("{}: {}", de.approach, de.reason))
                    .collect();
                entry.decisions = data
                    .intent
                    .decisions
                    .iter()
                    .map(|d| format!("{}: {}", d.description, d.rationale))
                    .collect();
            }
        }

        entries.push(entry);
    }

    // Generate report
    let report = match args.report {
        ReportFormat::Json => render_json(&entries),
        ReportFormat::Markdown => render_markdown(&entries),
        ReportFormat::Csv => render_csv(&entries),
    };

    // Output
    if let Some(path) = &args.output {
        let mut file =
            std::fs::File::create(path).context(format!("Cannot create output file: {path}"))?;
        file.write_all(report.as_bytes())
            .context("Failed to write report")?;
        let traced = entries.iter().filter(|e| e.engram_id.is_some()).count();
        let total = entries.len();
        println!("Audit report written to {path} ({traced}/{total} commits traced)",);
    } else {
        print!("{report}");
    }

    Ok(())
}

fn render_json(entries: &[AuditEntry]) -> String {
    let traced = entries.iter().filter(|e| e.engram_id.is_some()).count();
    let untraced = entries.len() - traced;
    let total_tokens: u64 = entries.iter().filter_map(|e| e.tokens).sum();
    let total_cost: f64 = entries.iter().filter_map(|e| e.cost).sum();

    let json_entries: Vec<_> = entries
        .iter()
        .map(|e| {
            serde_json::json!({
                "commit": &e.commit_sha[..8.min(e.commit_sha.len())],
                "commit_full": e.commit_sha,
                "date": e.commit_date,
                "author": e.commit_author,
                "message": e.commit_message,
                "traced": e.engram_id.is_some(),
                "engram_id": e.engram_id.as_deref().map(|id| &id[..8.min(id.len())]),
                "agent": e.agent,
                "model": e.model,
                "tokens": e.tokens,
                "cost_usd": e.cost,
                "dead_ends": e.dead_ends,
                "decisions": e.decisions,
            })
        })
        .collect();

    let output = serde_json::json!({
        "audit_report": {
            "total_commits": entries.len(),
            "traced_commits": traced,
            "untraced_commits": untraced,
            "coverage_pct": if entries.is_empty() { 0.0 } else { (traced as f64 / entries.len() as f64) * 100.0 },
            "total_tokens": total_tokens,
            "total_cost_usd": total_cost,
        },
        "entries": json_entries,
    });

    serde_json::to_string_pretty(&output).unwrap_or_default()
}

fn render_markdown(entries: &[AuditEntry]) -> String {
    let traced = entries.iter().filter(|e| e.engram_id.is_some()).count();
    let total = entries.len();
    let total_tokens: u64 = entries.iter().filter_map(|e| e.tokens).sum();
    let total_cost: f64 = entries.iter().filter_map(|e| e.cost).sum();
    let coverage = if total > 0 {
        (traced as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    let mut out = String::new();

    // Header
    out.push_str("# Engram Audit Report\n\n");
    out.push_str(&format!(
        "| Metric | Value |\n|--------|-------|\n| Total commits | {} |\n| Traced commits | {} |\n| Untraced commits | {} |\n| Coverage | {:.1}% |\n| Total tokens | {} |\n| Total cost | ${:.2} |\n\n",
        total,
        traced,
        total - traced,
        coverage,
        total_tokens,
        total_cost
    ));

    // Table
    out.push_str("## Commit Details\n\n");
    out.push_str("| Commit | Date | Author | Message | Traced | Agent | Tokens | Cost |\n");
    out.push_str("|--------|------|--------|---------|--------|-------|--------|------|\n");

    for e in entries {
        let short_sha = &e.commit_sha[..8.min(e.commit_sha.len())];
        let traced_mark = if e.engram_id.is_some() {
            "yes"
        } else {
            "**NO**"
        };
        let agent = e.agent.as_deref().unwrap_or("-");
        let tokens = e
            .tokens
            .map(|t| t.to_string())
            .unwrap_or_else(|| "-".to_string());
        let cost = e
            .cost
            .map(|c| format!("${c:.2}"))
            .unwrap_or_else(|| "-".to_string());
        let msg = if e.commit_message.len() > 50 {
            let mut end = 47;
            while end > 0 && !e.commit_message.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}...", &e.commit_message[..end])
        } else {
            e.commit_message.clone()
        };

        out.push_str(&format!(
            "| {short_sha} | {} | {} | {msg} | {traced_mark} | {agent} | {tokens} | {cost} |\n",
            e.commit_date, e.commit_author
        ));
    }

    // Untraced section
    let untraced: Vec<_> = entries.iter().filter(|e| e.engram_id.is_none()).collect();
    if !untraced.is_empty() {
        out.push_str("\n## Untraced Commits\n\n");
        out.push_str("These commits have no associated AI reasoning chain:\n\n");
        for e in &untraced {
            let short_sha = &e.commit_sha[..8.min(e.commit_sha.len())];
            out.push_str(&format!(
                "- `{short_sha}` {} — {}\n",
                e.commit_date, e.commit_message
            ));
        }
    }

    out
}

fn render_csv(entries: &[AuditEntry]) -> String {
    let mut out = String::new();
    out.push_str("commit,date,author,message,traced,engram_id,agent,model,tokens,cost_usd\n");

    for e in entries {
        let short_sha = &e.commit_sha[..8.min(e.commit_sha.len())];
        let engram_short = e
            .engram_id
            .as_deref()
            .map(|id| &id[..8.min(id.len())])
            .unwrap_or("");
        let agent = e.agent.as_deref().unwrap_or("");
        let model = e.model.as_deref().unwrap_or("");
        let tokens = e.tokens.map(|t| t.to_string()).unwrap_or_default();
        let cost = e.cost.map(|c| format!("{c:.4}")).unwrap_or_default();
        // Escape all fields for CSV (quote fields that may contain commas/newlines)
        let msg = e.commit_message.replace('"', "\"\"");
        let author = e.commit_author.replace('"', "\"\"");

        out.push_str(&format!(
            "{short_sha},{},\"{author}\",\"{msg}\",{},{engram_short},{agent},{model},{tokens},{cost}\n",
            e.commit_date,
            e.engram_id.is_some(),
        ));
    }

    out
}
