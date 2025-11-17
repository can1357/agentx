use crate::issue::{Issue, Priority, Status};
use crate::storage::Storage;
use anyhow::{Context, Result};
use chrono::Utc;
use serde_json::json;
use std::collections::HashMap;

pub struct Commands {
    storage: Storage,
}

impl Commands {
    pub fn new(storage: Storage) -> Self {
        Self { storage }
    }

    pub fn list(&self, status: &str, verbose: bool, json: bool) -> Result<()> {
        let issues = match status {
            "open" => self.storage.list_open_issues()?,
            "closed" => self.storage.list_closed_issues()?,
            _ => anyhow::bail!("Invalid status: {status}"),
        };

        if json {
            let data: Vec<_> = issues
                .iter()
                .map(|issue| {
                    json!({
                        "num": issue.metadata.id,
                        "title": issue.metadata.title,
                        "priority": issue.metadata.priority.to_string(),
                        "status": issue.metadata.status.to_string(),
                        "files": issue.metadata.files,
                        "effort": issue.metadata.effort,
                        "blocked_reason": issue.metadata.blocked_reason,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&data)?);
            return Ok(());
        }

        if issues.is_empty() {
            println!("No {status} issues found");
            return Ok(());
        }

        println!("\n{}", "=".repeat(80));
        println!("{} ISSUES ({} total)", status.to_uppercase(), issues.len());
        println!("{}\n", "=".repeat(80));

        let mut by_priority: HashMap<Priority, Vec<&Issue>> = HashMap::new();
        for issue in &issues {
            by_priority
                .entry(issue.metadata.priority)
                .or_default()
                .push(issue);
        }

        for priority in [
            Priority::Critical,
            Priority::High,
            Priority::Medium,
            Priority::Low,
        ] {
            let bugs = by_priority.get(&priority);
            if bugs.is_none() || bugs.unwrap().is_empty() {
                continue;
            }

            let bugs = bugs.unwrap();
            println!("{} ({})", priority.to_string().to_uppercase(), bugs.len());
            println!("{}", "-".repeat(80));

            for issue in bugs {
                let marker = issue.metadata.status.marker();
                println!(
                    "  {} BUG-{}: {}",
                    marker, issue.metadata.id, issue.metadata.title
                );

                if issue.metadata.status == Status::Blocked {
                    if let Some(reason) = &issue.metadata.blocked_reason {
                        println!("       Blocked: {reason}");
                    }
                }

                if verbose && !issue.metadata.files.is_empty() {
                    for file in &issue.metadata.files {
                        println!("       → {file}");
                    }
                }
            }
            println!();
        }

        Ok(())
    }

    pub fn show(&self, bug_ref: &str, json: bool) -> Result<()> {
        let bug_num = self.storage.resolve_bug_ref(bug_ref)?;
        let issue = self.storage.load_issue(bug_num)?;

        if json {
            let output = json!({
                "metadata": issue.metadata,
                "body": issue.body,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            print!("{}", issue.to_mdx());
        }

        Ok(())
    }

    pub fn create_issue(
        &self,
        title: String,
        priority_str: &str,
        files: Vec<String>,
        issue: String,
        impact: String,
        acceptance: String,
        effort: Option<String>,
        context: Option<String>,
        json: bool,
    ) -> Result<()> {
        let priority = match priority_str {
            "critical" => Priority::Critical,
            "high" => Priority::High,
            "medium" => Priority::Medium,
            "low" => Priority::Low,
            _ => anyhow::bail!("Invalid priority: {priority_str}"),
        };

        let bug_num = self.storage.next_bug_number()?;
        let issue_obj = Issue::new(
            bug_num, title, priority, files, issue, impact, acceptance, effort, context,
        );

        let path = self.storage.save_issue(&issue_obj, true)?;

        if json {
            let output = json!({
                "bug_num": bug_num,
                "path": path.display().to_string(),
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("✓ Created BUG-{bug_num} → {}", path.display());
        }

        Ok(())
    }

    pub fn start(&self, bug_ref: &str, json: bool) -> Result<()> {
        let bug_num = self.storage.resolve_bug_ref(bug_ref)?;

        self.storage.update_issue_metadata(bug_num, |meta| {
            meta.status = Status::InProgress;
            meta.started = Some(Utc::now());
        })?;

        if json {
            let output = json!({
                "bug_num": bug_num,
                "status": "in_progress",
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("🔄 BUG-{bug_num} marked as IN PROGRESS");
        }

        Ok(())
    }

    pub fn block(&self, bug_ref: &str, reason: String, json: bool) -> Result<()> {
        let bug_num = self.storage.resolve_bug_ref(bug_ref)?;

        self.storage.update_issue_metadata(bug_num, |meta| {
            meta.status = Status::Blocked;
            meta.blocked_reason = Some(reason.clone());
        })?;

        if json {
            let output = json!({
                "bug_num": bug_num,
                "status": "blocked",
                "reason": reason,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("🚫 BUG-{bug_num} marked as BLOCKED: {reason}");
        }

        Ok(())
    }

    pub fn close(&self, bug_ref: &str, message: Option<String>, json: bool) -> Result<()> {
        let bug_num = self.storage.resolve_bug_ref(bug_ref)?;

        // Update metadata
        self.storage.update_issue_metadata(bug_num, |meta| {
            meta.status = Status::Closed;
            meta.closed = Some(Utc::now());
        })?;

        // Add close note if provided
        if let Some(note) = message {
            let mut issue = self.storage.load_issue(bug_num)?;
            let timestamp = Utc::now().format("%Y-%m-%d").to_string();
            issue.body.push_str(&format!("\n\n---\n\n**Closed** ({timestamp}): {note}\n"));
            self.storage.save_issue(&issue, true)?;
        }

        // Move to closed directory
        self.storage.move_issue(bug_num, false)?;

        if json {
            let output = json!({
                "bug_num": bug_num,
                "status": "closed",
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("✓ BUG-{bug_num} marked as CLOSED");
        }

        Ok(())
    }

    pub fn open(&self, bug_ref: &str, json: bool) -> Result<()> {
        let bug_num = self.storage.resolve_bug_ref(bug_ref)?;

        // Update metadata
        self.storage.update_issue_metadata(bug_num, |meta| {
            meta.status = Status::NotStarted;
            meta.closed = None;
        })?;

        // Move to open directory
        self.storage.move_issue(bug_num, true)?;

        if json {
            let output = json!({
                "bug_num": bug_num,
                "status": "not_started",
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("↻ BUG-{bug_num} marked as OPEN");
        }

        Ok(())
    }

    pub fn checkpoint(&self, bug_ref: &str, note: String, json: bool) -> Result<()> {
        let bug_num = self.storage.resolve_bug_ref(bug_ref)?;
        let mut issue = self.storage.load_issue(bug_num)?;

        let timestamp = Utc::now().format("%Y-%m-%d %H:%M").to_string();
        let checkpoint = format!("\n\n**Checkpoint** ({timestamp}): {note}");

        issue.body.push_str(&checkpoint);

        // Determine if open or closed
        let is_open = issue.metadata.status != Status::Closed;
        self.storage.save_issue(&issue, is_open)?;

        if json {
            let output = json!({
                "bug_num": bug_num,
                "checkpoint": note,
                "timestamp": timestamp,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("✓ Added checkpoint to BUG-{bug_num}");
        }

        Ok(())
    }

    pub fn context(&self, json: bool) -> Result<()> {
        let issues = self.storage.list_open_issues()?;

        if issues.is_empty() {
            if json {
                println!("{}", json!({"summary": "No open issues"}));
            } else {
                println!("No open issues");
            }
            return Ok(());
        }

        let mut in_progress = Vec::new();
        let mut blocked = Vec::new();
        let mut high_priority = Vec::new();
        let mut ready = Vec::new();

        for issue in &issues {
            let item = json!({
                "num": issue.metadata.id,
                "title": issue.metadata.title,
                "priority": issue.metadata.priority.to_string(),
                "status": issue.metadata.status.to_string(),
            });

            match issue.metadata.status {
                Status::InProgress => in_progress.push(item),
                Status::Blocked => {
                    let mut item = item;
                    if let Some(obj) = item.as_object_mut() {
                        obj.insert(
                            "blocked_reason".to_string(),
                            json!(issue.metadata.blocked_reason),
                        );
                    }
                    blocked.push(item);
                }
                Status::NotStarted => {
                    if matches!(
                        issue.metadata.priority,
                        Priority::Critical | Priority::High
                    ) {
                        high_priority.push(item.clone());
                    }
                    ready.push(item);
                }
                _ => {}
            }
        }

        if json {
            let output = json!({
                "in_progress": in_progress,
                "blocked": blocked,
                "high_priority": high_priority,
                "ready_to_start": ready.iter().take(5).collect::<Vec<_>>(),
                "total_open": issues.len(),
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
            return Ok(());
        }

        println!("\n{}", "=".repeat(80));
        println!("CURRENT CONTEXT");
        println!("{}\n", "=".repeat(80));

        if !in_progress.is_empty() {
            println!("🔄 IN PROGRESS ({}):", in_progress.len());
            for item in &in_progress {
                println!(
                    "   BUG-{}: {}",
                    item["num"], item["title"].as_str().unwrap()
                );
            }
            println!();
        }

        if !blocked.is_empty() {
            println!("🚫 BLOCKED ({}):", blocked.len());
            for item in &blocked {
                println!(
                    "   BUG-{}: {}",
                    item["num"], item["title"].as_str().unwrap()
                );
                if let Some(reason) = item.get("blocked_reason") {
                    if !reason.is_null() {
                        println!("      → {}", reason.as_str().unwrap());
                    }
                }
            }
            println!();
        }

        if !high_priority.is_empty() {
            println!("⚠️  HIGH PRIORITY QUEUE ({}):", high_priority.len());
            for item in &high_priority {
                println!(
                    "   [{}] BUG-{}: {}",
                    item["priority"].as_str().unwrap().to_uppercase(),
                    item["num"],
                    item["title"].as_str().unwrap()
                );
            }
            println!();
        }

        if !ready.is_empty() {
            println!("✓ READY TO START ({} tasks):", ready.len());
            for item in ready.iter().take(5) {
                println!(
                    "   BUG-{}: {}",
                    item["num"], item["title"].as_str().unwrap()
                );
            }
            if ready.len() > 5 {
                println!("   ... and {} more", ready.len() - 5);
            }
            println!();
        }

        println!("Total open issues: {}", issues.len());

        Ok(())
    }

    pub fn focus(&self, json: bool) -> Result<()> {
        let issues = self.storage.list_open_issues()?;

        let mut focus_issues: Vec<_> = issues
            .iter()
            .map(|issue| {
                let sort_key = match issue.metadata.status {
                    Status::InProgress | Status::Blocked => -1,
                    _ => issue.metadata.priority.sort_key() as i32,
                };

                (issue, sort_key)
            })
            .collect();

        focus_issues.sort_by_key(|(_, key)| *key);
        let focus_issues: Vec<_> = focus_issues.iter().take(5).map(|(issue, _)| issue).collect();

        if json {
            let data: Vec<_> = focus_issues
                .iter()
                .map(|issue| {
                    json!({
                        "num": issue.metadata.id,
                        "title": issue.metadata.title,
                        "priority": issue.metadata.priority.to_string(),
                        "status": issue.metadata.status.to_string(),
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&data)?);
            return Ok(());
        }

        println!("\n{}", "=".repeat(80));
        println!("FOCUS - Top Priority Tasks");
        println!("{}\n", "=".repeat(80));

        for issue in focus_issues {
            let marker = issue.metadata.status.marker();
            let priority_label = format!("[{}]", issue.metadata.priority.to_string().to_uppercase());
            println!(
                "{} {:10} BUG-{}: {}",
                marker, priority_label, issue.metadata.id, issue.metadata.title
            );
        }

        Ok(())
    }

    pub fn blocked(&self, json: bool) -> Result<()> {
        let issues = self.storage.list_open_issues()?;

        let blocked_issues: Vec<_> = issues
            .iter()
            .filter(|issue| issue.metadata.status == Status::Blocked)
            .collect();

        if json {
            let data: Vec<_> = blocked_issues
                .iter()
                .map(|issue| {
                    json!({
                        "num": issue.metadata.id,
                        "title": issue.metadata.title,
                        "reason": issue.metadata.blocked_reason,
                        "priority": issue.metadata.priority.to_string(),
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&data)?);
            return Ok(());
        }

        if blocked_issues.is_empty() {
            println!("No blocked tasks");
            return Ok(());
        }

        println!("\n{}", "=".repeat(80));
        println!("BLOCKED TASKS ({})", blocked_issues.len());
        println!("{}\n", "=".repeat(80));

        for issue in blocked_issues {
            println!("🚫 BUG-{}: {}", issue.metadata.id, issue.metadata.title);
            if let Some(reason) = &issue.metadata.blocked_reason {
                println!("   Reason: {reason}");
            }
            println!(
                "   Priority: {}\n",
                issue.metadata.priority.to_string().to_uppercase()
            );
        }

        Ok(())
    }

    pub fn ready(&self, json: bool) -> Result<()> {
        let issues = self.storage.list_open_issues()?;

        let mut ready_issues: Vec<_> = issues
            .iter()
            .filter(|issue| issue.metadata.status == Status::NotStarted)
            .collect();

        ready_issues.sort_by_key(|issue| issue.metadata.priority.sort_key());

        if json {
            let data: Vec<_> = ready_issues
                .iter()
                .map(|issue| {
                    json!({
                        "num": issue.metadata.id,
                        "title": issue.metadata.title,
                        "priority": issue.metadata.priority.to_string(),
                        "files": issue.metadata.files,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&data)?);
            return Ok(());
        }

        if ready_issues.is_empty() {
            println!("No tasks ready to start");
            return Ok(());
        }

        println!("\n{}", "=".repeat(80));
        println!("READY TO START ({} tasks)", ready_issues.len());
        println!("{}\n", "=".repeat(80));

        for issue in ready_issues {
            let priority_label = format!("[{}]", issue.metadata.priority.to_string().to_uppercase());
            println!(
                "⭕ {:10} BUG-{}: {}",
                priority_label, issue.metadata.id, issue.metadata.title
            );
            if !issue.metadata.files.is_empty() {
                println!("   Files: {}", issue.metadata.files.join(", "));
            }
        }

        Ok(())
    }

    pub fn import(&self, file: Option<String>, json: bool) -> Result<()> {
        let yaml_input = if let Some(path) = file {
            std::fs::read_to_string(path)?
        } else {
            use std::io::Read;
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer)?;
            buffer
        };

        let data: Vec<serde_yaml::Value> = serde_yaml::from_str(&yaml_input)
            .context("Failed to parse YAML input")?;

        let mut created = Vec::new();

        for item in data {
            let obj = item.as_mapping().context("Item must be a mapping")?;

            let title = obj
                .get("title")
                .and_then(|v| v.as_str())
                .context("Missing title")?
                .to_string();

            let priority_str = obj
                .get("priority")
                .and_then(|v| v.as_str())
                .unwrap_or("medium");

            let files: Vec<String> = obj
                .get("files")
                .and_then(|v| v.as_sequence())
                .map(|seq| {
                    seq.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();

            let issue = obj
                .get("issue")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let impact = obj
                .get("impact")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let acceptance = obj
                .get("acceptance")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let effort = obj
                .get("effort")
                .and_then(|v| v.as_str())
                .map(String::from);

            let context = obj
                .get("context")
                .and_then(|v| v.as_str())
                .map(String::from);

            self.create_issue(
                title,
                priority_str,
                files,
                issue,
                impact,
                acceptance,
                effort,
                context,
                false,
            )?;

            let bug_num = self.storage.next_bug_number()? - 1;
            created.push(bug_num);
        }

        if json {
            let output = json!({
                "created": created,
                "count": created.len(),
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("\n✓ Created {} issues", created.len());
        }

        Ok(())
    }

    pub fn alias_list(&self, json: bool) -> Result<()> {
        let aliases = self.storage.load_aliases()?;

        if json {
            println!("{}", serde_json::to_string_pretty(&aliases)?);
            return Ok(());
        }

        if aliases.is_empty() {
            println!("No aliases defined");
            return Ok(());
        }

        println!("\nAliases:");
        let mut items: Vec<_> = aliases.iter().collect();
        items.sort_by_key(|(k, _)| *k);

        for (alias, bug_num) in items {
            println!("  {alias} → BUG-{bug_num}");
        }

        Ok(())
    }

    pub fn alias_add(&self, bug_ref: &str, alias: &str, json: bool) -> Result<()> {
        let bug_num = self.storage.resolve_bug_ref(bug_ref)?;

        // Verify bug exists
        self.storage.find_issue_file(bug_num)?;

        let mut aliases = self.storage.load_aliases()?;
        aliases.insert(alias.to_string(), bug_num);
        self.storage.save_aliases(&aliases)?;

        if json {
            let output = json!({
                "alias": alias,
                "bug_num": bug_num,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("✓ Created alias: {alias} → BUG-{bug_num}");
        }

        Ok(())
    }

    pub fn alias_remove(&self, alias: &str, json: bool) -> Result<()> {
        let mut aliases = self.storage.load_aliases()?;

        let bug_num = aliases
            .remove(alias)
            .ok_or_else(|| anyhow::anyhow!("Alias '{alias}' not found"))?;

        self.storage.save_aliases(&aliases)?;

        if json {
            let output = json!({
                "removed": alias,
                "was": bug_num,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        } else {
            println!("✓ Removed alias: {alias}");
        }

        Ok(())
    }
}
