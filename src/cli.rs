use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "issues")]
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
    Start { bug_ref: String },

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
    },

    /// Reopen a closed issue
    Open { bug_ref: String },

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
