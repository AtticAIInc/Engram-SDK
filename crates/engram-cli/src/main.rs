use std::io::IsTerminal;
use std::sync::mpsc;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::{fmt, EnvFilter};

mod commands;
mod output;

#[derive(Parser)]
#[command(
    name = "engram",
    version,
    about = "Capture agent reasoning as Git-native versioned data"
)]
struct Cli {
    /// Increase verbosity (-v info, -vv debug, -vvv trace)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Output format
    #[arg(long, global = true, default_value = "text")]
    format: output::OutputFormat,

    #[command(subcommand)]
    command: commands::Commands,
}

fn init_tracing(verbose: u8) {
    let filter = match verbose {
        0 => EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
        1 => EnvFilter::new("info"),
        2 => EnvFilter::new("debug"),
        _ => EnvFilter::new("trace"),
    };
    fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .init();
}

/// Spawn a background thread for update checking.
/// Returns None for commands that should not trigger checks.
fn maybe_spawn_update_check(
    command: &commands::Commands,
) -> Option<mpsc::Receiver<Option<engram_core::update::UpdateInfo>>> {
    // Skip for internal/long-running commands
    if matches!(
        command,
        commands::Commands::HookHandler(_)
            | commands::Commands::Mcp
            | commands::Commands::Browse(_)
            | commands::Commands::Dashboard(_)
            | commands::Commands::Version
    ) {
        return None;
    }

    // Skip if disabled (cheap check before spawning thread)
    if engram_core::update::is_update_check_disabled() {
        return None;
    }

    let (tx, rx) = mpsc::channel();
    let current = env!("CARGO_PKG_VERSION").to_string();

    std::thread::spawn(move || {
        let result = engram_core::update::check_for_update(&current, false);
        let _ = tx.send(result);
    });

    Some(rx)
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    // Spawn background update check (non-blocking)
    let update_rx = maybe_spawn_update_check(&cli.command);

    let result = match &cli.command {
        commands::Commands::Init(args) => commands::init::run(args),
        commands::Commands::Config(args) => commands::config::run(args),
        commands::Commands::Record(args) => commands::record::run(args),
        commands::Commands::Import(args) => commands::import::run(args),
        commands::Commands::Log(args) => commands::log::run(args, cli.format),
        commands::Commands::Show(args) => commands::show::run(args, cli.format),
        commands::Commands::Search(args) => commands::search::run(args, cli.format),
        commands::Commands::Trace(args) => commands::trace::run(args, cli.format),
        commands::Commands::Diff(args) => commands::diff::run(args, cli.format),
        commands::Commands::Graph(args) => commands::graph::run(args, cli.format),
        commands::Commands::Review(args) => commands::review::run(args, cli.format),
        commands::Commands::Mcp => commands::mcp::run(),
        commands::Commands::PrSummary(args) => commands::pr_summary::run(args, cli.format),
        commands::Commands::Push(args) => commands::push::run(args),
        commands::Commands::Pull(args) => commands::pull::run(args),
        commands::Commands::Fetch(args) => commands::fetch::run(args),
        commands::Commands::Stats(args) => commands::stats::run(args, cli.format),
        commands::Commands::DeadEnds(args) => commands::dead_ends::run(args, cli.format),
        commands::Commands::Gc(args) => commands::gc::run(args),
        commands::Commands::Blame(args) => commands::blame::run(args, cli.format),
        commands::Commands::Reindex => commands::reindex::run(),
        commands::Commands::Why(args) => commands::why::run(args, cli.format),
        commands::Commands::Annotate(args) => commands::annotate::run(args),
        commands::Commands::Audit(args) => commands::audit::run(args, cli.format),
        commands::Commands::Browse(args) => commands::browse::run(args),
        commands::Commands::Dashboard(args) => commands::dashboard::run(args),
        commands::Commands::Doctor(args) => commands::doctor::run(args, cli.format),
        commands::Commands::Version => commands::version::run(),
        commands::Commands::HookHandler(args) => commands::hook_handler::run(args),
    };

    // After command completes, check if update notification is ready
    if let Some(rx) = update_rx {
        if let Ok(Some(info)) = rx.try_recv() {
            if std::io::stderr().is_terminal() {
                eprintln!("{}", engram_core::update::format_update_notice(&info));
            }
        }
    }

    result
}
