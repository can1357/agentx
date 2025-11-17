use crate::commands::Commands;
use crate::interactive::{validators, wizard};
use crate::storage::Storage;
use anyhow::Result;
use console::Style;
use std::path::PathBuf;

/// Interactive wizard for creating a new issue
pub fn new_issue_wizard(storage: &Storage, json: bool) -> Result<()> {
    wizard::section("🚀 Create New Issue");

    // Title
    let title = wizard::prompt_required("Title", validators::validate_non_empty)?;

    // Priority selection
    let priorities = vec!["Critical - Production outage", "High - Major feature blocked", "Medium - Standard priority", "Low - Nice to have"];
    let priority_idx = wizard::prompt_select("Priority", &priorities)?;
    let priority = match priority_idx {
        0 => "critical",
        1 => "high",
        2 => "medium",
        3 => "low",
        _ => "medium",
    };

    // Issue description (multi-line editor)
    wizard::info("Opening editor for issue description...");
    let issue = wizard::prompt_editor("📝 Issue Description", None)?
        .unwrap_or_else(|| "No description provided".to_string());

    // Impact description
    wizard::info("Opening editor for impact description...");
    let impact = wizard::prompt_editor("💥 Impact", None)?
        .unwrap_or_else(|| "No impact description provided".to_string());

    // Acceptance criteria
    wizard::info("Opening editor for acceptance criteria...");
    let acceptance = wizard::prompt_editor("✓ Acceptance Criteria", None)?
        .unwrap_or_else(|| "No acceptance criteria provided".to_string());

    // Effort estimation
    wizard::section("📊 Effort Estimation");
    let effort_options = vec![
        "XS - Very small (< 1h)",
        "S - Small (1-2h)",
        "M - Medium (2-4h)",
        "L - Large (1 day)",
        "XL - Very large (2+ days)",
        "Skip",
    ];
    let effort_idx = wizard::prompt_select("T-shirt size", &effort_options)?;
    let effort = if effort_idx < 5 {
        Some(match effort_idx {
            0 => "XS",
            1 => "S",
            2 => "M",
            3 => "L",
            4 => "XL",
            _ => "M",
        }.to_string())
    } else {
        None
    };

    // Related files (optional)
    wizard::section("📁 Related Files");
    let add_files = wizard::prompt_confirm("Add related files?", false)?;
    let files = if add_files {
        let mut selected_files = Vec::new();
        loop {
            let file = wizard::prompt_optional("File path (or empty to finish)", None)?;
            if file.trim().is_empty() {
                break;
            }
            if validators::validate_file_exists(&file).is_ok() {
                selected_files.push(file);
                wizard::success(&format!("Added: {}", selected_files.last().unwrap()));
            } else {
                wizard::error(&format!("File not found: {}", file));
            }
        }
        selected_files
    } else {
        Vec::new()
    };

    // Context (optional)
    let add_context = wizard::prompt_confirm("Add additional context?", false)?;
    let context = if add_context {
        wizard::prompt_editor("📌 Additional Context", None)?
    } else {
        None
    };

    // Preview
    wizard::section("✨ Preview");
    let preview = format!(
        "Title: {}\nPriority: {}\nEffort: {}\nFiles: {}\nDescription: {}",
        title,
        priority,
        effort.as_deref().unwrap_or("Not specified"),
        if files.is_empty() { "None".to_string() } else { files.join(", ") },
        if issue.len() > 100 { format!("{}...", &issue[..100]) } else { issue.clone() }
    );
    wizard::display_preview("New Issue", &preview);

    // Confirmation
    if !wizard::prompt_confirm("Create this issue?", true)? {
        wizard::info("Cancelled");
        return Ok(());
    }

    // Create the issue
    let commands = Commands::new(storage.clone());
    commands.create_issue(title, priority, files, issue, impact, acceptance, effort, context, json)?;

    wizard::success("Issue created successfully!");
    Ok(())
}

/// Interactive wizard for importing issues
pub fn import_wizard(storage: &Storage, json: bool) -> Result<()> {
    wizard::section("📥 Import Issues");

    let file = wizard::prompt_required("YAML file path", validators::validate_file_exists)?;

    // Preview file contents
    if wizard::prompt_confirm("Preview file before importing?", true)? {
        if let Ok(contents) = std::fs::read_to_string(&file) {
            let preview = if contents.len() > 500 {
                format!("{}...\n(truncated)", &contents[..500])
            } else {
                contents
            };
            wizard::display_preview("Import File", &preview);
        }
    }

    if !wizard::prompt_confirm("Import these issues?", true)? {
        wizard::info("Cancelled");
        return Ok(());
    }

    let commands = Commands::new(storage.clone());
    commands.import(Some(file), json)?;

    wizard::success("Issues imported successfully!");
    Ok(())
}

