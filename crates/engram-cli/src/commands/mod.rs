pub mod blame;
pub mod diff;
pub mod fetch;
pub mod gc;
pub mod graph;
pub mod hook_handler;
pub mod import;
pub mod init;
pub mod log;
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
    Stats,
    /// Garbage collect old engrams
    Gc(gc::GcArgs),
    /// Show reasoning blame for a file
    Blame(blame::BlameArgs),
    /// Rebuild the search index
    Reindex,
    /// Print version information
    Version,
    /// Internal: handle git hook callbacks
    #[command(hide = true)]
    HookHandler(hook_handler::HookHandlerArgs),
}
