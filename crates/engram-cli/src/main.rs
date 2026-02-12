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

fn main() -> Result<()> {
    let cli = Cli::parse();
    init_tracing(cli.verbose);

    match &cli.command {
        commands::Commands::Init(args) => commands::init::run(args),
        commands::Commands::Record(args) => commands::record::run(args),
        commands::Commands::Import(args) => commands::import::run(args),
        commands::Commands::Log(args) => commands::log::run(args, cli.format),
        commands::Commands::Show(args) => commands::show::run(args, cli.format),
        commands::Commands::Search(args) => commands::search::run(args, cli.format),
        commands::Commands::Trace(args) => commands::trace::run(args, cli.format),
        commands::Commands::Diff(args) => commands::diff::run(args, cli.format),
        commands::Commands::Graph(args) => commands::graph::run(args, cli.format),
        commands::Commands::Review(args) => commands::review::run(args, cli.format),
        commands::Commands::Push(args) => commands::push::run(args),
        commands::Commands::Pull(args) => commands::pull::run(args),
        commands::Commands::Fetch(args) => commands::fetch::run(args),
        commands::Commands::Stats => commands::stats::run(cli.format),
        commands::Commands::Gc(args) => commands::gc::run(args),
        commands::Commands::Blame(args) => commands::blame::run(args, cli.format),
        commands::Commands::Reindex => commands::reindex::run(),
        commands::Commands::Version => commands::version::run(),
        commands::Commands::HookHandler(args) => commands::hook_handler::run(args),
    }
}
