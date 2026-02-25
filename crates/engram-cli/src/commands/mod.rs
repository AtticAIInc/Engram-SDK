pub mod annotate;
pub mod audit;
pub mod blame;
pub mod browse;
pub mod dashboard;
pub mod dead_ends;
pub mod diff;
pub mod fetch;
pub mod gc;
pub mod graph;
pub mod hook_handler;
pub mod import;
pub mod init;
pub mod log;
pub mod mcp;
pub mod pr_summary;
pub mod pull;
pub mod push;
pub mod record;
pub mod reindex;
pub mod review;
pub mod search;
pub mod show;
pub mod stats;
pub mod trace;
pub mod version;
pub mod why;

use clap::Subcommand;

#[derive(Subcommand)]
pub enum Commands {
    /// Initialize engram in the current Git repository
    Init(init::InitArgs),
    /// Record an agent session (wraps any command in a PTY)
    Record(record::RecordArgs),
    /// Import sessions from known agent formats
    Import(import::ImportArgs),
    /// List engrams (most recent first)
    Log(log::LogArgs),
    /// Show details of a specific engram
    Show(show::ShowArgs),
    /// Search engrams by content
    Search(search::SearchArgs),
    /// Trace reasoning history for a file
    Trace(trace::TraceArgs),
    /// Compare two engrams
    Diff(diff::DiffArgs),
    /// Show the context graph
    Graph(graph::GraphArgs),
    /// Review intent chain for a branch range
    Review(review::ReviewArgs),
    /// Push engram refs to a remote
    Push(push::PushArgs),
    /// Pull engram refs from a remote and reindex
    Pull(pull::PullArgs),
    /// Fetch engram refs from a remote (no reindex)
    Fetch(fetch::FetchArgs),
    /// Show aggregate statistics across all engrams
    Stats(stats::StatsArgs),
    /// Start MCP server (stdio transport) for AI agent integration
    Mcp,
    /// Generate a PR description from the engram chain
    PrSummary(pr_summary::PrSummaryArgs),
    /// Garbage collect old engrams
    Gc(gc::GcArgs),
    /// Surface dead ends and recurring rejected approaches
    DeadEnds(dead_ends::DeadEndsArgs),
    /// Show reasoning blame for a file
    Blame(blame::BlameArgs),
    /// Rebuild the search index
    Reindex,
    /// Explain why a file exists through its reasoning chain
    Why(why::WhyArgs),
    /// Attach engram reasoning as git notes on commits
    Annotate(annotate::AnnotateArgs),
    /// Generate audit trail report for compliance
    Audit(audit::AuditArgs),
    /// Interactive terminal UI for browsing engrams
    Browse(browse::BrowseArgs),
    /// Start the web dashboard for visualizing engram data
    Dashboard(dashboard::DashboardArgs),
    /// Print version information
    Version,
    /// Internal: handle git hook callbacks
    #[command(hide = true)]
    HookHandler(hook_handler::HookHandlerArgs),
}
