use anyhow::{Context, Result};
use clap::Args;

use engram_capture::pty::{PtySession, PtyWrapperConfig};
use engram_capture::session::SessionBuilder;
use engram_core::hooks::ActiveSession;
use engram_core::model::{AgentInfo, EngramId};
use engram_core::storage::GitStorage;
use engram_query::search::SearchEngine;

#[derive(Args)]
pub struct RecordArgs {
    /// Agent name (auto-detected from command if not specified)
    #[arg(long)]
    pub agent: Option<String>,

    /// Model name (e.g. claude-sonnet-4-5, gpt-4o)
    #[arg(long)]
    pub model: Option<String>,

    /// Command and arguments to run (after --)
    #[arg(trailing_var_arg = true, required = true)]
    pub command: Vec<String>,
}

pub fn run(args: &RecordArgs) -> Result<()> {
    let storage = GitStorage::discover().context("Not inside a Git repository")?;

    if !storage.is_initialized() {
        anyhow::bail!("Engram is not initialized. Run `engram init` first.");
    }

    if args.command.is_empty() {
        anyhow::bail!("No command specified. Usage: engram record -- <command> [args...]");
    }

    let cmd = &args.command[0];
    let cmd_args = &args.command[1..];
    let agent_name = args.agent.clone().unwrap_or_else(|| detect_agent_name(cmd));

    let working_dir = std::env::current_dir().context("Failed to get current directory")?;

    eprintln!(
        "Recording session: {} {} (agent: {})",
        cmd,
        cmd_args.join(" "),
        agent_name
    );

    let git_dir = storage.repo().path().to_path_buf();

    // Create active session so hooks can inject trailers during recording
    let agent_info_for_session = AgentInfo {
        name: agent_name.clone(),
        model: args.model.clone(),
        version: None,
    };
    let active_session = ActiveSession::new(EngramId::new(), agent_info_for_session);
    active_session
        .save(&git_dir)
        .context("Failed to create active session")?;

    let config = PtyWrapperConfig {
        command: cmd.clone(),
        args: cmd_args.to_vec(),
        working_dir,
        agent_name: Some(agent_name.clone()),
    };

    let session = PtySession::start(config).context("Failed to start PTY session")?;
    let captured = session.run().context("PTY session failed")?;

    // Load accumulated commits from active session before cleanup
    let commits = ActiveSession::load(&git_dir)
        .map(|s| s.commits.clone())
        .unwrap_or_default();

    // Clean up active session
    ActiveSession::cleanup(&git_dir);

    let exit_code = captured.exit_code;
    let file_count = captured.file_changes.len();
    let duration = captured.end_time - captured.start_time;

    let agent_info = AgentInfo {
        name: agent_name,
        model: args.model.clone(),
        version: None,
    };

    let data = SessionBuilder::new(agent_info, captured)
        .with_commits(commits)
        .build();
    let id = storage.create(&data).context("Failed to store engram")?;

    // Best-effort incremental index update
    if let Ok(search) = SearchEngine::open(&storage) {
        let _ = search.index_engram(&data);
    }

    eprintln!();
    eprintln!("Engram {} captured:", &id.as_str()[..8]);
    eprintln!(
        "  Exit code: {}",
        exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "unknown".into())
    );
    eprintln!(
        "  Duration:  {:.1}s",
        duration.num_milliseconds() as f64 / 1000.0
    );
    eprintln!("  Files changed: {file_count}");
    eprintln!();
    eprintln!("View with: engram show {}", &id.as_str()[..8]);

    Ok(())
}

fn detect_agent_name(cmd: &str) -> String {
    let basename = std::path::Path::new(cmd)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| cmd.to_string());

    match basename.as_str() {
        "claude" | "claude-code" => "claude-code".into(),
        "aider" => "aider".into(),
        "cursor" | "cursor-cli" => "cursor".into(),
        "copilot" => "copilot".into(),
        _ => basename,
    }
}
