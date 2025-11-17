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
pub struct ContextRequest {
   #[schemars(description = "Output format: 'summary', 'detailed', or 'json'")]
   pub format: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateIssueRequest {
   #[schemars(description = "Issue title")]
   pub title:      String,
   #[schemars(description = "Priority: 'critical', 'high', 'medium', or 'low'")]
   pub priority:   Option<String>,
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
   #[schemars(description = "New status: 'start', 'block', 'done', 'close', or 'reopen'")]
   pub status:  String,
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

   #[schemars(description = "Filter by status (e.g., 'active', 'blocked')")]
   pub status: Option<String>,

   #[schemars(description = "Filter by priority (e.g., 'critical', 'high')")]
   pub priority: Option<String>,

   #[schemars(description = "Filter by tags (fuzzy matching)")]
   pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueryRequest {
   #[schemars(
      description = "Filter by status: 'open', 'active', 'blocked', 'done', 'closed', 'backlog'"
   )]
   pub status:        Option<String>,
   #[schemars(description = "Filter by priority: 'critical', 'high', 'medium', 'low'")]
   pub priority:      Option<String>,
   #[schemars(description = "Filter by maximum effort (e.g., '2h')")]
   pub max_effort:    Option<String>,
   #[schemars(description = "Filter by file path (contains match)")]
   pub file_contains: Option<String>,
   #[schemars(description = "Maximum number of results")]
   pub limit:         Option<usize>,
   #[schemars(description = "Filter by tags (fuzzy matching)")]
   pub tags:          Option<Vec<String>>,
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
      name = "issues/context",
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

   #[tool(name = "issues/create", description = "Create a new issue/task")]
   async fn create(
      &self,
      Parameters(request): Parameters<CreateIssueRequest>,
   ) -> Result<CallToolResult, McpError> {
      let priority_str = request.priority.as_deref().unwrap_or("medium");

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
      name = "issues/status",
      description = "Update issue status (start, block, done, close, reopen, defer, activate)"
   )]
   async fn status(
      &self,
      Parameters(request): Parameters<UpdateStatusRequest>,
   ) -> Result<CallToolResult, McpError> {
      let result = match request.status.as_str() {
         "start" => self.commands.start(&request.bug_ref, false, false, true),
         "block" => {
            let reason = request.reason.ok_or_else(|| McpError {
               code:    ErrorCode(-32602),
               message: Cow::from("Block status requires a reason"),
               data:    None,
            })?;
            self.commands.block(&request.bug_ref, reason, true)
         },
         "close" => self
            .commands
            .close(&request.bug_ref, request.reason, false, false, true),
         "reopen" => self.commands.open(&request.bug_ref, true),
         "defer" => self.commands.defer(&request.bug_ref, true),
         "activate" => self.commands.activate(&request.bug_ref, true),
         _ => {
            return Err(McpError {
               code:    ErrorCode(-32602),
               message: Cow::from(format!("Invalid status: {}", request.status)),
               data:    None,
            });
         },
      };

      result
         .map(|_| {
            CallToolResult::success(vec![Content::text(
               serde_json::json!({
                   "success": true,
                   "status": request.status,
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

   #[tool(name = "issues/show", description = "Show full issue details")]
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

   #[tool(name = "issues/checkpoint", description = "Add checkpoint/progress note to an issue")]
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

   #[tool(name = "issues/wins", description = "Find quick-win tasks under effort threshold")]
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
      name = "issues/search",
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
            if let Some(ref status_filter) = request.status {
               matches =
                  matches && issue_with_id.issue.metadata.status.to_string() == *status_filter;
            }

            // Apply priority filter if provided
            if let Some(ref priority_filter) = request.priority {
               matches =
                  matches && issue_with_id.issue.metadata.priority.to_string() == *priority_filter;
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
      name = "issues/query",
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
            if let Some(ref status_filter) = request.status {
               let status_str = issue_with_id.issue.metadata.status.to_string();
               if status_str != *status_filter {
                  return false;
               }
            }

            // Filter by priority
            if let Some(ref priority_filter) = request.priority {
               let priority_str = issue_with_id.issue.metadata.priority.to_string();
               if priority_str != *priority_filter {
                  return false;
               }
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
             issues/context to see current work, issues/create to add tasks, issues/status to \
             update status (start, block, close, defer, activate), issues/checkpoint for progress \
             notes, issues/search for full-text search, issues/query for advanced filtering, and \
             issues/wins to find quick-win tasks. Defer non-urgent tasks to backlog with 'defer' \
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
