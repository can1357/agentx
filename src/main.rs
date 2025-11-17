use agentx::{
   cli::{AliasAction, Cli, Command},
   commands::Commands,
   config::Config,
   guide,
   interactive::wizards,
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
   let storage = Storage::new(issues_dir.clone());
   let commands = Commands::new(storage);

   match cli.command {
      Command::List { status, verbose } => {
         commands.list(&status, verbose, cli.json)?;
      },
      Command::Show { bug_ref } => {
         commands.show(&bug_ref, cli.json)?;
      },
      Command::New { title, priority, tags, files, issue, impact, acceptance, effort, context } => {
         // Check if we should use interactive mode
         // Interactive mode triggers if: --interactive flag OR missing required fields
         let use_interactive = cli.interactive
            || title.is_none()
            || issue.is_none()
            || impact.is_none()
            || acceptance.is_none();

         if use_interactive && atty::is(atty::Stream::Stdin) {
            let wizard_storage = Storage::new(issues_dir.clone());
            wizards::new_issue_wizard(&wizard_storage, cli.json)?;
         } else {
            // All fields must be present for non-interactive mode
            let title = title.ok_or_else(|| {
               anyhow::anyhow!("--title is required (use -i for interactive mode)")
            })?;
            let issue = issue.ok_or_else(|| {
               anyhow::anyhow!("--issue is required (use -i for interactive mode)")
            })?;
            let impact = impact.ok_or_else(|| {
               anyhow::anyhow!("--impact is required (use -i for interactive mode)")
            })?;
            let acceptance = acceptance.ok_or_else(|| {
               anyhow::anyhow!("--acceptance is required (use -i for interactive mode)")
            })?;

            commands.create_issue(
               title.to_string(),
               &priority,
               tags.into_iter().map(|s| s.to_string()).collect(),
               files.into_iter().map(|s| s.to_string()).collect(),
               issue.to_string(),
               impact.to_string(),
               acceptance.to_string(),
               effort.map(|s| s.to_string()),
               context.map(|s| s.to_string()),
               cli.json,
            )?;
         }
      },
      Command::Start { bug_ref, branch, no_branch } => {
         commands.start(&bug_ref, branch, no_branch, cli.json)?;
      },
      Command::Block { bug_ref, reason } => {
         commands.block(&bug_ref, reason.to_string(), cli.json)?;
      },
      Command::Close { bug_ref, message, commit, no_commit } => {
         commands.close(&bug_ref, message.map(|s| s.to_string()), commit, no_commit, cli.json)?;
      },
      Command::Open { bug_ref } => {
         commands.open(&bug_ref, cli.json)?;
      },
      Command::Checkpoint { bug_ref, message } => {
         let use_interactive = cli.interactive || (bug_ref.is_empty() && message.is_empty());

         if use_interactive && atty::is(atty::Stream::Stdin) {
            let wizard_storage = Storage::new(issues_dir.clone());
            let bug_ref_opt = if bug_ref.is_empty() {
               None
            } else {
               Some(bug_ref.to_string())
            };
            wizards::checkpoint_wizard(&wizard_storage, bug_ref_opt, cli.json)?;
         } else {
            let note = message
               .iter()
               .map(|s| s.as_str())
               .collect::<Vec<_>>()
               .join(" ");
            commands.checkpoint(&bug_ref, note, cli.json)?;
         }
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
         let use_interactive = cli.interactive || file.is_none();

         if use_interactive && atty::is(atty::Stream::Stdin) {
            let wizard_storage = Storage::new(issues_dir.clone());
            wizards::import_wizard(&wizard_storage, cli.json)?;
         } else {
            commands.import(file.map(|s| s.to_string()), cli.json)?;
         }
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
         commands.bulk_start(bug_refs.into_iter().map(|s| s.to_string()).collect(), cli.json)?;
      },
      Command::BulkClose { bug_refs, message } => {
         commands.bulk_close(
            bug_refs.into_iter().map(|s| s.to_string()).collect(),
            message.map(|s| s.to_string()),
            cli.json,
         )?;
      },
      Command::Summary { hours } => {
         commands.summary(hours, cli.json)?;
      },
      Command::Dependencies { bug_ref } => {
         commands.dependencies(&bug_ref, cli.json)?;
      },
      Command::Depend { bug_ref, on, remove } => {
         let use_interactive =
            cli.interactive || (bug_ref.is_empty() && on.is_empty() && remove.is_empty());

         if use_interactive && atty::is(atty::Stream::Stdin) {
            let wizard_storage = Storage::new(issues_dir.clone());
            let bug_ref_opt = if bug_ref.is_empty() {
               None
            } else {
               Some(bug_ref.to_string())
            };
            wizards::depend_wizard(&wizard_storage, bug_ref_opt, cli.json)?;
         } else {
            commands.depend(
               &bug_ref,
               on.into_iter().map(|s| s.to_string()).collect(),
               remove.into_iter().map(|s| s.to_string()).collect(),
               cli.json,
            )?;
         }
      },
      Command::Tag { bug_ref, add, remove, list } => {
         commands.manage_tags(
            &bug_ref,
            add.into_iter().map(|s| s.to_string()).collect(),
            remove.into_iter().map(|s| s.to_string()).collect(),
            list,
            cli.json,
         )?;
      },
      Command::CriticalPath => {
         commands.critical_path(cli.json)?;
      },
      Command::DepsGraph { issue } => {
         commands.deps_graph(issue.as_deref(), cli.json)?;
      },
      Command::Metrics { period } => {
         commands.metrics(&period, cli.json)?;
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
         if cli.interactive && atty::is(atty::Stream::Stdin) {
            wizards::init_wizard()?;
         } else {
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
         }
      },
      Command::Serve => {
         IssueTrackerMCP::serve_stdio().await?;
      },
      Command::Defer { bug_ref } => {
         commands.defer(&bug_ref, cli.json)?;
      },
      Command::Activate { bug_ref } => {
         commands.activate(&bug_ref, cli.json)?;
      },
      Command::Ui => {
         let dashboard_storage = Storage::new(issues_dir);
         agentx::tui::launch_dashboard(dashboard_storage)?;
      },
   }

   Ok(())
}
