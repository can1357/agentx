use std::{borrow::Cow, sync::Arc};

use anyhow::Result;
use rmcp::{
   RoleServer, ServerHandler, ServiceExt,
   handler::server::{router::tool::ToolRouter, wrapper::Parameters},
   model::{
      Annotated, CallToolResult, Content, ErrorCode, ErrorData as McpError, Implementation,
      ListResourcesResult, PaginatedRequestParam, ProtocolVersion, RawResource,
      ReadResourceRequestParam, ReadResourceResult, ResourceContents, ServerCapabilities,
      ServerInfo,
   },
   service::RequestContext,
   tool, tool_handler, tool_router,
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::{
   commands::Commands,
   fuzzy::filter_by_tags,
   issue::{Priority, Status},
   storage::Storage,
};

// Tool parameter structures

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum StatusAction {
   Start,
   Block,
   Done,
   Close,
   Reopen,
   Defer,
   Activate,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ContextRequest {
   #[schemars(description = "Output format: 'summary', 'detailed', or 'json'")]
   pub format: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateIssueRequest {
   #[schemars(description = "Issue title")]
   pub title:      String,
   #[schemars(description = "Priority level")]
   pub priority:   Option<Priority>,
   #[schemars(description = "Tags for categorization")]
   pub tags:       Option<Vec<String>>,
   #[schemars(description = "Files related to this issue")]
   pub files:      Option<Vec<String>>,
   #[schemars(description = "Description of the issue/problem")]
   pub issue:      String,
   #[schemars(description = "Impact of the issue")]
   pub impact:     String,
   #[schemars(description = "Acceptance criteria for completion")]
   pub acceptance: String,
   #[schemars(description = "Effort estimate (e.g., '30m', '2h', '1d')")]
   pub effort:     Option<String>,
   #[schemars(description = "Additional context")]
   pub context:    Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateStatusRequest {
   #[schemars(description = "Bug reference (number or alias)")]
   pub bug_ref: String,
   #[schemars(description = "Status action to perform")]
   pub status:  StatusAction,
   #[schemars(description = "Reason (required for 'block', optional for 'close')")]
   pub reason:  Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ShowRequest {
   #[schemars(description = "Bug reference (number or alias)")]
   pub bug_ref: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckpointRequest {
   #[schemars(description = "Bug reference (number or alias)")]
   pub bug_ref: String,
   #[schemars(description = "Progress note/checkpoint message")]
   pub message: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QuickWinsRequest {
   #[schemars(description = "Effort threshold (e.g., '30m', '1h', '2h'). Default: '1h'")]
   pub threshold: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchRequest {
   #[schemars(description = "Search query string")]
   pub query: String,

   #[schemars(description = "Include closed issues in search. Default: false")]
   pub include_closed: Option<bool>,

   #[schemars(description = "Filter by status")]
   pub status: Option<Status>,

   #[schemars(description = "Filter by priority")]
   pub priority: Option<Priority>,

   #[schemars(description = "Filter by tags (fuzzy matching)")]
   pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryRequest {
   #[schemars(description = "Filter by status")]
   pub status:        Option<Status>,
   #[schemars(description = "Filter by priority")]
   pub priority:      Option<Priority>,
   #[schemars(description = "Filter by maximum effort (e.g., '2h')")]
   pub max_effort:    Option<String>,
   #[schemars(description = "Filter by file path (contains match)")]
   pub file_contains: Option<String>,
   #[schemars(description = "Maximum number of results")]
   pub limit:         Option<usize>,
   #[schemars(description = "Filter by tags (fuzzy matching)")]
   pub tags:          Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListRequest {
   #[schemars(description = "Status filter: 'open' or 'closed'. Default: 'open'")]
   pub status: Option<String>,
   #[schemars(description = "Include verbose output with file information")]
   pub verbose: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ImportRequest {
   #[schemars(description = "YAML content to import (array of issue definitions)")]
   pub yaml_content: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AliasListRequest {}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AliasAddRequest {
   #[schemars(description = "Bug reference (number or alias)")]
   pub bug_ref: String,
   #[schemars(description = "New alias name")]
   pub alias: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct AliasRemoveRequest {
   #[schemars(description = "Alias to remove")]
   pub alias: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BulkStartRequest {
   #[schemars(description = "Bug references to start (numbers or aliases)")]
   pub bug_refs: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct BulkCloseRequest {
   #[schemars(description = "Bug references to close (numbers or aliases)")]
   pub bug_refs: Vec<String>,
   #[schemars(description = "Optional close message")]
   pub message: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SummaryRequest {
   #[schemars(description = "Hours to look back (default: 24)")]
   pub hours: Option<u64>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DependenciesRequest {
   #[schemars(description = "Bug reference (number or alias)")]
   pub bug_ref: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DependRequest {
   #[schemars(description = "Bug reference (number or alias)")]
   pub bug_ref: String,
   #[schemars(description = "Dependencies to add")]
   pub add: Option<Vec<String>>,
   #[schemars(description = "Dependencies to remove")]
   pub remove: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TagRequest {
   #[schemars(description = "Bug reference (number or alias)")]
   pub bug_ref: String,
   #[schemars(description = "Tags to add")]
   pub add: Option<Vec<String>>,
   #[schemars(description = "Tags to remove")]
   pub remove: Option<Vec<String>>,
   #[schemars(description = "List tags only")]
   pub list: Option<bool>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct MetricsRequest {
   #[schemars(description = "Time period: 'day', 'week', 'month', 'all'. Default: 'week'")]
   pub period: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct DepsGraphRequest {
   #[schemars(description = "Show only this issue and its dependencies")]
   pub issue: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IssueTrackerMCP {
   commands:    Arc<Commands>,
   storage:     Arc<Storage>,
   tool_router: ToolRouter<Self>,
}

#[tool_router]
impl IssueTrackerMCP {
   pub fn new(storage: Storage, commands: Commands) -> Self {
      Self {
         commands:    Arc::new(commands),
         storage:     Arc::new(storage),
         tool_router: Self::tool_router(),
      }
   }

   pub async fn serve_stdio() -> Result<()> {
      let storage = Storage::new(".");
      let commands = Commands::new(storage.clone());
      let service = Self::new(storage, commands);

      let server = service.serve(rmcp::transport::stdio()).await?;
      server.waiting().await?;

      Ok(())
   }

   #[tool(
      name = "issues_context",
      description = "Get current work context - in-progress, blocked, priority tasks, and backlog \
                     count"
   )]
   async fn context(
      &self,
      Parameters(_request): Parameters<ContextRequest>,
   ) -> Result<CallToolResult, McpError> {
      let issues = self.storage.list_open_issues().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to list issues: {}", e)),
         data:    None,
      })?;

      let mut in_progress = vec![];
      let mut blocked = vec![];
      let mut high_priority = vec![];
      let mut backlog_count = 0;

      for issue_with_id in &issues {
         match issue_with_id.issue.metadata.status {
            Status::InProgress => in_progress.push(issue_with_id),
            Status::Blocked => blocked.push(issue_with_id),
            Status::Backlog => backlog_count += 1,
            Status::NotStarted => {
               if matches!(
                  issue_with_id.issue.metadata.priority,
                  Priority::Critical | Priority::High
               ) {
                  high_priority.push(issue_with_id);
               }
            },
            _ => {},
         }
      }

      let json_output = serde_json::json!({
          "active": in_progress.iter().map(|i| serde_json::json!({
              "num": i.id,
              "title": i.issue.metadata.title,
              "priority": i.issue.metadata.priority.to_string(),
          })).collect::<Vec<_>>(),
          "blocked": blocked.iter().map(|i| serde_json::json!({
              "num": i.id,
              "title": i.issue.metadata.title,
              "reason": i.issue.metadata.blocked_reason,
          })).collect::<Vec<_>>(),
          "high_priority": high_priority.iter().map(|i| serde_json::json!({
              "num": i.id,
              "title": i.issue.metadata.title,
              "priority": i.issue.metadata.priority.to_string(),
          })).collect::<Vec<_>>(),
          "total_open": issues.len() - backlog_count,
          "backlog_count": backlog_count,
      });

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&json_output).unwrap(),
      )]))
   }

   #[tool(name = "issues_create", description = "Create a new issue/task")]
   async fn create(
      &self,
      Parameters(request): Parameters<CreateIssueRequest>,
   ) -> Result<CallToolResult, McpError> {
      let priority = request.priority.unwrap_or(Priority::Medium);
      let priority_str = &priority.to_string();

      match self.commands.create_issue(
         request.title,
         priority_str,
         request.tags.unwrap_or_default(),
         request.files.unwrap_or_default(),
         request.issue,
         request.impact,
         request.acceptance,
         request.effort,
         request.context,
         true,
      ) {
         Ok(_) => {
            let bug_num = self.storage.next_bug_number().map_err(|e| McpError {
               code:    ErrorCode(-32603),
               message: Cow::from(format!("Failed to get bug number: {}", e)),
               data:    None,
            })? - 1;

            let result = serde_json::json!({
                "bug_num": bug_num,
                "message": format!("Created {}", self.commands.config().format_issue_ref(bug_num)),
            });

            Ok(CallToolResult::success(vec![Content::text(
               serde_json::to_string_pretty(&result).unwrap(),
            )]))
         },
         Err(e) => Err(McpError {
            code:    ErrorCode(-32603),
            message: Cow::from(format!("Failed to create issue: {}", e)),
            data:    None,
         }),
      }
   }

   #[tool(
      name = "issues_status",
      description = "Update issue status (start, block, done, close, reopen, defer, activate)"
   )]
   async fn status(
      &self,
      Parameters(request): Parameters<UpdateStatusRequest>,
   ) -> Result<CallToolResult, McpError> {
      let result = match request.status {
         StatusAction::Start => self.commands.start(&request.bug_ref, false, false, true),
         StatusAction::Block => {
            let reason = request.reason.ok_or_else(|| McpError {
               code:    ErrorCode(-32602),
               message: Cow::from("Block status requires a reason"),
               data:    None,
            })?;
            self.commands.block(&request.bug_ref, reason, true)
         },
         StatusAction::Done | StatusAction::Close => {
            self
               .commands
               .close(&request.bug_ref, request.reason, false, false, true)
         },
         StatusAction::Reopen => self.commands.open(&request.bug_ref, true),
         StatusAction::Defer => self.commands.defer(&request.bug_ref, true),
         StatusAction::Activate => self.commands.activate(&request.bug_ref, true),
      };

      let status_str = match request.status {
         StatusAction::Start => "start",
         StatusAction::Block => "block",
         StatusAction::Done => "done",
         StatusAction::Close => "close",
         StatusAction::Reopen => "reopen",
         StatusAction::Defer => "defer",
         StatusAction::Activate => "activate",
      };

      result
         .map(|_| {
            CallToolResult::success(vec![Content::text(
               serde_json::json!({
                   "success": true,
                   "status": status_str,
               })
               .to_string(),
            )])
         })
         .map_err(|e| McpError {
            code:    ErrorCode(-32603),
            message: Cow::from(format!("Failed to update status: {}", e)),
            data:    None,
         })
   }

   #[tool(name = "issues_show", description = "Show full issue details")]
   async fn show(
      &self,
      Parameters(request): Parameters<ShowRequest>,
   ) -> Result<CallToolResult, McpError> {
      let bug_num = self
         .storage
         .resolve_bug_ref(&request.bug_ref)
         .map_err(|e| McpError {
            code:    ErrorCode(-32602),
            message: Cow::from(format!("Invalid bug ref: {}", e)),
            data:    None,
         })?;

      let issue = self.storage.load_issue(bug_num).map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to load issue: {}", e)),
         data:    None,
      })?;

      Ok(CallToolResult::success(vec![Content::text(issue.to_mdx())]))
   }

   #[tool(name = "issues_checkpoint", description = "Add checkpoint/progress note to an issue")]
   async fn checkpoint(
      &self,
      Parameters(request): Parameters<CheckpointRequest>,
   ) -> Result<CallToolResult, McpError> {
      let message = request.message.clone();
      self
         .commands
         .checkpoint(&request.bug_ref, request.message, true)
         .map_err(|e| McpError {
            code:    ErrorCode(-32603),
            message: Cow::from(format!("Failed to add checkpoint: {}", e)),
            data:    None,
         })?;

      let bug_num = self
         .storage
         .resolve_bug_ref(&request.bug_ref)
         .map_err(|e| McpError {
            code:    ErrorCode(-32602),
            message: Cow::from(format!("Invalid bug ref: {}", e)),
            data:    None,
         })?;

      let result = serde_json::json!({
          "success": true,
          "bug_num": bug_num,
          "message": message,
      });

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&result).unwrap(),
      )]))
   }

   #[tool(name = "issues_wins", description = "Find quick-win tasks under effort threshold")]
   async fn wins(
      &self,
      Parameters(request): Parameters<QuickWinsRequest>,
   ) -> Result<CallToolResult, McpError> {
      let threshold = request.threshold.as_deref().unwrap_or("1h");

      let threshold_minutes = crate::utils::parse_effort(threshold).map_err(|e| McpError {
         code:    ErrorCode(-32602),
         message: Cow::from(format!("Invalid threshold: {}", e)),
         data:    None,
      })?;

      let issues = self.storage.list_open_issues().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to list issues: {}", e)),
         data:    None,
      })?;

      let quick: Vec<_> = issues
         .iter()
         .filter(|issue_with_id| {
            issue_with_id
               .issue
               .metadata
               .effort
               .as_ref()
               .and_then(|e| crate::utils::parse_effort(e).ok())
               .map(|m| m <= threshold_minutes)
               .unwrap_or(false)
         })
         .map(|issue_with_id| {
            serde_json::json!({
                "num": issue_with_id.id,
                "title": issue_with_id.issue.metadata.title,
                "priority": issue_with_id.issue.metadata.priority.to_string(),
                "effort": issue_with_id.issue.metadata.effort,
                "status": issue_with_id.issue.metadata.status.to_string(),
                "files": issue_with_id.issue.metadata.files,
                "tags": issue_with_id.issue.metadata.tags,
            })
         })
         .collect();

      let result = serde_json::json!({
          "threshold": threshold,
          "tasks": quick,
          "count": quick.len(),
      });

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&result).unwrap(),
      )]))
   }

   #[tool(
      name = "issues_search",
      description = "Full-text search across all issues (title, content, metadata)"
   )]
   async fn search(
      &self,
      Parameters(request): Parameters<SearchRequest>,
   ) -> Result<CallToolResult, McpError> {
      let query = request.query.to_lowercase();
      let include_closed = request.include_closed.unwrap_or(false);

      let mut all_issues = self.storage.list_open_issues().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to list open issues: {}", e)),
         data:    None,
      })?;

      if include_closed {
         let closed = self.storage.list_closed_issues().map_err(|e| McpError {
            code:    ErrorCode(-32603),
            message: Cow::from(format!("Failed to list closed issues: {}", e)),
            data:    None,
         })?;
         all_issues.extend(closed);
      }

      let mut matches: Vec<_> = all_issues
         .into_iter()
         .filter(|issue_with_id| {
            // Full-text search in title and body
            let title_match = issue_with_id
               .issue
               .metadata
               .title
               .to_lowercase()
               .contains(&query);
            let body_match = issue_with_id.issue.body.to_lowercase().contains(&query);
            let files_match = issue_with_id
               .issue
               .metadata
               .files
               .iter()
               .any(|f| f.to_lowercase().contains(&query));

            let mut matches = title_match || body_match || files_match;

            // Apply status filter if provided
            if let Some(status_filter) = request.status {
               matches = matches && issue_with_id.issue.metadata.status == status_filter;
            }

            // Apply priority filter if provided
            if let Some(priority_filter) = request.priority {
               matches = matches && issue_with_id.issue.metadata.priority == priority_filter;
            }

            matches
         })
         .collect();

      // Apply fuzzy tag filter if provided
      if let Some(ref tags) = request.tags {
         matches = filter_by_tags(matches, tags);
      }

      let results: Vec<_> = matches
         .iter()
         .map(|issue_with_id| {
            // Generate snippet from body
            let body_lower = issue_with_id.issue.body.to_lowercase();
            let snippet = if let Some(pos) = body_lower.find(&query) {
               let start = pos.saturating_sub(50);
               let end = (pos + query.len() + 50).min(issue_with_id.issue.body.len());
               let snippet_text = &issue_with_id.issue.body[start..end];
               format!("...{}...", snippet_text.trim())
            } else {
               issue_with_id
                  .issue
                  .body
                  .lines()
                  .next()
                  .unwrap_or("")
                  .to_string()
            };

            serde_json::json!({
                "num": issue_with_id.id,
                "title": issue_with_id.issue.metadata.title,
                "priority": issue_with_id.issue.metadata.priority.to_string(),
                "status": issue_with_id.issue.metadata.status.to_string(),
                "snippet": snippet,
                "files": issue_with_id.issue.metadata.files,
                "effort": issue_with_id.issue.metadata.effort,
                "tags": issue_with_id.issue.metadata.tags,
            })
         })
         .collect();

      let result = serde_json::json!({
          "query": request.query,
          "matches": results,
          "count": results.len(),
      });

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&result).unwrap(),
      )]))
   }

   #[tool(
      name = "issues_query",
      description = "Query issues with filters (status, priority, effort, files)"
   )]
   async fn query(
      &self,
      Parameters(request): Parameters<QueryRequest>,
   ) -> Result<CallToolResult, McpError> {
      let issues = self.storage.list_open_issues().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to list issues: {}", e)),
         data:    None,
      })?;

      let max_effort_minutes = if let Some(ref max_effort) = request.max_effort {
         Some(crate::utils::parse_effort(max_effort).map_err(|e| McpError {
            code:    ErrorCode(-32602),
            message: Cow::from(format!("Invalid max_effort: {}", e)),
            data:    None,
         })?)
      } else {
         None
      };

      let mut filtered: Vec<_> = issues
         .into_iter()
         .filter(|issue_with_id| {
            // Filter by status
            if let Some(status_filter) = request.status
               && issue_with_id.issue.metadata.status != status_filter {
                  return false;
               }

            // Filter by priority
            if let Some(priority_filter) = request.priority
               && issue_with_id.issue.metadata.priority != priority_filter {
                  return false;
               }

            // Filter by effort
            if let Some(max_effort) = max_effort_minutes {
               if let Some(ref effort) = issue_with_id.issue.metadata.effort {
                  if let Ok(effort_minutes) = crate::utils::parse_effort(effort)
                     && effort_minutes > max_effort
                  {
                     return false;
                  }
               } else {
                  // No effort specified - exclude if filtering by effort
                  return false;
               }
            }

            // Filter by file path
            if let Some(ref file_filter) = request.file_contains
               && !issue_with_id
                  .issue
                  .metadata
                  .files
                  .iter()
                  .any(|f| f.contains(file_filter))
            {
               return false;
            }

            true
         })
         .collect();

      // Apply fuzzy tag filter if provided
      if let Some(ref tags) = request.tags {
         filtered = filter_by_tags(filtered, tags);
      }

      let results: Vec<_> = filtered
         .iter()
         .take(request.limit.unwrap_or(100))
         .map(|issue_with_id| {
            serde_json::json!({
                "num": issue_with_id.id,
                "title": issue_with_id.issue.metadata.title,
                "priority": issue_with_id.issue.metadata.priority.to_string(),
                "status": issue_with_id.issue.metadata.status.to_string(),
                "effort": issue_with_id.issue.metadata.effort,
                "files": issue_with_id.issue.metadata.files,
                "tags": issue_with_id.issue.metadata.tags,
            })
         })
         .collect();

      let result = serde_json::json!({
          "filters": {
              "status": request.status,
              "priority": request.priority,
              "max_effort": request.max_effort,
              "file_contains": request.file_contains,
              "tags": request.tags,
          },
          "issues": results,
          "count": results.len(),
      });

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&result).unwrap(),
      )]))
   }

   #[tool(name = "issues_list", description = "List issues with status filter and verbose option")]
   async fn list(
      &self,
      Parameters(request): Parameters<ListRequest>,
   ) -> Result<CallToolResult, McpError> {
      let status = request.status.as_deref().unwrap_or("open");
      let verbose = request.verbose.unwrap_or(false);

      let issues = match status {
         "open" => self.storage.list_open_issues(),
         "closed" => self.storage.list_closed_issues(),
         _ => {
            return Err(McpError {
               code:    ErrorCode(-32602),
               message: Cow::from("Invalid status: must be 'open' or 'closed'"),
               data:    None,
            })
         },
      }
      .map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to list issues: {}", e)),
         data:    None,
      })?;

      let data: Vec<_> = issues
         .iter()
         .map(|issue_with_id| {
            let mut obj = serde_json::json!({
                "num": issue_with_id.id,
                "title": issue_with_id.issue.metadata.title,
                "priority": issue_with_id.issue.metadata.priority.to_string(),
                "status": issue_with_id.issue.metadata.status.to_string(),
                "tags": issue_with_id.issue.metadata.tags,
            });

            if verbose {
               obj["files"] = serde_json::json!(issue_with_id.issue.metadata.files);
               obj["effort"] = serde_json::json!(issue_with_id.issue.metadata.effort);
               obj["blocked_reason"] = serde_json::json!(issue_with_id.issue.metadata.blocked_reason);
            }

            obj
         })
         .collect();

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&data).unwrap(),
      )]))
   }

   #[tool(name = "issues_focus", description = "Show top priority tasks")]
   async fn focus(
      &self,
      Parameters(_request): Parameters<ContextRequest>,
   ) -> Result<CallToolResult, McpError> {
      let issues = self.storage.list_open_issues().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to list issues: {}", e)),
         data:    None,
      })?;

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
         .map(|(issue_with_id, _)| {
            serde_json::json!({
                "num": issue_with_id.id,
                "title": issue_with_id.issue.metadata.title,
                "priority": issue_with_id.issue.metadata.priority.to_string(),
                "status": issue_with_id.issue.metadata.status.to_string(),
            })
         })
         .collect();

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&focus_issues).unwrap(),
      )]))
   }

   #[tool(name = "issues_blocked", description = "Show blocked tasks")]
   async fn blocked(
      &self,
      Parameters(_request): Parameters<ContextRequest>,
   ) -> Result<CallToolResult, McpError> {
      let issues = self.storage.list_open_issues().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to list issues: {}", e)),
         data:    None,
      })?;

      let blocked_issues: Vec<_> = issues
         .iter()
         .filter(|issue_with_id| issue_with_id.issue.metadata.status == Status::Blocked)
         .map(|issue_with_id| {
            serde_json::json!({
                "num": issue_with_id.id,
                "title": issue_with_id.issue.metadata.title,
                "reason": issue_with_id.issue.metadata.blocked_reason,
                "priority": issue_with_id.issue.metadata.priority.to_string(),
            })
         })
         .collect();

      let result = serde_json::json!({
          "blocked": blocked_issues,
          "count": blocked_issues.len(),
      });

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&result).unwrap(),
      )]))
   }

   #[tool(name = "issues_ready", description = "Show tasks ready to start")]
   async fn ready(
      &self,
      Parameters(_request): Parameters<ContextRequest>,
   ) -> Result<CallToolResult, McpError> {
      let issues = self.storage.list_open_issues().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to list issues: {}", e)),
         data:    None,
      })?;

      let mut ready_issues: Vec<_> = issues
         .iter()
         .filter(|issue_with_id| issue_with_id.issue.metadata.status == Status::NotStarted)
         .collect();

      ready_issues.sort_by_key(|issue_with_id| issue_with_id.issue.metadata.priority.sort_key());

      let data: Vec<_> = ready_issues
         .iter()
         .map(|issue_with_id| {
            serde_json::json!({
                "num": issue_with_id.id,
                "title": issue_with_id.issue.metadata.title,
                "priority": issue_with_id.issue.metadata.priority.to_string(),
                "files": issue_with_id.issue.metadata.files,
            })
         })
         .collect();

      let result = serde_json::json!({
          "ready": data,
          "count": data.len(),
      });

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&result).unwrap(),
      )]))
   }

   #[tool(name = "issues_import", description = "Import multiple issues from YAML")]
   async fn import(
      &self,
      Parameters(request): Parameters<ImportRequest>,
   ) -> Result<CallToolResult, McpError> {
      let data: Vec<serde_yaml::Value> =
         serde_yaml::from_str(&request.yaml_content).map_err(|e| McpError {
            code:    ErrorCode(-32602),
            message: Cow::from(format!("Failed to parse YAML: {}", e)),
            data:    None,
         })?;

      let mut created = Vec::new();

      for item in data {
         let obj = item.as_mapping().ok_or_else(|| McpError {
            code:    ErrorCode(-32602),
            message: Cow::from("Item must be a mapping"),
            data:    None,
         })?;

         let title = obj
            .get("title")
            .and_then(|v| v.as_str())
            .ok_or_else(|| McpError {
               code:    ErrorCode(-32602),
               message: Cow::from("Missing title"),
               data:    None,
            })?
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

         self
            .commands
            .create_issue(
               title,
               priority_str,
               tags,
               files,
               issue,
               impact,
               acceptance,
               effort,
               context,
               true,
            )
            .map_err(|e| McpError {
               code:    ErrorCode(-32603),
               message: Cow::from(format!("Failed to create issue: {}", e)),
               data:    None,
            })?;

         let bug_num = self.storage.next_bug_number().map_err(|e| McpError {
            code:    ErrorCode(-32603),
            message: Cow::from(format!("Failed to get bug number: {}", e)),
            data:    None,
         })? - 1;

         created.push(bug_num);
      }

      let result = serde_json::json!({
          "created": created,
          "count": created.len(),
      });

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&result).unwrap(),
      )]))
   }

   #[tool(name = "issues_alias_list", description = "List all aliases")]
   async fn alias_list(
      &self,
      Parameters(_request): Parameters<AliasListRequest>,
   ) -> Result<CallToolResult, McpError> {
      let aliases = self.storage.load_aliases().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to load aliases: {}", e)),
         data:    None,
      })?;

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&aliases).unwrap(),
      )]))
   }

   #[tool(name = "issues_alias_add", description = "Add an alias for an issue")]
   async fn alias_add(
      &self,
      Parameters(request): Parameters<AliasAddRequest>,
   ) -> Result<CallToolResult, McpError> {
      let bug_num = self
         .storage
         .resolve_bug_ref(&request.bug_ref)
         .map_err(|e| McpError {
            code:    ErrorCode(-32602),
            message: Cow::from(format!("Invalid bug ref: {}", e)),
            data:    None,
         })?;

      self.storage.find_issue_file(bug_num).map_err(|e| McpError {
         code:    ErrorCode(-32602),
         message: Cow::from(format!("Issue not found: {}", e)),
         data:    None,
      })?;

      let mut aliases = self.storage.load_aliases().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to load aliases: {}", e)),
         data:    None,
      })?;

      aliases.insert(request.alias.clone(), bug_num);
      self.storage.save_aliases(&aliases).map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to save aliases: {}", e)),
         data:    None,
      })?;

      let result = serde_json::json!({
          "alias": request.alias,
          "bug_num": bug_num,
      });

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&result).unwrap(),
      )]))
   }

   #[tool(name = "issues_alias_remove", description = "Remove an alias")]
   async fn alias_remove(
      &self,
      Parameters(request): Parameters<AliasRemoveRequest>,
   ) -> Result<CallToolResult, McpError> {
      let mut aliases = self.storage.load_aliases().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to load aliases: {}", e)),
         data:    None,
      })?;

      let bug_num = aliases.remove(&request.alias).ok_or_else(|| McpError {
         code:    ErrorCode(-32602),
         message: Cow::from(format!("Alias '{}' not found", request.alias)),
         data:    None,
      })?;

      self.storage.save_aliases(&aliases).map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to save aliases: {}", e)),
         data:    None,
      })?;

      let result = serde_json::json!({
          "removed": request.alias,
          "was": bug_num,
      });

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&result).unwrap(),
      )]))
   }

   #[tool(name = "issues_bulk_start", description = "Start multiple issues at once")]
   async fn bulk_start(
      &self,
      Parameters(request): Parameters<BulkStartRequest>,
   ) -> Result<CallToolResult, McpError> {
      use chrono::Utc;

      let mut results = Vec::new();
      let mut errors = Vec::new();

      for bug_ref in request.bug_refs {
         match self.storage.resolve_bug_ref(&bug_ref) {
            Ok(bug_num) => {
               if let Err(e) = self.storage.update_issue_metadata(bug_num, |meta| {
                  meta.status = Status::InProgress;
                  meta.started = Some(Utc::now());
               }) {
                  errors.push(serde_json::json!({
                      "bug_ref": bug_ref,
                      "error": e.to_string(),
                  }));
               } else {
                  results.push(bug_num);
               }
            },
            Err(e) => {
               errors.push(serde_json::json!({
                   "bug_ref": bug_ref,
                   "error": e.to_string(),
               }));
            },
         }
      }

      let output = serde_json::json!({
          "started": results,
          "errors": errors,
      });

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&output).unwrap(),
      )]))
   }

   #[tool(name = "issues_bulk_close", description = "Close multiple issues at once")]
   async fn bulk_close(
      &self,
      Parameters(request): Parameters<BulkCloseRequest>,
   ) -> Result<CallToolResult, McpError> {
      use chrono::Utc;

      let mut results = Vec::new();
      let mut errors = Vec::new();

      for bug_ref in request.bug_refs {
         match self.storage.resolve_bug_ref(&bug_ref) {
            Ok(bug_num) => {
               if let Err(e) = self.storage.update_issue_metadata(bug_num, |meta| {
                  meta.status = Status::Closed;
                  meta.closed = Some(Utc::now());
               }) {
                  errors.push(serde_json::json!({
                      "bug_ref": bug_ref.clone(),
                      "error": e.to_string(),
                  }));
                  continue;
               }

               if let Some(ref note) = request.message
                  && let Ok(mut issue) = self.storage.load_issue(bug_num) {
                     let timestamp = Utc::now().format("%Y-%m-%d").to_string();
                     issue
                        .body
                        .push_str(&format!("\n\n---\n\n**Closed** ({timestamp}): {note}\n"));
                     if let Err(e) = self.storage.save_issue(&issue, bug_num, true) {
                        errors.push(serde_json::json!({
                            "bug_ref": bug_ref.clone(),
                            "error": e.to_string(),
                        }));
                        continue;
                     }
                  }

               if let Err(e) = self.storage.move_issue(bug_num, false) {
                  errors.push(serde_json::json!({
                      "bug_ref": bug_ref,
                      "error": e.to_string(),
                  }));
               } else {
                  results.push(bug_num);
               }
            },
            Err(e) => {
               errors.push(serde_json::json!({
                   "bug_ref": bug_ref,
                   "error": e.to_string(),
               }));
            },
         }
      }

      let output = serde_json::json!({
          "closed": results,
          "errors": errors,
      });

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&output).unwrap(),
      )]))
   }

   #[tool(name = "issues_summary", description = "Show session summary (recent activity)")]
   async fn summary(
      &self,
      Parameters(request): Parameters<SummaryRequest>,
   ) -> Result<CallToolResult, McpError> {
      use chrono::{Duration, Utc};

      let hours = request.hours.unwrap_or(24);
      let since = Utc::now() - Duration::hours(hours as i64);

      let all_issues = self.storage.list_open_issues().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to list issues: {}", e)),
         data:    None,
      })?;

      let closed_issues = self.storage.list_closed_issues().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to list closed issues: {}", e)),
         data:    None,
      })?;

      let started: Vec<_> = all_issues
         .iter()
         .filter(|i| {
            if let Some(started_time) = i.issue.metadata.started {
               started_time > since
            } else {
               false
            }
         })
         .map(|i| i.id)
         .collect();

      let closed: Vec<_> = closed_issues
         .iter()
         .filter(|i| {
            if let Some(closed_time) = i.issue.metadata.closed {
               closed_time > since
            } else {
               false
            }
         })
         .map(|i| i.id)
         .collect();

      let checkpointed: Vec<_> = all_issues
         .iter()
         .filter(|i| i.issue.body.contains("**Checkpoint**"))
         .map(|i| i.id)
         .collect();

      let output = serde_json::json!({
          "since": since.to_rfc3339(),
          "hours": hours,
          "started": started,
          "closed": closed,
          "checkpointed": checkpointed,
      });

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&output).unwrap(),
      )]))
   }

   #[tool(name = "issues_dependencies", description = "Show issue dependencies")]
   async fn dependencies(
      &self,
      Parameters(request): Parameters<DependenciesRequest>,
   ) -> Result<CallToolResult, McpError> {
      let bug_num = self
         .storage
         .resolve_bug_ref(&request.bug_ref)
         .map_err(|e| McpError {
            code:    ErrorCode(-32602),
            message: Cow::from(format!("Invalid bug ref: {}", e)),
            data:    None,
         })?;

      let issue = self.storage.load_issue(bug_num).map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to load issue: {}", e)),
         data:    None,
      })?;

      let depends_on: Vec<_> = issue
         .metadata
         .depends_on
         .iter()
         .filter_map(|&dep_num| {
            self
               .storage
               .load_issue(dep_num)
               .ok()
               .map(|dep_issue| {
                  serde_json::json!({
                      "num": dep_num,
                      "title": dep_issue.metadata.title,
                      "status": dep_issue.metadata.status.to_string(),
                  })
               })
         })
         .collect();

      let all_issues = self.storage.list_open_issues().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to list issues: {}", e)),
         data:    None,
      })?;

      let blocks: Vec<_> = all_issues
         .iter()
         .filter(|i| i.issue.metadata.depends_on.contains(&bug_num))
         .map(|i| {
            serde_json::json!({
                "num": i.id,
                "title": i.issue.metadata.title,
                "status": i.issue.metadata.status.to_string(),
            })
         })
         .collect();

      let output = serde_json::json!({
          "issue": {
              "num": bug_num,
              "title": issue.metadata.title,
          },
          "depends_on": depends_on,
          "blocks": blocks,
      });

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&output).unwrap(),
      )]))
   }

   #[tool(name = "issues_depend", description = "Manage issue dependencies")]
   async fn depend(
      &self,
      Parameters(request): Parameters<DependRequest>,
   ) -> Result<CallToolResult, McpError> {
      

      let bug_num = self
         .storage
         .resolve_bug_ref(&request.bug_ref)
         .map_err(|e| McpError {
            code:    ErrorCode(-32602),
            message: Cow::from(format!("Invalid bug ref: {}", e)),
            data:    None,
         })?;

      let add_deps = request.add.unwrap_or_default();
      let remove_deps = request.remove.unwrap_or_default();

      let mut add_nums = Vec::new();
      for dep_ref in &add_deps {
         let dep_num = self
            .storage
            .resolve_bug_ref(dep_ref)
            .map_err(|e| McpError {
               code:    ErrorCode(-32602),
               message: Cow::from(format!("Invalid dependency ref: {}", e)),
               data:    None,
            })?;
         self.storage.load_issue(dep_num).map_err(|e| McpError {
            code:    ErrorCode(-32602),
            message: Cow::from(format!("Dependency issue not found: {}", e)),
            data:    None,
         })?;
         add_nums.push(dep_num);
      }

      let mut remove_nums = Vec::new();
      for dep_ref in &remove_deps {
         let dep_num = self
            .storage
            .resolve_bug_ref(dep_ref)
            .map_err(|e| McpError {
               code:    ErrorCode(-32602),
               message: Cow::from(format!("Invalid dependency ref: {}", e)),
               data:    None,
            })?;
         remove_nums.push(dep_num);
      }

      self
         .storage
         .update_issue_metadata(bug_num, |meta| {
            for dep_num in add_nums.iter() {
               if !meta.depends_on.contains(dep_num) {
                  meta.depends_on.push(*dep_num);
               }
            }
            meta.depends_on.retain(|&d| !remove_nums.contains(&d));
            meta.depends_on.sort_unstable();
         })
         .map_err(|e| McpError {
            code:    ErrorCode(-32603),
            message: Cow::from(format!("Failed to update dependencies: {}", e)),
            data:    None,
         })?;

      for &dep_num in &add_nums {
         self
            .storage
            .update_issue_metadata(dep_num, |meta| {
               if !meta.blocks.contains(&bug_num) {
                  meta.blocks.push(bug_num);
               }
               meta.blocks.sort_unstable();
            })
            .map_err(|e| McpError {
               code:    ErrorCode(-32603),
               message: Cow::from(format!("Failed to update reverse dependencies: {}", e)),
               data:    None,
            })?;
      }

      for &dep_num in &remove_nums {
         self
            .storage
            .update_issue_metadata(dep_num, |meta| {
               meta.blocks.retain(|&b| b != bug_num);
            })
            .map_err(|e| McpError {
               code:    ErrorCode(-32603),
               message: Cow::from(format!("Failed to update reverse dependencies: {}", e)),
               data:    None,
            })?;
      }

      let issue = self.storage.load_issue(bug_num).map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to load issue: {}", e)),
         data:    None,
      })?;

      let output = serde_json::json!({
          "bug_num": bug_num,
          "added": add_nums,
          "removed": remove_nums,
          "depends_on": issue.metadata.depends_on,
      });

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&output).unwrap(),
      )]))
   }

   #[tool(name = "issues_tag", description = "Manage issue tags")]
   async fn tag(
      &self,
      Parameters(request): Parameters<TagRequest>,
   ) -> Result<CallToolResult, McpError> {
      use smol_str::SmolStr;

      let bug_num = self
         .storage
         .resolve_bug_ref(&request.bug_ref)
         .map_err(|e| McpError {
            code:    ErrorCode(-32602),
            message: Cow::from(format!("Invalid bug ref: {}", e)),
            data:    None,
         })?;

      let issue = self.storage.load_issue(bug_num).map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to load issue: {}", e)),
         data:    None,
      })?;

      if request.list.unwrap_or(false) {
         let output = serde_json::json!({
             "bug_num": bug_num,
             "tags": issue.metadata.tags,
         });
         return Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&output).unwrap(),
         )]));
      }

      let add_tags = request.add.unwrap_or_default();
      let remove_tags = request.remove.unwrap_or_default();

      if add_tags.is_empty() && remove_tags.is_empty() {
         return Err(McpError {
            code:    ErrorCode(-32602),
            message: Cow::from("Specify add or remove tags, or use list=true"),
            data:    None,
         });
      }

      let normalize_tag = |t: &str| -> String { t.trim().trim_start_matches('#').to_lowercase() };

      let add_tags: Vec<String> = add_tags.iter().map(|t| normalize_tag(t)).collect();
      let remove_tags: Vec<String> = remove_tags.iter().map(|t| normalize_tag(t)).collect();

      self
         .storage
         .update_issue_metadata(bug_num, |meta| {
            for tag in &add_tags {
               let tag_smol = SmolStr::from(tag.as_str());
               if !meta.tags.contains(&tag_smol) {
                  meta.tags.push(tag_smol);
               }
            }

            let remove_smol: Vec<SmolStr> = remove_tags
               .iter()
               .map(|s| SmolStr::from(s.as_str()))
               .collect();
            meta.tags.retain(|t| !remove_smol.contains(t));
            meta.tags.sort();
         })
         .map_err(|e| McpError {
            code:    ErrorCode(-32603),
            message: Cow::from(format!("Failed to update tags: {}", e)),
            data:    None,
         })?;

      let updated_issue = self.storage.load_issue(bug_num).map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to load updated issue: {}", e)),
         data:    None,
      })?;

      let output = serde_json::json!({
          "bug_num": bug_num,
          "added": add_tags,
          "removed": remove_tags,
          "tags": updated_issue.metadata.tags,
      });

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&output).unwrap(),
      )]))
   }

   #[tool(name = "issues_critical_path", description = "Find longest dependency chain")]
   async fn critical_path(
      &self,
      Parameters(_request): Parameters<ContextRequest>,
   ) -> Result<CallToolResult, McpError> {
      let issues = self.storage.list_open_issues().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to list issues: {}", e)),
         data:    None,
      })?;

      let issue_map: std::collections::HashMap<u32, &crate::issue::IssueWithId> =
         issues.iter().map(|i| (i.id, i)).collect();

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
            return;
         }

         visited.insert(issue_id);
         current_chain.push(issue_id);

         if current_chain.len() > longest.len() {
            *longest = current_chain.clone();
         }

         for issue_with_id in issues {
            if issue_with_id.issue.metadata.depends_on.contains(&issue_id) {
               find_chain(issue_with_id.id, issues, visited, current_chain, longest);
            }
         }

         current_chain.pop();
         visited.remove(&issue_id);
      }

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

      let chain_details: Vec<_> = longest_chain
         .iter()
         .filter_map(|&id| issue_map.get(&id).copied())
         .map(|i| {
            serde_json::json!({
                "num": i.id,
                "title": i.issue.metadata.title,
                "status": i.issue.metadata.status.to_string(),
                "priority": i.issue.metadata.priority.to_string(),
            })
         })
         .collect();

      let output = serde_json::json!({
          "length": longest_chain.len(),
          "chain": chain_details,
      });

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&output).unwrap(),
      )]))
   }

   #[tool(name = "issues_deps_graph", description = "Visualize dependency graph")]
   async fn deps_graph(
      &self,
      Parameters(request): Parameters<DepsGraphRequest>,
   ) -> Result<CallToolResult, McpError> {
      let issues = self.storage.list_open_issues().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to list issues: {}", e)),
         data:    None,
      })?;

      if issues.is_empty() {
         return Ok(CallToolResult::success(vec![Content::text("[]".to_string())]));
      }

      let issue_map: std::collections::HashMap<u32, &crate::issue::IssueWithId> =
         issues.iter().map(|i| (i.id, i)).collect();

      let relevant_issues: Vec<u32> = if let Some(ref_str) = request.issue {
         let focus_num = self
            .storage
            .resolve_bug_ref(&ref_str)
            .map_err(|e| McpError {
               code:    ErrorCode(-32602),
               message: Cow::from(format!("Invalid bug ref: {}", e)),
               data:    None,
            })?;

         let mut result = std::collections::HashSet::new();
         let mut to_visit = vec![focus_num];

         while let Some(id) = to_visit.pop() {
            if result.contains(&id) {
               continue;
            }
            result.insert(id);

            if let Some(issue_with_id) = issues.iter().find(|i| i.id == id) {
               for &dep in &issue_with_id.issue.metadata.depends_on {
                  if !result.contains(&dep) {
                     to_visit.push(dep);
                  }
               }
            }

            for issue_with_id in &issues {
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
      } else {
         issues.iter().map(|i| i.id).collect()
      };

      let graph_data: Vec<_> = relevant_issues
         .iter()
         .filter_map(|&id| issue_map.get(&id))
         .map(|i| {
            serde_json::json!({
                "id": i.id,
                "title": i.issue.metadata.title,
                "status": i.issue.metadata.status.to_string(),
                "depends_on": i.issue.metadata.depends_on,
            })
         })
         .collect();

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&graph_data).unwrap(),
      )]))
   }

   #[tool(name = "issues_metrics", description = "Show performance metrics")]
   async fn metrics(
      &self,
      Parameters(request): Parameters<MetricsRequest>,
   ) -> Result<CallToolResult, McpError> {
      use chrono::{Duration, Utc};
      use std::collections::HashMap;

      let period = request.period.as_deref().unwrap_or("week");

      let open_issues = self.storage.list_open_issues().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to list issues: {}", e)),
         data:    None,
      })?;

      let closed_issues = self.storage.list_closed_issues().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to list closed issues: {}", e)),
         data:    None,
      })?;

      let now = Utc::now();
      let since = match period {
         "day" => now - Duration::days(1),
         "week" => now - Duration::weeks(1),
         "month" => now - Duration::days(30),
         "all" => Utc::now() - Duration::days(36500),
         _ => {
            return Err(McpError {
               code:    ErrorCode(-32602),
               message: Cow::from("Invalid period: use day, week, month, or all"),
               data:    None,
            })
         },
      };

      let closed_in_period: Vec<_> = closed_issues
         .iter()
         .filter(|i| {
            if let Some(closed_time) = i.issue.metadata.closed {
               closed_time > since
            } else {
               false
            }
         })
         .collect();

      let opened_in_period: Vec<_> = open_issues
         .iter()
         .chain(closed_issues.iter())
         .filter(|i| i.issue.metadata.created > since)
         .collect();

      let mut close_times = Vec::new();
      for i in &closed_in_period {
         if let Some(closed) = i.issue.metadata.closed {
            let duration = closed - i.issue.metadata.created;
            close_times.push(duration.num_hours());
         }
      }

      let avg_close_time = if !close_times.is_empty() {
         close_times.iter().sum::<i64>() / close_times.len() as i64
      } else {
         0
      };

      let mut priority_counts = HashMap::new();
      for i in &open_issues {
         *priority_counts
            .entry(i.issue.metadata.priority)
            .or_insert(0) += 1;
      }

      let mut status_counts = HashMap::new();
      for i in &open_issues {
         *status_counts.entry(i.issue.metadata.status).or_insert(0) += 1;
      }

      let output = serde_json::json!({
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

      Ok(CallToolResult::success(vec![Content::text(
         serde_json::to_string_pretty(&output).unwrap(),
      )]))
   }
}

