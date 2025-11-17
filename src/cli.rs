use clap::{Parser, Subcommand};
use smol_str::SmolStr;

#[derive(Parser)]
#[command(name = "agentx")]
#[command(about = "Issue tracker CLI for structured task/bug tracking")]
pub struct Cli {
   #[arg(long, global = true, help = "Output in JSON format")]
   pub json: bool,

   #[arg(long, short = 'i', global = true, help = "Force interactive mode")]
   pub interactive: bool,

   #[command(subcommand)]
   pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
   /// List issues
   #[command(alias = "ls")]
   List {
      #[arg(long, default_value = "open")]
      status: SmolStr,

      #[arg(short, long)]
      verbose: bool,
   },

   /// Show full issue details
   Show { bug_ref: SmolStr },

   /// Create a new issue (use -i for interactive mode)
   #[command(alias = "add")]
   New {
      #[arg(long)]
      title: Option<SmolStr>,

      #[arg(long, default_value = "medium")]
      priority: SmolStr,

      #[arg(long = "tag")]
      tags: Vec<SmolStr>,

      #[arg(long = "file")]
      files: Vec<SmolStr>,

      #[arg(long)]
      issue: Option<SmolStr>,

      #[arg(long)]
      impact: Option<SmolStr>,

      #[arg(long)]
      acceptance: Option<SmolStr>,

      #[arg(long)]
      effort: Option<SmolStr>,

      #[arg(long)]
      context: Option<SmolStr>,
   },

   /// Mark issue as in-progress
   Start {
      bug_ref: SmolStr,

      #[arg(long, help = "Create git branch (overrides config)")]
      branch: bool,

      #[arg(long, help = "Skip git branch creation (overrides config)")]
      no_branch: bool,
   },

   /// Mark issue as blocked
   Block {
      bug_ref: SmolStr,

      #[arg(long)]
      reason: SmolStr,
   },

   /// Mark issue as closed
   Close {
      bug_ref: SmolStr,

      #[arg(short, long)]
      message: Option<SmolStr>,

      #[arg(long, help = "Create git commit (overrides config)")]
      commit: bool,

      #[arg(long, help = "Skip git commit (overrides config)")]
      no_commit: bool,
   },

   /// Reopen a closed issue
   Open { bug_ref: SmolStr },

   /// Move issue to backlog
   Defer { bug_ref: SmolStr },

   /// Activate issue from backlog
   Activate { bug_ref: SmolStr },

   /// Add checkpoint to issue
   Checkpoint { bug_ref: SmolStr, message: Vec<SmolStr> },

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
      file: Option<SmolStr>,
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
      threshold: SmolStr,
   },

   /// Start multiple issues at once
   BulkStart { bug_refs: Vec<SmolStr> },

   /// Close multiple issues at once
   BulkClose {
      bug_refs: Vec<SmolStr>,

      #[arg(short, long)]
      message: Option<SmolStr>,
   },

   /// Show session summary (what changed recently)
   Summary {
      #[arg(long, help = "Hours to look back (default: 24)")]
      hours: Option<u64>,
   },

   /// Show issue dependencies (what it depends on, what depends on it)
   Dependencies { bug_ref: SmolStr },

   /// Manage issue dependencies
   Depend {
      bug_ref: SmolStr,

      #[arg(long, value_delimiter = ',')]
      on: Vec<SmolStr>,

      #[arg(long, value_delimiter = ',')]
      remove: Vec<SmolStr>,
   },

   /// Manage issue tags
   Tag {
      bug_ref: SmolStr,

      #[arg(long, value_delimiter = ',')]
      add: Vec<SmolStr>,

      #[arg(long, value_delimiter = ',')]
      remove: Vec<SmolStr>,

      #[arg(long, short = 'l')]
      list: bool,
   },

   /// Find longest dependency chain (critical path)
   CriticalPath,

   /// Visualize dependency graph as ASCII art
   DepsGraph {
      #[arg(long, help = "Show only this issue and its dependencies")]
      issue: Option<SmolStr>,
   },

   /// Show performance metrics
   Metrics {
      #[arg(long, default_value = "week", help = "Time period: day, week, month, all")]
      period: SmolStr,
   },

   /// Generate shell completions
   Completions {
      #[arg(value_name = "SHELL")]
      shell: SmolStr,
   },

   /// Initialize config file
   Init {
      #[arg(long, help = "Create in home directory instead of current directory")]
      global: bool,
   },

   /// Start MCP server on stdio
   Serve,

   /// Launch interactive TUI dashboard
   #[command(alias = "dash")]
   Ui,

   /// Install MCP server configuration for supported clients
   Install {
      #[arg(long, help = "Uninstall MCP server configuration")]
      uninstall: bool,
   },
}

#[derive(Subcommand)]
pub enum AliasAction {
   /// List all aliases
   List,

   /// Add an alias
   Add { bug_ref: SmolStr, alias: SmolStr },

   /// Remove an alias
   Remove { alias: SmolStr },
}