/// Interactive wizard for managing dependencies
pub fn depend_wizard(storage: &Storage, bug_ref: Option<String>, json: bool) -> Result<()> {
    wizard::section("🔗 Manage Dependencies");

    // Get bug reference
    let bug_ref = if let Some(ref_id) = bug_ref {
        ref_id
    } else {
        wizard::prompt_required("Issue reference", validators::validate_issue_ref)?
    };

    // Show current dependencies
    let commands = Commands::new(storage.clone());
    wizard::info("Current dependencies:");
    commands.dependencies(&bug_ref, false)?;

    // Action selection
    let actions = vec!["Add dependencies", "Remove dependencies", "Cancel"];
    let action_idx = wizard::prompt_select("What would you like to do?", &actions)?;

    match action_idx {
        0 => {
            // Add dependencies
            let mut deps = Vec::new();
            wizard::info("Enter issue references (empty to finish):");
            loop {
                let dep = wizard::prompt_optional("Depends on", None)?;
                if dep.trim().is_empty() {
                    break;
                }
                if validators::validate_issue_ref(&dep).is_ok() {
                    deps.push(dep.clone());
                    wizard::success(&format!("Will add dependency: {}", dep));
                } else {
                    wizard::error("Invalid issue reference");
                }
            }

            if !deps.is_empty() {
                commands.depend(&bug_ref, deps, Vec::new(), json)?;
                wizard::success("Dependencies added!");
            }
        }
        1 => {
            // Remove dependencies
            let mut to_remove = Vec::new();
            wizard::info("Enter issue references to remove (empty to finish):");
            loop {
                let dep = wizard::prompt_optional("Remove dependency", None)?;
                if dep.trim().is_empty() {
                    break;
                }
                to_remove.push(dep.clone());
                wizard::success(&format!("Will remove dependency: {}", dep));
            }

            if !to_remove.is_empty() {
                commands.depend(&bug_ref, Vec::new(), to_remove, json)?;
                wizard::success("Dependencies removed!");
            }
        }
        _ => {
            wizard::info("Cancelled");
        }
    }

    Ok(())
}

/// Interactive wizard for adding checkpoint
pub fn checkpoint_wizard(storage: &Storage, bug_ref: Option<String>, json: bool) -> Result<()> {
    wizard::section("📍 Add Checkpoint");

    // Get bug reference
    let bug_ref = if let Some(ref_id) = bug_ref {
        ref_id
    } else {
        wizard::prompt_required("Issue reference", validators::validate_issue_ref)?
    };

    // Message templates
    wizard::info("Quick templates (or write custom message):");
    let templates = vec![
        "Started investigation",
        "Found root cause",
        "Implemented fix",
        "Testing in progress",
        "Ready for review",
        "Custom message",
    ];

    let template_idx = wizard::prompt_select("Select template or custom", &templates)?;
    let message = if template_idx < 5 {
        templates[template_idx].to_string()
    } else {
        wizard::prompt_required("Checkpoint message", validators::validate_non_empty)?
    };

    // Preview
    wizard::display_preview("Checkpoint", &format!("{}: {}", bug_ref, message));

    if !wizard::prompt_confirm("Add this checkpoint?", true)? {
        wizard::info("Cancelled");
        return Ok(());
    }

    let commands = Commands::new(storage.clone());
    commands.checkpoint(&bug_ref, message, json)?;

    wizard::success("Checkpoint added!");
    Ok(())
}

/// Interactive wizard for init command
pub fn init_wizard() -> Result<()> {
    wizard::section("⚙️ Initialize Configuration");

    let location_options = vec!["Current directory (.agentxrc.yaml)", "Home directory (~/.agentxrc.yaml)"];
    let location_idx = wizard::prompt_select("Configuration location", &location_options)?;
    let global = location_idx == 1;

    // Issue directory
    let default_dir = if global {
        "~/.agentx/issues".to_string()
    } else {
        "./.agentx/issues".to_string()
    };

    let issues_dir = wizard::prompt_optional(
        &format!("Issues directory (default: {})", default_dir),
        Some(&default_dir),
    )?;

    let issues_dir = if issues_dir.trim().is_empty() {
        default_dir
    } else {
        issues_dir
    };

    // Git integration
    let git_enabled = wizard::prompt_confirm("Enable Git integration (branch creation, commits)?", true)?;

    // ID format
    let id_formats = vec!["BUG-### (default)", "TASK-###", "ISSUE-###", "Custom prefix"];
    let id_format_idx = wizard::prompt_select("Issue ID format", &id_formats)?;
    let id_prefix = match id_format_idx {
        0 => "BUG",
        1 => "TASK",
        2 => "ISSUE",
        3 => &wizard::prompt_required("Custom prefix", validators::validate_non_empty)?,
        _ => "BUG",
    };

    // Preview
    let preview = format!(
        "Location: {}\nIssues directory: {}\nGit integration: {}\nID format: {}-###",
        if global { "Global (~/.agentxrc.yaml)" } else { "Local (./.agentxrc.yaml)" },
        issues_dir,
        if git_enabled { "Enabled" } else { "Disabled" },
        id_prefix
    );
    wizard::display_preview("Configuration", &preview);

    if !wizard::prompt_confirm("Create this configuration?", true)? {
        wizard::info("Cancelled");
        return Ok(());
    }

    // Create config using the existing init logic
    use crate::config::{Config, IssuesLocation};
    use std::path::PathBuf;
    let mut config = Config::default();
    config.issues_location = Some(IssuesLocation::Fixed { path: PathBuf::from(issues_dir) });
    config.git_integration.enabled = git_enabled;
    config.issue_prefix = id_prefix.to_string();

    let config_path = if global {
        dirs::home_dir()
            .ok_or_else(|| anyhow::anyhow!("Could not determine home directory"))?
            .join(".agentxrc.yaml")
    } else {
        std::env::current_dir()?.join(".agentxrc.yaml")
    };

    if config_path.exists() {
        if !wizard::prompt_confirm("Config file already exists. Overwrite?", false)? {
            wizard::info("Cancelled");
            return Ok(());
        }
    }

    let yaml = serde_yaml::to_string(&config)?;
    std::fs::write(&config_path, yaml)?;

    wizard::success(&format!("Configuration created at: {}", config_path.display()));
    Ok(())
}