#[tool_handler]
impl ServerHandler for IssueTrackerMCP {
   fn get_info(&self) -> ServerInfo {
      ServerInfo {
         protocol_version: ProtocolVersion::V_2024_11_05,
         capabilities:     ServerCapabilities::builder()
            .enable_tools()
            .enable_resources()
            .build(),
         server_info:      Implementation {
            name:        "agentx-mcp".into(),
            version:     env!("CARGO_PKG_VERSION").into(),
            title:       None,
            website_url: None,
            icons:       None,
         },
         instructions:     Some(
            "Issue tracker MCP server providing tools for managing tasks and bugs. Use \
             issues_context to see current work, issues_create to add tasks, issues_status to \
             update status (start, block, close, defer, activate), issues_checkpoint for progress \
             notes, issues_search for full-text search, issues_query for advanced filtering, and \
             issues_wins to find quick-win tasks. Defer non-urgent tasks to backlog with 'defer' \
             status."
               .to_string(),
         ),
      }
   }

   async fn list_resources(
      &self,
      _request: Option<PaginatedRequestParam>,
      _context: RequestContext<RoleServer>,
   ) -> Result<ListResourcesResult, McpError> {
      let open_issues = self.storage.list_open_issues().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to list issues: {}", e)),
         data:    None,
      })?;

      let closed_issues = self.storage.list_closed_issues().map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to list closed issues: {}", e)),
         data:    None,
      })?;

      let mut resources = Vec::new();

      // Add open issues
      for issue_with_id in open_issues {
         resources.push(Annotated::new(
            RawResource {
               uri:         format!("issue://{}", issue_with_id.id),
               name:        format!(
                  "{}: {}",
                  self.commands.config().format_issue_ref(issue_with_id.id),
                  issue_with_id.issue.metadata.title
               ),
               title:       None,
               description: Some(format!(
                  "[{}] {} - {}",
                  issue_with_id.issue.metadata.status,
                  issue_with_id.issue.metadata.priority,
                  issue_with_id.issue.metadata.title
               )),
               mime_type:   Some("text/markdown".into()),
               size:        None,
               icons:       None,
            },
            None,
         ));
      }

      // Add closed issues
      for issue_with_id in closed_issues {
         resources.push(Annotated::new(
            RawResource {
               uri:         format!("issue://{}", issue_with_id.id),
               name:        format!(
                  "{}: {} (closed)",
                  self.commands.config().format_issue_ref(issue_with_id.id),
                  issue_with_id.issue.metadata.title
               ),
               title:       None,
               description: Some(format!("[closed] {}", issue_with_id.issue.metadata.title)),
               mime_type:   Some("text/markdown".into()),
               size:        None,
               icons:       None,
            },
            None,
         ));
      }

      Ok(ListResourcesResult { next_cursor: None, resources })
   }

   async fn read_resource(
      &self,
      request: ReadResourceRequestParam,
      _context: RequestContext<RoleServer>,
   ) -> Result<ReadResourceResult, McpError> {
      let bug_num = request
         .uri
         .strip_prefix("issue://")
         .and_then(|s| s.parse::<u32>().ok())
         .ok_or_else(|| McpError {
            code:    ErrorCode(-32602),
            message: Cow::from(format!("Invalid issue URI: {}", request.uri)),
            data:    None,
         })?;

      let issue = self.storage.load_issue(bug_num).map_err(|e| McpError {
         code:    ErrorCode(-32603),
         message: Cow::from(format!("Failed to load issue: {}", e)),
         data:    None,
      })?;

      Ok(ReadResourceResult {
         contents: vec![ResourceContents::TextResourceContents {
            uri:       request.uri,
            mime_type: Some("text/markdown".into()),
            text:      issue.to_mdx(),
            meta:      None,
         }],
      })
   }
}
