use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "agentx")]
#[command(about = "Issue tracker CLI for structured task/bug tracking")]
pub struct Cli {
    #[arg(long, global = true, help = "Output in JSON format")]
    pub json: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// List issues
    List {
        #[arg(long, default_value = "open")]
        status: String,

        #[arg(short, long)]
        verbose: bool,
    },

    /// Show full issue details
    Show { bug_ref: String },

    /// Create a new issue
    New {
        #[arg(long)]
        title: String,

        #[arg(long, default_value = "medium")]
        priority: String,

        #[arg(long = "file")]
        files: Vec<String>,

        #[arg(long)]
        issue: String,

        #[arg(long)]
        impact: String,

        #[arg(long)]
        acceptance: String,

        #[arg(long)]
        effort: Option<String>,

        #[arg(long)]
        context: Option<String>,
    },

    /// Mark issue as in-progress
    Start {
        bug_ref: String,

        #[arg(long, help = "Create git branch (overrides config)")]
        branch: bool,

        #[arg(long, help = "Skip git branch creation (overrides config)")]
        no_branch: bool,
    },

    /// Mark issue as blocked
    Block {
        bug_ref: String,

        #[arg(long)]
        reason: String,
    },

    /// Mark issue as closed
    Close {
        bug_ref: String,

        #[arg(short, long)]
        message: Option<String>,

        #[arg(long, help = "Create git commit (overrides config)")]
        commit: bool,

        #[arg(long, help = "Skip git commit (overrides config)")]
        no_commit: bool,
    },

    /// Reopen a closed issue
    Open { bug_ref: String },

    /// Move issue to backlog
    Defer { bug_ref: String },

    /// Activate issue from backlog
    Activate { bug_ref: String },

    /// Add checkpoint to issue
    Checkpoint {
        bug_ref: String,
        message: Vec<String>,
    },

    /// Show current work context
    Context,

    /// Show top priority tasks
    Focus,

    /// Show blocked tasks
    Blocked,

    /// Show tasks ready to start
    Ready,

    /// Import multiple issues from YAML
    Import {
        #[arg(long)]
        file: Option<String>,
    },

    /// Manage bug aliases
    Alias {
        #[command(subcommand)]
        action: AliasAction,
    },

    /// Show agent usage guide
    Guide,

    /// Show quick wins (low-effort tasks)
    QuickWins {
        #[arg(long, default_value = "1h")]
        threshold: String,
    },

    /// Start multiple issues at once
    BulkStart { bug_refs: Vec<String> },

    /// Close multiple issues at once
    BulkClose {
        bug_refs: Vec<String>,

        #[arg(short, long)]
        message: Option<String>,
    },

    /// Show session summary (what changed recently)
    Summary {
        #[arg(long, help = "Hours to look back (default: 24)")]
        hours: Option<u64>,
    },

    /// Show issue dependencies (what it depends on, what depends on it)
    Dependencies { bug_ref: String },

    /// Manage issue dependencies
    Depend {
        bug_ref: String,

        #[arg(long, value_delimiter = ',')]
        on: Vec<String>,

        #[arg(long, value_delimiter = ',')]
        remove: Vec<String>,
    },

    /// Find longest dependency chain (critical path)
    CriticalPath,

    /// Visualize dependency graph as ASCII art
    DepsGraph {
        #[arg(long, help = "Show only this issue and its dependencies")]
        issue: Option<String>,
    },

    /// Show performance metrics
    Metrics {
        #[arg(long, default_value = "week", help = "Time period: day, week, month, all")]
        period: String,
    },

    /// Generate shell completions
    Completions {
        #[arg(value_name = "SHELL")]
        shell: String,
    },

    /// Initialize config file
    Init {
        #[arg(long, help = "Create in home directory instead of current directory")]
        global: bool,
    },

    /// Start MCP server on stdio
    Serve,
}

#[derive(Subcommand)]
pub enum AliasAction {
    /// List all aliases
    List,

    /// Add an alias
    Add { bug_ref: String, alias: String },

    /// Remove an alias
    Remove { alias: String },
}
