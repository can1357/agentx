use anyhow::Result;
use clap::Parser;
use issues::cli::{AliasAction, Cli, Command};
use issues::commands::Commands;
use issues::guide;
use issues::mcp::IssueTrackerMCP;
use issues::storage::Storage;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let current_dir = env::current_dir()?;
    let storage = Storage::new(current_dir);
    let commands = Commands::new(storage);

    match cli.command {
        Command::List { status, verbose } => {
            commands.list(&status, verbose, cli.json)?;
        }
        Command::Show { bug_ref } => {
            commands.show(&bug_ref, cli.json)?;
        }
        Command::New {
            title,
            priority,
            files,
            issue,
            impact,
            acceptance,
            effort,
            context,
        } => {
            commands.create_issue(
                title, &priority, files, issue, impact, acceptance, effort, context, cli.json,
            )?;
        }
        Command::Start { bug_ref } => {
            commands.start(&bug_ref, cli.json)?;
        }
        Command::Block { bug_ref, reason } => {
            commands.block(&bug_ref, reason, cli.json)?;
        }
        Command::Close { bug_ref, message } => {
            commands.close(&bug_ref, message, cli.json)?;
        }
        Command::Open { bug_ref } => {
            commands.open(&bug_ref, cli.json)?;
        }
        Command::Checkpoint { bug_ref, message } => {
            let note = message.join(" ");
            commands.checkpoint(&bug_ref, note, cli.json)?;
        }
        Command::Context => {
            commands.context(cli.json)?;
        }
        Command::Focus => {
            commands.focus(cli.json)?;
        }
        Command::Blocked => {
            commands.blocked(cli.json)?;
        }
        Command::Ready => {
            commands.ready(cli.json)?;
        }
        Command::Import { file } => {
            commands.import(file, cli.json)?;
        }
        Command::Alias { action } => match action {
            AliasAction::List => {
                commands.alias_list(cli.json)?;
            }
            AliasAction::Add { bug_ref, alias } => {
                commands.alias_add(&bug_ref, &alias, cli.json)?;
            }
            AliasAction::Remove { alias } => {
                commands.alias_remove(&alias, cli.json)?;
            }
        },
        Command::Guide => {
            guide::print_guide();
        }
        Command::QuickWins { threshold } => {
            commands.quick_wins(&threshold, cli.json)?;
        }
        Command::BulkStart { bug_refs } => {
            commands.bulk_start(bug_refs, cli.json)?;
        }
        Command::BulkClose { bug_refs, message } => {
            commands.bulk_close(bug_refs, message, cli.json)?;
        }
        Command::Summary { hours } => {
            commands.summary(hours, cli.json)?;
        }
        Command::Serve => {
            IssueTrackerMCP::serve_stdio().await?;
        }
    }

    Ok(())
}
