use agentx::{
   cli::{AliasAction, Cli, Command},
   commands::Commands,
   config::Config,
   guide,
   mcp::IssueTrackerMCP,
   storage::Storage,
};
use anyhow::Result;
use clap::{CommandFactory, Parser};
use clap_complete::{Shell, generate};

#[tokio::main]
async fn main() -> Result<()> {
   let cli = Cli::try_parse()?;
   let config = Config::load();
   let issues_dir = config.resolve_issues_directory();
   let storage = Storage::new(issues_dir);
   let commands = Commands::new(storage);

   match cli.command {
      Command::List { status, verbose } => {
         commands.list(&status, verbose, cli.json)?;
      },
      Command::Show { bug_ref } => {
         commands.show(&bug_ref, cli.json)?;
      },
      Command::New { title, priority, files, issue, impact, acceptance, effort, context } => {
         commands.create_issue(
            title, &priority, files, issue, impact, acceptance, effort, context, cli.json,
         )?;
      },
      Command::Start { bug_ref } => {
         commands.start(&bug_ref, cli.json)?;
      },
      Command::Block { bug_ref, reason } => {
         commands.block(&bug_ref, reason, cli.json)?;
      },
      Command::Close { bug_ref, message } => {
         commands.close(&bug_ref, message, cli.json)?;
      },
      Command::Open { bug_ref } => {
         commands.open(&bug_ref, cli.json)?;
      },
      Command::Checkpoint { bug_ref, message } => {
         let note = message.join(" ");
         commands.checkpoint(&bug_ref, note, cli.json)?;
      },
      Command::Context => {
         commands.context(cli.json)?;
      },
      Command::Focus => {
         commands.focus(cli.json)?;
      },
      Command::Blocked => {
         commands.blocked(cli.json)?;
      },
      Command::Ready => {
         commands.ready(cli.json)?;
      },
      Command::Import { file } => {
         commands.import(file, cli.json)?;
      },
      Command::Alias { action } => match action {
         AliasAction::List => {
            commands.alias_list(cli.json)?;
         },
         AliasAction::Add { bug_ref, alias } => {
            commands.alias_add(&bug_ref, &alias, cli.json)?;
         },
         AliasAction::Remove { alias } => {
            commands.alias_remove(&alias, cli.json)?;
         },
      },
      Command::Guide => {
         guide::print_guide();
      },
      Command::QuickWins { threshold } => {
         commands.quick_wins(&threshold, cli.json)?;
      },
      Command::BulkStart { bug_refs } => {
         commands.bulk_start(bug_refs, cli.json)?;
      },
      Command::BulkClose { bug_refs, message } => {
         commands.bulk_close(bug_refs, message, cli.json)?;
      },
      Command::Summary { hours } => {
         commands.summary(hours, cli.json)?;
      },
      Command::Dependencies { bug_ref } => {
         commands.dependencies(&bug_ref, cli.json)?;
      },
      Command::Depend { bug_ref, on, remove } => {
         commands.depend(&bug_ref, on, remove, cli.json)?;
      },
      Command::CriticalPath => {
         commands.critical_path(cli.json)?;
      },
      Command::Completions { shell } => {
         let shell_type = match shell.to_lowercase().as_str() {
            "bash" => Shell::Bash,
            "zsh" => Shell::Zsh,
            "fish" => Shell::Fish,
            "powershell" => Shell::PowerShell,
            "elvish" => Shell::Elvish,
            _ => {
               eprintln!("Unsupported shell: {shell}");
               eprintln!("Supported: bash, zsh, fish, powershell, elvish");
               std::process::exit(1);
            },
         };

         let mut cmd = Cli::command();
         generate(shell_type, &mut cmd, "agentx", &mut std::io::stdout());
      },
      Command::Init { global } => {
         let config = Config::default();
         let config_path = if global {
            dirs::home_dir()
               .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
               .join(".agentxrc.yaml")
         } else {
            std::env::current_dir()?.join(".agentxrc.yaml")
         };

         if config_path.exists() {
            eprintln!("Config file already exists at: {}", config_path.display());
            std::process::exit(1);
         }

         let yaml = serde_yaml::to_string(&config)?;
         std::fs::write(&config_path, yaml)?;
         println!("Created config file at: {}", config_path.display());
      },
      Command::Serve => {
         IssueTrackerMCP::serve_stdio().await?;
      },
   }

   Ok(())
}
