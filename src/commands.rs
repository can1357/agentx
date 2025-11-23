use std::collections::HashMap;

use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use serde_json::json;
use smol_str::SmolStr;

use crate::{
   config::Config,
   git::GitOps,
   issue::{Issue, IssueWithId, Priority, Status},
   storage::Storage,
   utils::parse_effort,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueListResult {
   pub status: String,
   pub count:  usize,
   pub issues: Vec<IssueWithId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextResult {
   pub active:         Vec<IssueWithId>,
   pub blocked:        Vec<IssueWithId>,
   pub high_priority:  Vec<IssueWithId>,
   pub ready_to_start: Vec<IssueWithId>,
   pub total_open:     usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShowResult {
   pub num:            u32,
   pub title:          String,
   pub priority:       String,
   pub status:         String,
   pub body:           String,
   pub tags:           Vec<String>,
   pub files:          Vec<String>,
   pub effort:         Option<String>,
   pub created:        DateTime<Utc>,
   pub started:        Option<DateTime<Utc>>,
   pub closed:         Option<DateTime<Utc>>,
   pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateIssueResult {
   pub bug_num: u32,
   pub title:   String,
   pub path:    String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusUpdateResult {
   pub bug_num: u32,
   pub status:  String,
   pub message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Commands {
   storage: Storage,
   config:  Config,
}

impl Commands {
   pub fn new(storage: Storage) -> Self {
      Self { storage, config: Config::load() }
   }

   pub fn config(&self) -> &Config {
      &self.config
   }

   pub fn list_data(&self, status: &str) -> Result<IssueListResult> {
      let issues = match status {
         "open" => self.storage.list_open_issues()?,
         "closed" => self.storage.list_closed_issues()?,
         _ => anyhow::bail!("Invalid status: {status}"),
      };

      Ok(IssueListResult {
         status: status.to_string(),
         count:  issues.len(),
         issues,
      })
   }

   pub fn list(&self, status: &str, verbose: bool, json: bool) -> Result<()> {
      let result = self.list_data(status)?;

      if json {
         let data: Vec<_> = result
            .issues
            .iter()
            .map(|issue_with_id| {
               json!({
                   "num": issue_with_id.id,
                   "title": issue_with_id.issue.metadata.title,
                   "priority": issue_with_id.issue.metadata.priority.to_string(),
                   "status": issue_with_id.issue.metadata.status.to_string(),
                   "files": issue_with_id.issue.metadata.files,
                   "effort": issue_with_id.issue.metadata.effort,
                   "blocked_reason": issue_with_id.issue.metadata.blocked_reason,
                   "tags": issue_with_id.issue.metadata.tags,
               })
            })
            .collect();
         println!("{}", serde_json::to_string_pretty(&data)?);
         return Ok(());
      }

      if result.issues.is_empty() {
         println!("No {} issues found", result.status);
         return Ok(());
      }

      let use_colors = self.config.colored_output;

      // Separate backlog from active issues
      let (active_issues, backlog_issues): (Vec<_>, Vec<_>) = result
         .issues
         .iter()
         .partition(|issue_with_id| issue_with_id.issue.metadata.status != Status::Backlog);

      println!("\n{}", "=".repeat(80));
      println!(
         "{} ISSUES ({} active, {} backlog)",
         status.to_uppercase(),
         active_issues.len(),
         backlog_issues.len()
      );
      println!("{}\n", "=".repeat(80));

      // Display active issues by priority
      let mut by_priority: HashMap<Priority, Vec<&IssueWithId>> = HashMap::new();
      for issue_with_id in active_issues {
         by_priority
            .entry(issue_with_id.issue.metadata.priority)
            .or_default()
            .push(issue_with_id);
      }

      for priority in [Priority::Critical, Priority::High, Priority::Medium, Priority::Low] {
         let bugs = by_priority.get(&priority);
         if bugs.is_none() || bugs.unwrap().is_empty() {
            continue;
         }

         let bugs = bugs.unwrap();
         let header = format!("{} ({})", priority.to_string().to_uppercase(), bugs.len());
         if use_colors {
            let colored_header = match priority {
               Priority::Critical => header.red().bold(),
               Priority::High => header.yellow().bold(),
               Priority::Medium => header.normal(),
               Priority::Low => header.bright_black(),
            };
            println!("{}", colored_header);
         } else {
            println!("{}", header);
         }
         println!("{}", "-".repeat(80));

         for issue_with_id in bugs {
            let marker = issue_with_id.issue.metadata.status.marker();
            let tags_str = if !issue_with_id.issue.metadata.tags.is_empty() {
               format!(
                  " {}",
                  issue_with_id
                     .issue
                     .metadata
                     .tags
                     .iter()
                     .map(|t| format!("#{}", t))
                     .collect::<Vec<_>>()
                     .join(" ")
               )
            } else {
               String::new()
            };
            let line = format!(
               "  {} {}: {}{}",
               marker,
               self.config.format_issue_ref(issue_with_id.id),
               issue_with_id.issue.metadata.title,
               tags_str
            );

            if use_colors {
               let colored_line = match priority {
                  Priority::Critical => line.red(),
                  Priority::High => line.yellow(),
                  Priority::Medium => line.normal(),
                  Priority::Low => line.bright_black(),
               };
               println!("{}", colored_line);
            } else {
               println!("{}", line);
            }

            if issue_with_id.issue.metadata.status == Status::Blocked
               && let Some(reason) = &issue_with_id.issue.metadata.blocked_reason
            {
               let blocked_line = format!("       Blocked: {reason}");
               if use_colors {
                  println!("{}", blocked_line.bright_red());
               } else {
                  println!("{}", blocked_line);
               }
            }

            if verbose && !issue_with_id.issue.metadata.files.is_empty() {
               for file in &issue_with_id.issue.metadata.files {
                  println!("       → {file}");
               }
            }
         }
         println!();
      }

      // Display backlog at the end (dimmed)
      if !backlog_issues.is_empty() {
         let header = format!("BACKLOG ({})", backlog_issues.len());
         if use_colors {
            println!("{}", header.dimmed().bold());
         } else {
            println!("{}", header);
         }
         println!("{}", "-".repeat(80));

         let mut backlog_by_priority: HashMap<Priority, Vec<&IssueWithId>> = HashMap::new();
         for issue_with_id in backlog_issues {
            backlog_by_priority
               .entry(issue_with_id.issue.metadata.priority)
               .or_default()
               .push(issue_with_id);
         }

         for priority in [Priority::Critical, Priority::High, Priority::Medium, Priority::Low] {
            if let Some(bugs) = backlog_by_priority.get(&priority) {
               for issue_with_id in bugs {
                  let marker = issue_with_id.issue.metadata.status.marker();
                  let tags_str = if !issue_with_id.issue.metadata.tags.is_empty() {
                     format!(
                        " {}",
                        issue_with_id
                           .issue
                           .metadata
                           .tags
                           .iter()
                           .map(|t| format!("#{}", t))
                           .collect::<Vec<_>>()
                           .join(" ")
                     )
                  } else {
                     String::new()
                  };
                  let line = format!(
                     "  {} {}: {}{}",
                     marker,
                     self.config.format_issue_ref(issue_with_id.id),
                     issue_with_id.issue.metadata.title,
                     tags_str
                  );

                  if use_colors {
                     println!("{}", line.dimmed());
                  } else {
                     println!("{}", line);
                  }

                  if verbose && !issue_with_id.issue.metadata.files.is_empty() {
                     for file in &issue_with_id.issue.metadata.files {
                        let file_line = format!("       → {file}");
                        if use_colors {
                           println!("{}", file_line.dimmed());
                        } else {
                           println!("{}", file_line);
                        }
                     }
                  }
               }
            }
         }
         println!();
      }

      Ok(())
   }

   pub fn show_data(&self, bug_ref: &str) -> Result<ShowResult> {
      let bug_num = self.storage.resolve_bug_ref(bug_ref)?;
      let issue = self.storage.load_issue(bug_num)?;

      Ok(ShowResult {
         num:            bug_num,
         title:          issue.metadata.title.to_string(),
         priority:       issue.metadata.priority.to_string(),
         status:         issue.metadata.status.to_string(),
         body:           issue.body.clone(),
         tags:           issue.metadata.tags.iter().map(|s| s.to_string()).collect(),
         files:          issue.metadata.files.iter().map(|s| s.to_string()).collect(),
         effort:         issue.metadata.effort.as_ref().map(|s| s.to_string()),
         created:        issue.metadata.created,
         started:        issue.metadata.started,
         closed:         issue.metadata.closed,
         blocked_reason: issue.metadata.blocked_reason.as_ref().map(|s| s.to_string()),
      })
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

   #[allow(clippy::too_many_arguments)]
   pub fn create_issue_data(
      &self,
      title: String,
      priority_str: &str,
      tags: Vec<String>,
      files: Vec<String>,
      issue: String,
      impact: String,
      acceptance: String,
      effort: Option<String>,
      context: Option<String>,
   ) -> Result<CreateIssueResult> {
      let priority = match priority_str {
         "critical" => Priority::Critical,
         "high" => Priority::High,
         "medium" => Priority::Medium,
         "low" => Priority::Low,
         _ => anyhow::bail!("Invalid priority: {priority_str}"),
      };

      let bug_num = self.storage.next_bug_number()?;
      let issue_obj =
         Issue::new(title.clone(), priority, tags, files, issue, impact, acceptance, effort, context);

      let path = self.storage.save_issue(&issue_obj, bug_num, true)?;

      Ok(CreateIssueResult {
         bug_num,
         title,
         path: path.display().to_string(),
      })
   }

   #[allow(clippy::too_many_arguments)]
   pub fn create_issue(
      &self,
      title: String,
      priority_str: &str,
      tags: Vec<String>,
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

      // Check for similar issues
      let existing_issues = self.storage.list_open_issues()?;
      let mut similar = Vec::new();

      for existing in &existing_issues {
         let similarity = strsim::jaro_winkler(
            &title.to_lowercase(),
            &existing.issue.metadata.title.to_lowercase(),
         );
         if similarity > 0.8 {
            similar.push((existing.id, &existing.issue.metadata.title, similarity));
         }
      }

      // Sort by similarity descending
      similar.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap());

      if !similar.is_empty() && !json {
         eprintln!("\n⚠️  Similar issues found:");
         for (id, sim_title, score) in similar.iter().take(3) {
            eprintln!("   #{}: {} ({:.0}% similar)", id, sim_title, score * 100.0);
         }
         eprintln!();
      }

      let bug_num = self.storage.next_bug_number()?;
      let issue_obj =
         Issue::new(title, priority, tags, files, issue, impact, acceptance, effort, context);

      let path = self.storage.save_issue(&issue_obj, bug_num, true)?;

      if json {
         let output = json!({
             "bug_num": bug_num,
             "path": path.display().to_string(),
             "similar_issues": similar.iter().take(3).map(|(id, title, score)| {
                 json!({
                     "id": id,
                     "title": title,
                     "similarity": score,
                 })
             }).collect::<Vec<_>>(),
         });
         println!("{}", serde_json::to_string_pretty(&output)?);
      } else {
         println!("✓ Created {} → {}", self.config.format_issue_ref(bug_num), path.display());
      }

      Ok(())
   }

   pub fn start_data(&self, bug_ref: &str) -> Result<StatusUpdateResult> {
      let bug_num = self.storage.resolve_bug_ref(bug_ref)?;

      self.storage.update_issue_metadata(bug_num, |meta| {
         meta.status = Status::InProgress;
         meta.started = Some(Utc::now());
      })?;

      Ok(StatusUpdateResult {
         bug_num,
         status:  "in_progress".to_string(),
         message: None,
      })
   }

   pub fn start(
      &self,
      bug_ref: &str,
      branch_flag: bool,
      no_branch_flag: bool,
      json: bool,
   ) -> Result<()> {
      let bug_num = self.storage.resolve_bug_ref(bug_ref)?;
      let issue = self.storage.load_issue(bug_num)?;

      self.storage.update_issue_metadata(bug_num, |meta| {
         meta.status = Status::InProgress;
         meta.started = Some(Utc::now());
      })?;

      // Determine if we should create a branch
      let should_create_branch = if no_branch_flag {
         false
      } else if branch_flag {
         true
      } else {
         self.config.git_integration.enabled && self.config.git_integration.auto_branch
      };

      let mut branch_created = None;

      if should_create_branch {
         match GitOps::open(".") {
            Ok(git) => {
               let branch_name = format!(
                  "{}{}",
                  self.config.git_integration.branch_prefix,
                  Storage::slugify(&issue.metadata.title)
               );

               match git.create_branch(&branch_name) {
                  Ok(_) => {
                     branch_created = Some(branch_name);
                  },
                  Err(e) => {
                     if !json {
                        eprintln!("⚠️  Failed to create git branch: {}", e);
                     }
                  },
               }
            },
            Err(e) => {
               if !json {
                  eprintln!("⚠️  Not a git repository: {}", e);
               }
            },
         }
      }

      if json {
         let output = json!({
             "bug_num": bug_num,
             "status": "active",
             "branch_created": branch_created,
         });
         println!("{}", serde_json::to_string_pretty(&output)?);
      } else {
         println!("🔄 {} marked as IN PROGRESS", self.config.format_issue_ref(bug_num));
         if let Some(branch) = branch_created {
            println!("🌿 Created git branch: {}", branch);
         }
      }

      Ok(())
   }

   pub fn block_data(&self, bug_ref: &str, reason: String) -> Result<StatusUpdateResult> {
      let bug_num = self.storage.resolve_bug_ref(bug_ref)?;

      self.storage.update_issue_metadata(bug_num, |meta| {
         meta.status = Status::Blocked;
         meta.blocked_reason = Some(reason.clone().into());
      })?;

      Ok(StatusUpdateResult {
         bug_num,
         status:  "blocked".to_string(),
         message: Some(reason),
      })
   }

   pub fn block(&self, bug_ref: &str, reason: String, json: bool) -> Result<()> {
      let bug_num = self.storage.resolve_bug_ref(bug_ref)?;

      self.storage.update_issue_metadata(bug_num, |meta| {
         meta.status = Status::Blocked;
         meta.blocked_reason = Some(reason.clone().into());
      })?;

      if json {
         let output = json!({
             "bug_num": bug_num,
             "status": "blocked",
             "reason": reason,
         });
         println!("{}", serde_json::to_string_pretty(&output)?);
      } else {
         println!("🚫 {} marked as BLOCKED: {reason}", self.config.format_issue_ref(bug_num));
      }

      Ok(())
   }

   pub fn close_data(&self, bug_ref: &str, message: Option<String>) -> Result<StatusUpdateResult> {
      let bug_num = self.storage.resolve_bug_ref(bug_ref)?;

      self.storage.update_issue_metadata(bug_num, |meta| {
         meta.status = Status::Closed;
         meta.closed = Some(Utc::now());
      })?;

      if let Some(note) = &message {
         let mut issue = self.storage.load_issue(bug_num)?;
         issue.body.push_str(&format!("\n\n## Closed\n\n{}", note));
         self.storage.save_issue(&issue, bug_num, false)?;
      }

      Ok(StatusUpdateResult {
         bug_num,
         status: "closed".to_string(),
         message,
      })
   }

   pub fn open_data(&self, bug_ref: &str) -> Result<StatusUpdateResult> {
      let bug_num = self.storage.resolve_bug_ref(bug_ref)?;

      self.storage.update_issue_metadata(bug_num, |meta| {
         meta.status = Status::NotStarted;
         meta.closed = None;
      })?;

      self.storage.move_issue(bug_num, true)?;

      Ok(StatusUpdateResult {
         bug_num,
         status:  "open".to_string(),
         message: None,
      })
   }

   pub fn defer_data(&self, bug_ref: &str) -> Result<StatusUpdateResult> {
      let bug_num = self.storage.resolve_bug_ref(bug_ref)?;

      self.storage.update_issue_metadata(bug_num, |meta| {
         meta.status = Status::Backlog;
      })?;

      Ok(StatusUpdateResult {
         bug_num,
         status:  "backlog".to_string(),
         message: None,
      })
   }

   pub fn activate_data(&self, bug_ref: &str) -> Result<StatusUpdateResult> {
      let bug_num = self.storage.resolve_bug_ref(bug_ref)?;

      self.storage.update_issue_metadata(bug_num, |meta| {
         meta.status = Status::NotStarted;
      })?;

      Ok(StatusUpdateResult {
         bug_num,
         status:  "open".to_string(),
         message: None,
      })
   }

   pub fn checkpoint_data(&self, bug_ref: &str, note: String) -> Result<StatusUpdateResult> {
      let bug_num = self.storage.resolve_bug_ref(bug_ref)?;
      let mut issue = self.storage.load_issue(bug_num)?;

      let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
      issue
         .body
         .push_str(&format!("\n\n## Checkpoint - {}\n\n{}", timestamp, note));

      let mut status_changed = false;
      if note.starts_with("BLOCKED:") {
         let reason = note.strip_prefix("BLOCKED:").unwrap_or("").trim().to_string();
         self.storage.update_issue_metadata(bug_num, |meta| {
            meta.status = Status::Blocked;
            meta.blocked_reason = Some(reason.into());
         })?;
         status_changed = true;
      } else if note.starts_with("DONE:") || note.starts_with("COMPLETED:") {
         self.storage.update_issue_metadata(bug_num, |meta| {
            meta.status = Status::Closed;
            meta.closed = Some(Utc::now());
         })?;
         status_changed = true;
      }

      self.storage.save_issue(&issue, bug_num, false)?;

      Ok(StatusUpdateResult {
         bug_num,
         status:  if status_changed { "updated".to_string() } else { "checkpoint_added".to_string() },
         message: Some(note),
      })
   }

   pub fn close(
      &self,
      bug_ref: &str,
      message: Option<String>,
      commit_flag: bool,
      no_commit_flag: bool,
      json: bool,
   ) -> Result<()> {
      let bug_num = self.storage.resolve_bug_ref(bug_ref)?;

      // Update metadata
      self.storage.update_issue_metadata(bug_num, |meta| {
         meta.status = Status::Closed;
         meta.closed = Some(Utc::now());
      })?;

      // Add close note if provided
      if let Some(note) = &message {
         let mut issue = self.storage.load_issue(bug_num)?;
         let timestamp = Utc::now().format("%Y-%m-%d").to_string();
         issue
            .body
            .push_str(&format!("\n\n---\n\n**Closed** ({timestamp}): {note}\n"));
         self.storage.save_issue(&issue, bug_num, true)?;
      }

      // Move to closed directory
      self.storage.move_issue(bug_num, false)?;

      // Determine if we should create a commit
      let should_commit = if no_commit_flag {
         false
      } else if commit_flag {
         true
      } else {
         self.config.git_integration.enabled
            && self.config.git_integration.commit_prefix_format.is_some()
      };

      let mut commit_created = None;

      if should_commit {
         match GitOps::open(".") {
            Ok(git) => {
               // Check if there are staged changes
               match git.has_staged_changes() {
                  Ok(true) => {
                     let commit_message = if let Some(ref format) =
                        self.config.git_integration.commit_prefix_format
                     {
                        let prefix = format.replace("{id}", &bug_num.to_string());
                        if let Some(msg) = &message {
                           format!("{} {}", prefix, msg)
                        } else {
                           format!("{} Close issue", prefix)
                        }
                     } else {
                        message.clone().unwrap_or_else(|| {
                           format!("Close {}", self.config.format_issue_ref(bug_num))
                        })
                     };

                     match git.create_commit(&commit_message) {
                        Ok(commit_id) => {
                           commit_created = Some(commit_id);
                        },
                        Err(e) => {
                           if !json {
                              eprintln!("⚠️  Failed to create git commit: {}", e);
                           }
                        },
                     }
                  },
                  Ok(false) => {
                     if !json {
                        eprintln!("⚠️  No staged changes to commit");
                     }
                  },
                  Err(e) => {
                     if !json {
                        eprintln!("⚠️  Failed to check git status: {}", e);
                     }
                  },
               }
            },
            Err(e) => {
               if !json {
                  eprintln!("⚠️  Not a git repository: {}", e);
               }
            },
         }
      }

      if json {
         let output = json!({
             "bug_num": bug_num,
             "status": "closed",
             "commit_created": commit_created,
         });
         println!("{}", serde_json::to_string_pretty(&output)?);
      } else {
         println!("✓ {} marked as CLOSED", self.config.format_issue_ref(bug_num));
         if let Some(commit_id) = commit_created {
            println!("📝 Created git commit: {}", &commit_id[..8]);
         }
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
             "status": "open",
         });
         println!("{}", serde_json::to_string_pretty(&output)?);
      } else {
         println!("↻ {} marked as OPEN", self.config.format_issue_ref(bug_num));
      }

      Ok(())
   }

   pub fn defer(&self, bug_ref: &str, json: bool) -> Result<()> {
      let bug_num = self.storage.resolve_bug_ref(bug_ref)?;

      self.storage.update_issue_metadata(bug_num, |meta| {
         meta.status = Status::Backlog;
      })?;

      if json {
         let output = json!({
             "bug_num": bug_num,
             "status": "backlog",
         });
         println!("{}", serde_json::to_string_pretty(&output)?);
      } else {
         println!("💤 {} moved to BACKLOG", self.config.format_issue_ref(bug_num));
      }

      Ok(())
   }

   pub fn activate(&self, bug_ref: &str, json: bool) -> Result<()> {
      let bug_num = self.storage.resolve_bug_ref(bug_ref)?;

      self.storage.update_issue_metadata(bug_num, |meta| {
         meta.status = Status::NotStarted;
      })?;

      if json {
         let output = json!({
             "bug_num": bug_num,
             "status": "open",
         });
         println!("{}", serde_json::to_string_pretty(&output)?);
      } else {
         println!("⭕ {} activated from BACKLOG", self.config.format_issue_ref(bug_num));
      }

      Ok(())
   }

   pub fn checkpoint(&self, bug_ref: &str, note: String, json: bool) -> Result<()> {
      let bug_num = self.storage.resolve_bug_ref(bug_ref)?;
      let mut issue = self.storage.load_issue(bug_num)?;

      // Auto-detect status changes from checkpoint message
      let mut status_changed = false;
      if note.starts_with("BLOCKED:") || note.to_uppercase().starts_with("BLOCKED:") {
         let reason = note
            .strip_prefix("BLOCKED:")
            .or_else(|| note.strip_prefix("blocked:"))
            .unwrap_or(&note)
            .trim()
            .to_string();

         issue.metadata.status = Status::Blocked;
         issue.metadata.blocked_reason = Some(reason.into());
         status_changed = true;
      } else if note.starts_with("FIXED:")
         || note.to_uppercase().starts_with("FIXED:")
         || note.starts_with("DONE:")
         || note.to_uppercase().starts_with("DONE:")
      {
         issue.metadata.status = Status::Done;
         status_changed = true;
      }

      let timestamp = Utc::now().format("%Y-%m-%d %H:%M").to_string();
      let checkpoint = format!("\n\n**Checkpoint** ({timestamp}): {note}");

      issue.body.push_str(&checkpoint);

      // Determine if open or closed
      let is_open = issue.metadata.status != Status::Closed;
      self.storage.save_issue(&issue, bug_num, is_open)?;

      if json {
         let output = json!({
             "bug_num": bug_num,
             "checkpoint": note,
             "timestamp": timestamp,
             "status_changed": status_changed,
             "new_status": if status_changed { Some(issue.metadata.status.to_string()) } else { None },
         });
         println!("{}", serde_json::to_string_pretty(&output)?);
      } else {
         println!("✓ Added checkpoint to {}", self.config.format_issue_ref(bug_num));
         if status_changed {
            println!("  Status updated to: {}", issue.metadata.status);
         }
      }

      Ok(())
   }

   pub fn context_data(&self) -> Result<ContextResult> {
      let issues = self.storage.list_open_issues()?;

      let mut in_progress = Vec::new();
      let mut blocked = Vec::new();
      let mut high_priority = Vec::new();
      let mut ready = Vec::new();

      for issue_with_id in issues.iter() {
         match issue_with_id.issue.metadata.status {
            Status::InProgress => in_progress.push(issue_with_id.clone()),
            Status::Blocked => blocked.push(issue_with_id.clone()),
            Status::NotStarted => {
               if matches!(
                  issue_with_id.issue.metadata.priority,
                  Priority::Critical | Priority::High
               ) {
                  high_priority.push(issue_with_id.clone());
               }
               ready.push(issue_with_id.clone());
            },
            _ => {},
         }
      }

      Ok(ContextResult {
         active: in_progress,
         blocked,
         high_priority,
         ready_to_start: ready.into_iter().take(5).collect(),
         total_open: issues.len(),
      })
   }

   pub fn context(&self, json: bool) -> Result<()> {
      let context_data = self.context_data()?;

      if json {
         println!("{}", serde_json::to_string_pretty(&context_data)?);
         return Ok(());
      }

      if context_data.total_open == 0 {
         println!("No open issues");
         return Ok(());
      }

      let in_progress = &context_data.active;
      let blocked = &context_data.blocked;
      let high_priority = &context_data.high_priority;
      let ready = &context_data.ready_to_start;
      let total_open = context_data.total_open;

      println!("\n{}", "=".repeat(80));
      println!("CURRENT CONTEXT");
      println!("{}\n", "=".repeat(80));

      if !in_progress.is_empty() {
         println!("🔄 IN PROGRESS ({}):", in_progress.len());
         for issue_with_id in in_progress {
            println!(
               "   {}: {}",
               self.config.format_issue_ref(issue_with_id.id),
               issue_with_id.issue.metadata.title
            );
         }
         println!();
      }

      if !blocked.is_empty() {
         println!("🚫 BLOCKED ({}):", blocked.len());
         for issue_with_id in blocked {
            println!(
               "   {}: {}",
               self.config.format_issue_ref(issue_with_id.id),
               issue_with_id.issue.metadata.title
            );
            if let Some(reason) = &issue_with_id.issue.metadata.blocked_reason {
               println!("      → {}", reason);
            }
         }
         println!();
      }

      if !high_priority.is_empty() {
         println!("⚠️  HIGH PRIORITY QUEUE ({}):", high_priority.len());
         for issue_with_id in high_priority {
            println!(
               "   [{}] {}: {}",
               issue_with_id.issue.metadata.priority.to_string().to_uppercase(),
               self.config.format_issue_ref(issue_with_id.id),
               issue_with_id.issue.metadata.title
            );
         }
         println!();
      }

      if !ready.is_empty() {
         println!("✓ READY TO START ({} tasks):", ready.len());
         for issue_with_id in ready.iter().take(5) {
            println!(
               "   {}: {}",
               self.config.format_issue_ref(issue_with_id.id),
               issue_with_id.issue.metadata.title
            );
         }
         if ready.len() > 5 {
            println!("   ... and {} more", ready.len() - 5);
         }
         println!();
      }

      println!("Total open issues: {}", total_open);

      Ok(())
   }

   pub fn focus(&self, json: bool) -> Result<()> {
      let issues = self.storage.list_open_issues()?;

      let mut focus_issues: Vec<_> = issues
         .iter()
         .map(|issue_with_id| {
            let sort_key = match issue_with_id.issue.metadata.status {
               Status::InProgress | Status::Blocked => -1,
               _ => issue_with_id.issue.metadata.priority.sort_key() as i32,
            };

            (issue_with_id, sort_key)
         })
         .collect();

      focus_issues.sort_by_key(|(_, key)| *key);
      let focus_issues: Vec<_> = focus_issues
         .iter()
         .take(5)
         .map(|(issue_with_id, _)| issue_with_id)
         .collect();

      if json {
         let data: Vec<_> = focus_issues
            .iter()
            .map(|issue_with_id| {
               json!({
                   "num": issue_with_id.id,
                   "title": issue_with_id.issue.metadata.title,
                   "priority": issue_with_id.issue.metadata.priority.to_string(),
                   "status": issue_with_id.issue.metadata.status.to_string(),
               })
            })
            .collect();
         println!("{}", serde_json::to_string_pretty(&data)?);
         return Ok(());
      }

      println!("\n{}", "=".repeat(80));
      println!("FOCUS - Top Priority Tasks");
      println!("{}\n", "=".repeat(80));

      for issue_with_id in focus_issues {
         let marker = issue_with_id.issue.metadata.status.marker();
         let priority_label = format!(
            "[{}]",
            issue_with_id
               .issue
               .metadata
               .priority
               .to_string()
               .to_uppercase()
         );
         println!(
            "{} {:10} {}: {}",
            marker,
            priority_label,
            self.config.format_issue_ref(issue_with_id.id),
            issue_with_id.issue.metadata.title
         );
      }

      Ok(())
   }

   pub fn blocked(&self, json: bool) -> Result<()> {
      let issues = self.storage.list_open_issues()?;

      let blocked_issues: Vec<_> = issues
         .iter()
         .filter(|issue_with_id| issue_with_id.issue.metadata.status == Status::Blocked)
         .collect();

      if json {
         let data: Vec<_> = blocked_issues
            .iter()
            .map(|issue_with_id| {
               json!({
                   "num": issue_with_id.id,
                   "title": issue_with_id.issue.metadata.title,
                   "reason": issue_with_id.issue.metadata.blocked_reason,
                   "priority": issue_with_id.issue.metadata.priority.to_string(),
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

      for issue_with_id in blocked_issues {
         println!(
            "🚫 {}: {}",
            self.config.format_issue_ref(issue_with_id.id),
            issue_with_id.issue.metadata.title
         );
         if let Some(reason) = &issue_with_id.issue.metadata.blocked_reason {
            println!("   Reason: {reason}");
         }
         println!(
            "   Priority: {}\n",
            issue_with_id
               .issue
               .metadata
               .priority
               .to_string()
               .to_uppercase()
         );
      }

      Ok(())
   }

   pub fn ready(&self, json: bool) -> Result<()> {
      let issues = self.storage.list_open_issues()?;

      let mut ready_issues: Vec<_> = issues
         .iter()
         .filter(|issue_with_id| issue_with_id.issue.metadata.status == Status::NotStarted)
         .collect();

      ready_issues.sort_by_key(|issue_with_id| issue_with_id.issue.metadata.priority.sort_key());

      if json {
         let data: Vec<_> = ready_issues
            .iter()
            .map(|issue_with_id| {
               json!({
                   "num": issue_with_id.id,
                   "title": issue_with_id.issue.metadata.title,
                   "priority": issue_with_id.issue.metadata.priority.to_string(),
                   "files": issue_with_id.issue.metadata.files,
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

      for issue_with_id in ready_issues {
         let priority_label = format!(
            "[{}]",
            issue_with_id
               .issue
               .metadata
               .priority
               .to_string()
               .to_uppercase()
         );
         println!(
            "⭕ {:10} {}: {}",
            priority_label,
            self.config.format_issue_ref(issue_with_id.id),
            issue_with_id.issue.metadata.title
         );
         if !issue_with_id.issue.metadata.files.is_empty() {
            println!("   Files: {}", issue_with_id.issue.metadata.files.join(", "));
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

      let data: Vec<serde_yaml::Value> =
         serde_yaml::from_str(&yaml_input).context("Failed to parse YAML input")?;

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

         let tags: Vec<String> = obj
            .get("tags")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
               seq.iter()
                  .filter_map(|v| v.as_str().map(String::from))
                  .collect()
            })
            .unwrap_or_default();

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

         let effort = obj.get("effort").and_then(|v| v.as_str()).map(String::from);

         let context = obj
            .get("context")
            .and_then(|v| v.as_str())
            .map(String::from);

         self.create_issue(
            title,
            priority_str,
            tags,
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
         println!("  {alias} → {}", self.config.format_issue_ref(*bug_num));
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
         println!("✓ Created alias: {alias} → {}", self.config.format_issue_ref(bug_num));
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

   pub fn quick_wins(&self, threshold: &str, json: bool) -> Result<()> {
      let threshold_minutes = parse_effort(threshold)?;
      let issues = self.storage.list_open_issues()?;

      let quick: Vec<_> = issues
         .iter()
         .filter(|issue_with_id| {
            issue_with_id
               .issue
               .metadata
               .effort
               .as_ref()
               .and_then(|e| parse_effort(e).ok())
               .map(|m| m <= threshold_minutes)
               .unwrap_or(false)
         })
         .collect();

      if json {
         let data: Vec<_> = quick
            .iter()
            .map(|issue_with_id| {
               json!({
                   "num": issue_with_id.id,
                   "title": issue_with_id.issue.metadata.title,
                   "priority": issue_with_id.issue.metadata.priority.to_string(),
                   "effort": issue_with_id.issue.metadata.effort,
                   "files": issue_with_id.issue.metadata.files,
               })
            })
            .collect();
         println!("{}", serde_json::to_string_pretty(&data)?);
         return Ok(());
      }

      if quick.is_empty() {
         println!("No quick wins found (threshold: {threshold})");
         return Ok(());
      }

      println!("\n{}", "=".repeat(80));
      println!("QUICK WINS - {} tasks ≤ {threshold}", quick.len());
      println!("{}\n", "=".repeat(80));

      for issue_with_id in quick {
         let marker = issue_with_id.issue.metadata.status.marker();
         let priority_label = format!(
            "[{}]",
            issue_with_id
               .issue
               .metadata
               .priority
               .to_string()
               .to_uppercase()
         );
         let effort = issue_with_id
            .issue
            .metadata
            .effort
            .as_deref()
            .unwrap_or("?");

         println!(
            "{} {:10} ({:>5}) {}: {}",
            marker,
            priority_label,
            effort,
            self.config.format_issue_ref(issue_with_id.id),
            issue_with_id.issue.metadata.title
         );

         if !issue_with_id.issue.metadata.files.is_empty() {
            println!("          Files: {}", issue_with_id.issue.metadata.files.join(", "));
         }
      }

      Ok(())
   }

   pub fn bulk_start(&self, bug_refs: Vec<String>, json: bool) -> Result<()> {
      let mut results = Vec::new();
      let mut errors = Vec::new();

      for bug_ref in bug_refs {
         match self.storage.resolve_bug_ref(&bug_ref) {
            Ok(bug_num) => {
               if let Err(e) = self.storage.update_issue_metadata(bug_num, |meta| {
                  meta.status = Status::InProgress;
                  meta.started = Some(Utc::now());
               }) {
                  errors.push((bug_ref, e.to_string()));
               } else {
                  results.push(bug_num);
               }
            },
            Err(e) => {
               errors.push((bug_ref, e.to_string()));
            },
         }
      }

      if json {
         let output = json!({
             "started": results,
             "errors": errors,
         });
         println!("{}", serde_json::to_string_pretty(&output)?);
      } else {
         if !results.is_empty() {
            println!("🔄 Started {} issues:", results.len());
            for bug_num in &results {
               println!("   {}", self.config.format_issue_ref(*bug_num));
            }
         }

         if !errors.is_empty() {
            println!("\n❌ Errors:");
            for (bug_ref, error) in &errors {
               println!("   {bug_ref}: {error}");
            }
         }
      }

      Ok(())
   }

   pub fn bulk_close(
      &self,
      bug_refs: Vec<String>,
      message: Option<String>,
      json: bool,
   ) -> Result<()> {
      let mut results = Vec::new();
      let mut errors = Vec::new();

      for bug_ref in bug_refs {
         match self.storage.resolve_bug_ref(&bug_ref) {
            Ok(bug_num) => {
               // Update metadata
               if let Err(e) = self.storage.update_issue_metadata(bug_num, |meta| {
                  meta.status = Status::Closed;
                  meta.closed = Some(Utc::now());
               }) {
                  errors.push((bug_ref.clone(), e.to_string()));
                  continue;
               }

               // Add close note if provided
               if let Some(note) = &message
                  && let Ok(mut issue) = self.storage.load_issue(bug_num)
               {
                  let timestamp = Utc::now().format("%Y-%m-%d").to_string();
                  issue
                     .body
                     .push_str(&format!("\n\n---\n\n**Closed** ({timestamp}): {note}\n"));
                  if let Err(e) = self.storage.save_issue(&issue, bug_num, true) {
                     errors.push((bug_ref.clone(), e.to_string()));
                     continue;
                  }
               }

               // Move to closed directory
               if let Err(e) = self.storage.move_issue(bug_num, false) {
                  errors.push((bug_ref, e.to_string()));
               } else {
                  results.push(bug_num);
               }
            },
            Err(e) => {
               errors.push((bug_ref, e.to_string()));
            },
         }
      }

      if json {
         let output = json!({
             "closed": results,
             "errors": errors,
         });
         println!("{}", serde_json::to_string_pretty(&output)?);
      } else {
         if !results.is_empty() {
            println!("✓ Closed {} issues:", results.len());
            for bug_num in &results {
               println!("   {}", self.config.format_issue_ref(*bug_num));
            }
         }

         if !errors.is_empty() {
            println!("\n❌ Errors:");
            for (bug_ref, error) in &errors {
               println!("   {bug_ref}: {error}");
            }
         }
      }

      Ok(())
   }

   pub fn summary(&self, hours: Option<u64>, json: bool) -> Result<()> {
      let hours = hours.unwrap_or(24);
      let since = Utc::now() - Duration::hours(hours as i64);

      let all_issues = self.storage.list_open_issues()?;
      let closed_issues = self.storage.list_closed_issues()?;

      let mut started = Vec::new();
      let mut closed = Vec::new();
      let mut checkpointed = Vec::new();

      // Check open issues for recent activity
      for issue_with_id in &all_issues {
         if let Some(started_time) = issue_with_id.issue.metadata.started
            && started_time > since
         {
            started.push(issue_with_id);
         }

         // Check for recent checkpoints in body
         if issue_with_id.issue.body.contains("**Checkpoint**") {
            // Simple heuristic: if body contains checkpoint, include it
            checkpointed.push(issue_with_id);
         }
      }

      // Check closed issues
      for issue_with_id in &closed_issues {
         if let Some(closed_time) = issue_with_id.issue.metadata.closed
            && closed_time > since
         {
            closed.push(issue_with_id);
         }
      }

      if json {
         let output = json!({
             "since": since.to_rfc3339(),
             "hours": hours,
             "started": started.iter().map(|i| i.id).collect::<Vec<_>>(),
             "closed": closed.iter().map(|i| i.id).collect::<Vec<_>>(),
             "checkpointed": checkpointed.iter().map(|i| i.id).collect::<Vec<_>>(),
         });
         println!("{}", serde_json::to_string_pretty(&output)?);
         return Ok(());
      }

      println!("\n{}", "=".repeat(80));
      println!("SESSION SUMMARY - Last {hours} hours");
      println!("{}\n", "=".repeat(80));

      if !started.is_empty() {
         println!("🔄 Started ({}):", started.len());
         for issue_with_id in &started {
            println!(
               "   {}: {}",
               self.config.format_issue_ref(issue_with_id.id),
               issue_with_id.issue.metadata.title
            );
         }
         println!();
      }

      if !closed.is_empty() {
         println!("✅ Closed ({}):", closed.len());
         for issue_with_id in &closed {
            println!(
               "   {}: {}",
               self.config.format_issue_ref(issue_with_id.id),
               issue_with_id.issue.metadata.title
            );
         }
         println!();
      }

      if !checkpointed.is_empty() {
         println!("📝 Checkpointed ({}):", checkpointed.len());
         for issue_with_id in &checkpointed {
            println!(
               "   {}: {}",
               self.config.format_issue_ref(issue_with_id.id),
               issue_with_id.issue.metadata.title
            );
         }
         println!();
      }

      if started.is_empty() && closed.is_empty() && checkpointed.is_empty() {
         println!("No activity in the last {hours} hours");
      }

      Ok(())
   }

   pub fn dependencies(&self, bug_ref: &str, json: bool) -> Result<()> {
      let bug_num = self.storage.resolve_bug_ref(bug_ref)?;
      let issue = self.storage.load_issue(bug_num)?;

      // Find what this issue depends on
      let depends_on: Vec<_> = issue
         .metadata
         .depends_on
         .iter()
         .filter_map(|&dep_num| {
            self
               .storage
               .load_issue(dep_num)
               .ok()
               .map(|dep_issue| (dep_num, dep_issue))
         })
         .collect();

      // Find what depends on this issue
      let all_issues = self.storage.list_open_issues()?;
      let blocks: Vec<_> = all_issues
         .iter()
         .filter(|issue_with_id| issue_with_id.issue.metadata.depends_on.contains(&bug_num))
         .collect();

      if json {
         let output = json!({
             "issue": {
                 "num": bug_num,
                 "title": issue.metadata.title,
             },
             "depends_on": depends_on.iter().map(|(dep_num, dep)| {
                 json!({
                     "num": dep_num,
                     "title": dep.metadata.title,
                     "status": dep.metadata.status.to_string(),
                 })
             }).collect::<Vec<_>>(),
             "blocks": blocks.iter().map(|issue_with_id| {
                 json!({
                     "num": issue_with_id.id,
                     "title": issue_with_id.issue.metadata.title,
                     "status": issue_with_id.issue.metadata.status.to_string(),
                 })
             }).collect::<Vec<_>>(),
         });
         println!("{}", serde_json::to_string_pretty(&output)?);
         return Ok(());
      }

      println!("\n{}", "=".repeat(80));
      println!(
         "DEPENDENCIES - {}: {}",
         self.config.format_issue_ref(bug_num),
         issue.metadata.title
      );
      println!("{}\n", "=".repeat(80));

      if !depends_on.is_empty() {
         println!("⬇️  Depends on ({}):", depends_on.len());
         for (dep_num, dep) in &depends_on {
            println!(
               "   {} [{}]: {}",
               self.config.format_issue_ref(*dep_num),
               dep.metadata.status,
               dep.metadata.title
            );
         }
         println!();
      } else {
         println!("⬇️  Depends on: (none)\n");
      }

      if !blocks.is_empty() {
         println!("⬆️  Blocks ({}):", blocks.len());
         for issue_with_id in &blocks {
            println!(
               "   {} [{}]: {}",
               self.config.format_issue_ref(issue_with_id.id),
               issue_with_id.issue.metadata.status,
               issue_with_id.issue.metadata.title
            );
         }
         println!();
      } else {
         println!("⬆️  Blocks: (none)\n");
      }

      Ok(())
   }

   pub fn depend(
      &self,
      bug_ref: &str,
      add_deps: Vec<String>,
      remove_deps: Vec<String>,
      json: bool,
   ) -> Result<()> {
      let bug_num = self.storage.resolve_bug_ref(bug_ref)?;

      // Resolve all dependency references
      let mut add_nums = Vec::new();
      for dep_ref in &add_deps {
         let dep_num = self.storage.resolve_bug_ref(dep_ref)?;
         // Verify dependency exists
         self.storage.load_issue(dep_num)?;
         add_nums.push(dep_num);
      }

      let mut remove_nums = Vec::new();
      for dep_ref in &remove_deps {
         let dep_num = self.storage.resolve_bug_ref(dep_ref)?;
         remove_nums.push(dep_num);
      }

      // Check for cycles before adding
      for &dep_num in &add_nums {
         if self.would_create_cycle(bug_num, dep_num)? {
            anyhow::bail!(
               "Adding {} as dependency would create a cycle ({} transitively depends on {})",
               self.config.format_issue_ref(dep_num),
               self.config.format_issue_ref(dep_num),
               self.config.format_issue_ref(bug_num)
            );
         }
      }

      // Update dependencies
      self.storage.update_issue_metadata(bug_num, |meta| {
         // Add new dependencies
         for dep_num in add_nums.iter() {
            if !meta.depends_on.contains(dep_num) {
               meta.depends_on.push(*dep_num);
            }
         }

         // Remove dependencies
         meta.depends_on.retain(|&d| !remove_nums.contains(&d));

         // Sort for consistent ordering
         meta.depends_on.sort_unstable();
      })?;

      // Update reverse dependencies (blocks field)
      for &dep_num in &add_nums {
         self.storage.update_issue_metadata(dep_num, |meta| {
            if !meta.blocks.contains(&bug_num) {
               meta.blocks.push(bug_num);
            }
            meta.blocks.sort_unstable();
         })?;
      }

      for &dep_num in &remove_nums {
         self.storage.update_issue_metadata(dep_num, |meta| {
            meta.blocks.retain(|&b| b != bug_num);
         })?;
      }

      // Load updated issue
      let issue = self.storage.load_issue(bug_num)?;

      if json {
         let output = json!({
             "bug_num": bug_num,
             "added": add_nums,
             "removed": remove_nums,
             "depends_on": issue.metadata.depends_on,
         });
         println!("{}", serde_json::to_string_pretty(&output)?);
      } else {
         println!("✓ Updated dependencies for {}", self.config.format_issue_ref(bug_num));

         if !add_nums.is_empty() {
            println!(
               "  Added: {}",
               add_nums
                  .iter()
                  .map(|n| self.config.format_issue_ref(*n))
                  .collect::<Vec<_>>()
                  .join(", ")
            );
         }

         if !remove_nums.is_empty() {
            println!(
               "  Removed: {}",
               remove_nums
                  .iter()
                  .map(|n| self.config.format_issue_ref(*n))
                  .collect::<Vec<_>>()
                  .join(", ")
            );
         }

         if !issue.metadata.depends_on.is_empty() {
            println!(
               "  Now depends on: {}",
               issue
                  .metadata
                  .depends_on
                  .iter()
                  .map(|n| self.config.format_issue_ref(*n))
                  .collect::<Vec<_>>()
                  .join(", ")
            );
         } else {
            println!("  Now depends on: (none)");
         }
      }

      Ok(())
   }

   pub fn manage_tags(
      &self,
      bug_ref: &str,
      add_tags: Vec<String>,
      remove_tags: Vec<String>,
      list_only: bool,
      json: bool,
   ) -> Result<()> {
      let bug_num = self.storage.resolve_bug_ref(bug_ref)?;
      let issue = self.storage.load_issue(bug_num)?;

      if list_only {
         if json {
            let output = json!({
                "bug_num": bug_num,
                "tags": issue.metadata.tags,
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
         } else {
            println!("Tags for {}:", self.config.format_issue_ref(bug_num));
            if issue.metadata.tags.is_empty() {
               println!("  (no tags)");
            } else {
               for tag in &issue.metadata.tags {
                  println!("  #{}", tag);
               }
            }
         }
         return Ok(());
      }

      if add_tags.is_empty() && remove_tags.is_empty() {
         anyhow::bail!("Specify --add or --remove tags, or use --list to show tags");
      }

      // Normalize tags: lowercase, trim, remove # prefix if present
      let normalize_tag = |t: &str| -> String { t.trim().trim_start_matches('#').to_lowercase() };

      let add_tags: Vec<String> = add_tags.iter().map(|t| normalize_tag(t)).collect();
      let remove_tags: Vec<String> = remove_tags.iter().map(|t| normalize_tag(t)).collect();

      // Update tags
      self.storage.update_issue_metadata(bug_num, |meta| {
         // Add new tags
         for tag in &add_tags {
            let tag_smol = SmolStr::from(tag.as_str());
            if !meta.tags.contains(&tag_smol) {
               meta.tags.push(tag_smol);
            }
         }

         // Remove tags
         let remove_smol: Vec<SmolStr> = remove_tags
            .iter()
            .map(|s| SmolStr::from(s.as_str()))
            .collect();
         meta.tags.retain(|t| !remove_smol.contains(t));

         // Sort for consistent ordering
         meta.tags.sort();
      })?;

      // Load updated issue
      let updated_issue = self.storage.load_issue(bug_num)?;

      if json {
         let output = json!({
             "bug_num": bug_num,
             "added": add_tags,
             "removed": remove_tags,
             "tags": updated_issue.metadata.tags,
         });
         println!("{}", serde_json::to_string_pretty(&output)?);
      } else {
         println!("✓ Updated tags for {}", self.config.format_issue_ref(bug_num));

         if !add_tags.is_empty() {
            println!(
               "  Added: {}",
               add_tags
                  .iter()
                  .map(|t| format!("#{}", t))
                  .collect::<Vec<_>>()
                  .join(" ")
            );
         }

         if !remove_tags.is_empty() {
            println!(
               "  Removed: {}",
               remove_tags
                  .iter()
                  .map(|t| format!("#{}", t))
                  .collect::<Vec<_>>()
                  .join(" ")
            );
         }

         if !updated_issue.metadata.tags.is_empty() {
            println!(
               "  Current tags: {}",
               updated_issue
                  .metadata
                  .tags
                  .iter()
                  .map(|t| format!("#{}", t))
                  .collect::<Vec<_>>()
                  .join(" ")
            );
         } else {
            println!("  Current tags: (none)");
         }
      }

      Ok(())
   }

   fn would_create_cycle(&self, bug_num: u32, dep_num: u32) -> Result<bool> {
      // Check if dep_num transitively depends on bug_num
      // If so, adding bug_num -> dep_num would create a cycle

      let mut visited = std::collections::HashSet::new();
      let mut stack = vec![dep_num];

      while let Some(current) = stack.pop() {
         if current == bug_num {
            return Ok(true); // Cycle detected
         }

         if visited.contains(&current) {
            continue;
         }
         visited.insert(current);

         // Add all dependencies of current to stack
         if let Ok(issue) = self.storage.load_issue(current) {
            for &dep in &issue.metadata.depends_on {
               if !visited.contains(&dep) {
                  stack.push(dep);
               }
            }
         }
      }

      Ok(false)
   }

   pub fn critical_path(&self, json: bool) -> Result<()> {
      let issues = self.storage.list_open_issues()?;

      // Build dependency graph using Tarjan's algorithm for robustness
      // Find strongly connected components (cycles) and longest acyclic path

      let issue_map: std::collections::HashMap<u32, &crate::issue::IssueWithId> =
         issues.iter().map(|i| (i.id, i)).collect();

      // Detect cycles using Tarjan's algorithm
      let cycles = Self::find_cycles(&issues);

      if !cycles.is_empty() && !json {
         println!("\n⚠️  Warning: Dependency cycles detected:");
         for cycle in &cycles {
            println!(
               "   {}",
               cycle
                  .iter()
                  .map(|id| self.config.format_issue_ref(*id))
                  .collect::<Vec<_>>()
                  .join(" → ")
            );
         }
         println!();
      }

      // Find longest path (critical path)
      let mut longest_chain = Vec::new();
      let mut visited = std::collections::HashSet::new();

      fn find_chain(
         issue_id: u32,
         issues: &[crate::issue::IssueWithId],
         visited: &mut std::collections::HashSet<u32>,
         current_chain: &mut Vec<u32>,
         longest: &mut Vec<u32>,
      ) {
         if visited.contains(&issue_id) {
            return; // Cycle or already visited
         }

         visited.insert(issue_id);
         current_chain.push(issue_id);

         if current_chain.len() > longest.len() {
            *longest = current_chain.clone();
         }

         // Find all issues that depend on this one
         for issue_with_id in issues {
            if issue_with_id.issue.metadata.depends_on.contains(&issue_id) {
               find_chain(issue_with_id.id, issues, visited, current_chain, longest);
            }
         }

         current_chain.pop();
         visited.remove(&issue_id);
      }

      // Try starting from each issue
      for issue_with_id in &issues {
         let mut current_chain = Vec::new();
         find_chain(
            issue_with_id.id,
            &issues,
            &mut visited,
            &mut current_chain,
            &mut longest_chain,
         );
      }

      if json {
         let chain_details: Vec<_> = longest_chain
            .iter()
            .filter_map(|&id| issue_map.get(&id).copied())
            .map(|issue_with_id| {
               json!({
                   "num": issue_with_id.id,
                   "title": issue_with_id.issue.metadata.title,
                   "status": issue_with_id.issue.metadata.status.to_string(),
                   "priority": issue_with_id.issue.metadata.priority.to_string(),
               })
            })
            .collect();

         let output = json!({
             "length": longest_chain.len(),
             "chain": chain_details,
         });
         println!("{}", serde_json::to_string_pretty(&output)?);
         return Ok(());
      }

      if longest_chain.is_empty() {
         println!("No dependency chains found");
         return Ok(());
      }

      println!("\n{}", "=".repeat(80));
      println!("CRITICAL PATH - Longest dependency chain ({} issues)", longest_chain.len());
      println!("{}\n", "=".repeat(80));

      for (i, &bug_id) in longest_chain.iter().enumerate() {
         if let Some(&issue_with_id) = issue_map.get(&bug_id) {
            let arrow = if i == 0 { "▶" } else { "↓" };
            println!(
               "{} {} [{}] [{}]: {}",
               arrow,
               self.config.format_issue_ref(bug_id),
               issue_with_id.issue.metadata.status,
               issue_with_id.issue.metadata.priority,
               issue_with_id.issue.metadata.title
            );
         }
      }

      Ok(())
   }

   pub fn deps_graph(&self, focus_issue: Option<&str>, json: bool) -> Result<()> {
      let issues = self.storage.list_open_issues()?;

      if issues.is_empty() {
         println!("No open issues found");
         return Ok(());
      }

      // Build issue map
      let issue_map: std::collections::HashMap<u32, &crate::issue::IssueWithId> =
         issues.iter().map(|i| (i.id, i)).collect();

      // If focus issue provided, filter to show only that issue and its dependencies
      let relevant_issues: Vec<u32> = if let Some(ref_str) = focus_issue {
         let focus_num = self.storage.resolve_bug_ref(ref_str)?;
         self.get_dependency_closure(focus_num, &issues)
      } else {
         issues.iter().map(|i| i.id).collect()
      };

      if json {
         let graph_data: Vec<_> = relevant_issues
            .iter()
            .filter_map(|&id| issue_map.get(&id))
            .map(|issue_with_id| {
               json!({
                   "id": issue_with_id.id,
                   "title": issue_with_id.issue.metadata.title,
                   "status": issue_with_id.issue.metadata.status.to_string(),
                   "depends_on": issue_with_id.issue.metadata.depends_on,
               })
            })
            .collect();

         println!("{}", serde_json::to_string_pretty(&graph_data)?);
         return Ok(());
      }

      // ASCII art visualization
      self.render_ascii_graph(&relevant_issues, &issue_map)?;
      Ok(())
   }

   fn get_dependency_closure(&self, root: u32, issues: &[crate::issue::IssueWithId]) -> Vec<u32> {
      let mut result = std::collections::HashSet::new();
      let mut to_visit = vec![root];

      while let Some(id) = to_visit.pop() {
         if result.contains(&id) {
            continue;
         }
         result.insert(id);

         // Add dependencies (what this issue depends on)
         if let Some(issue_with_id) = issues.iter().find(|i| i.id == id) {
            for &dep in &issue_with_id.issue.metadata.depends_on {
               if !result.contains(&dep) {
                  to_visit.push(dep);
               }
            }
         }

         // Add dependents (what depends on this issue)
         for issue_with_id in issues {
            if issue_with_id.issue.metadata.depends_on.contains(&id)
               && !result.contains(&issue_with_id.id)
            {
               to_visit.push(issue_with_id.id);
            }
         }
      }

      let mut vec: Vec<_> = result.into_iter().collect();
      vec.sort();
      vec
   }

   fn render_ascii_graph(
      &self,
      issue_ids: &[u32],
      issue_map: &std::collections::HashMap<u32, &crate::issue::IssueWithId>,
   ) -> Result<()> {
      println!("\n{}", "=".repeat(80));
      println!("DEPENDENCY GRAPH");
      println!("{}\n", "=".repeat(80));

      // Build layers for topological layout
      let layers = self.compute_graph_layers(issue_ids, issue_map);

      // Render each layer
      for (level, layer_issues) in layers.iter().enumerate() {
         if level > 0 {
            // Draw arrows between layers
            for _ in 0..layer_issues.len() {
               print!("     │     ");
            }
            println!();
            for _ in 0..layer_issues.len() {
               print!("     ▼     ");
            }
            println!("\n");
         }

         // Draw boxes for issues in this layer
         for &id in layer_issues {
            if let Some(issue_with_id) = issue_map.get(&id) {
               let status_marker = issue_with_id.issue.metadata.status.marker();
               let title = if issue_with_id.issue.metadata.title.len() > 20 {
                  format!("{}...", &issue_with_id.issue.metadata.title[..17])
               } else {
                  issue_with_id.issue.metadata.title.as_str().to_string()
               };

               let box_width = 30;
               let line1 = format!("┌{}┐", "─".repeat(box_width - 2));
               let line2 = format!(
                  "│ {} #{:<2} {:>20} │",
                  status_marker,
                  id,
                  format!("[{}]", issue_with_id.issue.metadata.priority)
               );
               let line3 = format!("│ {:<28} │", title);
               let line4 = format!("└{}┘", "─".repeat(box_width - 2));

               if self.config.colored_output {
                  use colored::Colorize;
                  let colored_box = match issue_with_id.issue.metadata.priority {
                     Priority::Critical => {
                        format!("{}\n{}\n{}\n{}", line1, line2, line3, line4).red()
                     },
                     Priority::High => {
                        format!("{}\n{}\n{}\n{}", line1, line2, line3, line4).yellow()
                     },
                     Priority::Medium => {
                        format!("{}\n{}\n{}\n{}", line1, line2, line3, line4).normal()
                     },
                     Priority::Low => {
                        format!("{}\n{}\n{}\n{}", line1, line2, line3, line4).bright_black()
                     },
                  };

                  if issue_with_id.issue.metadata.status == Status::Backlog {
                     print!("{}", colored_box.dimmed());
                  } else {
                     print!("{}", colored_box);
                  }
               } else {
                  println!("{}", line1);
                  println!("{}", line2);
                  println!("{}", line3);
                  println!("{}", line4);
               }

               // Show what this depends on
               if !issue_with_id.issue.metadata.depends_on.is_empty() {
                  let deps_str = issue_with_id
                     .issue
                     .metadata
                     .depends_on
                     .iter()
                     .map(|d| format!("#{}", d))
                     .collect::<Vec<_>>()
                     .join(", ");
                  println!("  └─> depends on: {}", deps_str);
               }

               println!();
            }
         }

         println!();
      }

      Ok(())
   }

   fn compute_graph_layers(
      &self,
      issue_ids: &[u32],
      issue_map: &std::collections::HashMap<u32, &crate::issue::IssueWithId>,
   ) -> Vec<Vec<u32>> {
      let mut layers: Vec<Vec<u32>> = Vec::new();
      let mut assigned = std::collections::HashSet::new();
      let mut remaining: Vec<u32> = issue_ids.to_vec();

      // Assign issues to layers based on dependencies
      while !remaining.is_empty() {
         let mut current_layer = Vec::new();

         for &id in &remaining {
            if let Some(issue_with_id) = issue_map.get(&id) {
               // Can be in this layer if all dependencies are already assigned
               let all_deps_assigned = issue_with_id
                  .issue
                  .metadata
                  .depends_on
                  .iter()
                  .all(|dep| assigned.contains(dep) || !issue_ids.contains(dep));

               if all_deps_assigned {
                  current_layer.push(id);
               }
            }
         }

         if current_layer.is_empty() && !remaining.is_empty() {
            // Cycle detected, just add all remaining to avoid infinite loop
            current_layer = remaining.clone();
         }

         for &id in &current_layer {
            assigned.insert(id);
         }

         remaining.retain(|id| !assigned.contains(id));
         current_layer.sort();
         layers.push(current_layer);
      }

      layers
   }

   pub fn metrics(&self, period: &str, json: bool) -> Result<()> {
      let open_issues = self.storage.list_open_issues()?;
      let closed_issues = self.storage.list_closed_issues()?;

      // Determine time period
      let now = Utc::now();
      let since = match period {
         "day" => now - Duration::days(1),
         "week" => now - Duration::weeks(1),
         "month" => now - Duration::days(30),
         "all" => Utc::now() - Duration::days(36500), // ~100 years
         _ => anyhow::bail!("Invalid period: {}. Use: day, week, month, all", period),
      };

      // Count closed issues in period
      let closed_in_period: Vec<_> = closed_issues
         .iter()
         .filter(|issue_with_id| {
            if let Some(closed_time) = issue_with_id.issue.metadata.closed {
               closed_time > since
            } else {
               false
            }
         })
         .collect();

      // Count opened issues in period
      let opened_in_period: Vec<_> = open_issues
         .iter()
         .chain(closed_issues.iter())
         .filter(|issue_with_id| issue_with_id.issue.metadata.created > since)
         .collect();

      // Calculate average time to close
      let mut close_times = Vec::new();
      for issue_with_id in &closed_in_period {
         if let (Some(created), Some(closed)) =
            (Some(issue_with_id.issue.metadata.created), issue_with_id.issue.metadata.closed)
         {
            let duration = closed - created;
            close_times.push(duration.num_hours());
         }
      }

      let avg_close_time = if !close_times.is_empty() {
         close_times.iter().sum::<i64>() / close_times.len() as i64
      } else {
         0
      };

      // Count by priority
      let mut priority_counts = HashMap::new();
      for issue_with_id in &open_issues {
         *priority_counts
            .entry(issue_with_id.issue.metadata.priority)
            .or_insert(0) += 1;
      }

      // Count by status
      let mut status_counts = HashMap::new();
      for issue_with_id in &open_issues {
         *status_counts
            .entry(issue_with_id.issue.metadata.status)
            .or_insert(0) += 1;
      }

      if json {
         let output = json!({
             "period": period,
             "total_open": open_issues.len(),
             "total_closed": closed_issues.len(),
             "opened_in_period": opened_in_period.len(),
             "closed_in_period": closed_in_period.len(),
             "avg_close_time_hours": avg_close_time,
             "by_priority": {
                 "critical": priority_counts.get(&Priority::Critical).unwrap_or(&0),
                 "high": priority_counts.get(&Priority::High).unwrap_or(&0),
                 "medium": priority_counts.get(&Priority::Medium).unwrap_or(&0),
                 "low": priority_counts.get(&Priority::Low).unwrap_or(&0),
             },
             "by_status": {
                 "open": status_counts.get(&Status::NotStarted).unwrap_or(&0),
                 "active": status_counts.get(&Status::InProgress).unwrap_or(&0),
                 "blocked": status_counts.get(&Status::Blocked).unwrap_or(&0),
                 "backlog": status_counts.get(&Status::Backlog).unwrap_or(&0),
             },
         });
         println!("{}", serde_json::to_string_pretty(&output)?);
         return Ok(());
      }

      println!("\n{}", "=".repeat(80));
      println!("PERFORMANCE METRICS - {}", period.to_uppercase());
      println!("{}\n", "=".repeat(80));

      println!("📊 Overview:");
      println!("  Total open issues:   {}", open_issues.len());
      println!("  Total closed issues: {}", closed_issues.len());
      println!("  Opened in period:    {}", opened_in_period.len());
      println!("  Closed in period:    {}", closed_in_period.len());
      println!();

      if avg_close_time > 0 {
         let days = avg_close_time / 24;
         let hours = avg_close_time % 24;
         println!("⏱️  Average time to close: {} days {} hours", days, hours);
         println!();
      }

      println!("🎯 By Priority:");
      for priority in [Priority::Critical, Priority::High, Priority::Medium, Priority::Low] {
         let count = priority_counts.get(&priority).unwrap_or(&0);
         if *count > 0 {
            println!("  {:10} {}", format!("{}:", priority), count);
         }
      }
      println!();

      println!("📋 By Status:");
      for (status, count) in &status_counts {
         if *count > 0 {
            println!("  {:15} {}", format!("{}:", status), count);
         }
      }

      Ok(())
   }

   // Tarjan's algorithm for finding strongly connected components (cycles)
   fn find_cycles(issues: &[crate::issue::IssueWithId]) -> Vec<Vec<u32>> {
      let mut index = 0;
      let mut stack = Vec::new();
      let mut indices: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
      let mut lowlinks: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
      let mut on_stack: std::collections::HashSet<u32> = std::collections::HashSet::new();
      let mut cycles = Vec::new();

      let issue_map: std::collections::HashMap<u32, &crate::issue::IssueWithId> =
         issues.iter().map(|i| (i.id, i)).collect();

      #[allow(clippy::too_many_arguments)]
      fn strongconnect(
         v: u32,
         issue_map: &std::collections::HashMap<u32, &crate::issue::IssueWithId>,
         index: &mut usize,
         stack: &mut Vec<u32>,
         indices: &mut std::collections::HashMap<u32, usize>,
         lowlinks: &mut std::collections::HashMap<u32, usize>,
         on_stack: &mut std::collections::HashSet<u32>,
         cycles: &mut Vec<Vec<u32>>,
      ) {
         indices.insert(v, *index);
         lowlinks.insert(v, *index);
         *index += 1;
         stack.push(v);
         on_stack.insert(v);

         // Find the issue
         if let Some(&issue_with_id) = issue_map.get(&v) {
            // Check all dependencies
            for &dep in &issue_with_id.issue.metadata.depends_on {
               if !indices.contains_key(&dep) {
                  // Dependency not visited
                  strongconnect(dep, issue_map, index, stack, indices, lowlinks, on_stack, cycles);
                  let dep_lowlink = lowlinks[&dep];
                  let v_lowlink = lowlinks[&v];
                  lowlinks.insert(v, v_lowlink.min(dep_lowlink));
               } else if on_stack.contains(&dep) {
                  // Dependency on stack - part of current SCC
                  let dep_index = indices[&dep];
                  let v_lowlink = lowlinks[&v];
                  lowlinks.insert(v, v_lowlink.min(dep_index));
               }
            }
         }

         // If v is a root node, pop the stack and generate an SCC
         if lowlinks[&v] == indices[&v] {
            let mut scc = Vec::new();
            loop {
               let w = stack.pop().unwrap();
               on_stack.remove(&w);
               scc.push(w);
               if w == v {
                  break;
               }
            }

            // Only report cycles (SCCs with more than one node)
            if scc.len() > 1 {
               scc.reverse();
               cycles.push(scc);
            }
         }
      }

      for issue_with_id in issues {
         if !indices.contains_key(&issue_with_id.id) {
            strongconnect(
               issue_with_id.id,
               &issue_map,
               &mut index,
               &mut stack,
               &mut indices,
               &mut lowlinks,
               &mut on_stack,
               &mut cycles,
            );
         }
      }

      cycles
   }
}
