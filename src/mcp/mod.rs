use crate::commands::Commands;
use crate::issue::{Priority, Status};
use crate::storage::Storage;
use anyhow::Result;
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{
        CallToolResult, Content, ErrorCode, ErrorData as McpError, GetPromptResult,
        Implementation, ListPromptsResult, ListResourcesResult, Prompt, PromptMessage,
        ProtocolVersion, ReadResourceResult, Resource, ResourceContents, ServerCapabilities,
        ServerInfo, TextContent,
    },
    tool, tool_handler, tool_router, ServiceExt, ServerHandler,
};
use schemars::JsonSchema;
use serde::Deserialize;
use std::borrow::Cow;
use std::sync::Arc;

// Tool parameter structures

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ContextRequest {
    #[schemars(description = "Output format: 'summary', 'detailed', or 'json'")]
    pub format: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreateIssueRequest {
    #[schemars(description = "Issue title")]
    pub title: String,
    #[schemars(description = "Priority: 'critical', 'high', 'medium', or 'low'")]
    pub priority: Option<String>,
    #[schemars(description = "Files related to this issue")]
    pub files: Option<Vec<String>>,
    #[schemars(description = "Description of the issue/problem")]
    pub issue: String,
    #[schemars(description = "Impact of the issue")]
    pub impact: String,
    #[schemars(description = "Acceptance criteria for completion")]
    pub acceptance: String,
    #[schemars(description = "Effort estimate (e.g., '30m', '2h', '1d')")]
    pub effort: Option<String>,
    #[schemars(description = "Additional context")]
    pub context: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdateStatusRequest {
    #[schemars(description = "Bug reference (number or alias)")]
    pub bug_ref: String,
    #[schemars(description = "New status: 'start', 'block', 'done', 'close', or 'reopen'")]
    pub status: String,
    #[schemars(description = "Reason (required for 'block', optional for 'close')")]
    pub reason: Option<String>,
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
pub struct QueryRequest {
    #[schemars(description = "Filter by status: 'not_started', 'in_progress', 'blocked', 'done'")]
    pub status: Option<String>,
    #[schemars(description = "Filter by priority: 'critical', 'high', 'medium', 'low'")]
    pub priority: Option<String>,
    #[schemars(description = "Filter by maximum effort (e.g., '2h')")]
    pub max_effort: Option<String>,
    #[schemars(description = "Filter by file path (contains match)")]
    pub file_contains: Option<String>,
    #[schemars(description = "Maximum number of results")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct IssueTrackerMCP {
    commands: Arc<Commands>,
    storage: Arc<Storage>,
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl IssueTrackerMCP {
    pub fn new(storage: Storage, commands: Commands) -> Self {
        Self {
            commands: Arc::new(commands),
            storage: Arc::new(storage),
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

    #[tool(description = "Get current work context - in-progress, blocked, and priority tasks")]
    async fn issues_context(
        &self,
        Parameters(_request): Parameters<ContextRequest>,
    ) -> Result<CallToolResult, McpError> {
        let issues = self.storage.list_open_issues().map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to list issues: {}", e)),
            data: None,
        })?;

        let mut in_progress = vec![];
        let mut blocked = vec![];
        let mut high_priority = vec![];

        for issue in &issues {
            match issue.metadata.status {
                Status::InProgress => in_progress.push(&issue.metadata),
                Status::Blocked => blocked.push(&issue.metadata),
                Status::NotStarted => {
                    if matches!(
                        issue.metadata.priority,
                        Priority::Critical | Priority::High
                    ) {
                        high_priority.push(&issue.metadata);
                    }
                }
                _ => {}
            }
        }

        let json_output = serde_json::json!({
            "in_progress": in_progress.iter().map(|m| serde_json::json!({
                "num": m.id,
                "title": m.title,
                "priority": m.priority.to_string(),
            })).collect::<Vec<_>>(),
            "blocked": blocked.iter().map(|m| serde_json::json!({
                "num": m.id,
                "title": m.title,
                "reason": m.blocked_reason,
            })).collect::<Vec<_>>(),
            "high_priority": high_priority.iter().map(|m| serde_json::json!({
                "num": m.id,
                "title": m.title,
                "priority": m.priority.to_string(),
            })).collect::<Vec<_>>(),
            "total_open": issues.len(),
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&json_output).unwrap(),
        )]))
    }

    #[tool(description = "Create a new issue/task")]
    async fn issues_create(
        &self,
        Parameters(request): Parameters<CreateIssueRequest>,
    ) -> Result<CallToolResult, McpError> {
        let priority_str = request.priority.as_deref().unwrap_or("medium");

        match self.commands.create_issue(
            request.title,
            priority_str,
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
                    code: ErrorCode(-32603),
                    message: Cow::from(format!("Failed to get bug number: {}", e)),
                    data: None,
                })? - 1;

                let result = serde_json::json!({
                    "bug_num": bug_num,
                    "message": format!("Created BUG-{}", bug_num),
                });

                Ok(CallToolResult::success(vec![Content::text(
                    serde_json::to_string_pretty(&result).unwrap(),
                )]))
            }
            Err(e) => Err(McpError {
                code: ErrorCode(-32603),
                message: Cow::from(format!("Failed to create issue: {}", e)),
                data: None,
            }),
        }
    }

    #[tool(description = "Update issue status (start, block, done, close, reopen)")]
    async fn issues_update_status(
        &self,
        Parameters(request): Parameters<UpdateStatusRequest>,
    ) -> Result<CallToolResult, McpError> {
        let result = match request.status.as_str() {
            "start" => self.commands.start(&request.bug_ref, true),
            "block" => {
                let reason = request.reason.ok_or_else(|| McpError {
                    code: ErrorCode(-32602),
                    message: Cow::from("Block status requires a reason"),
                    data: None,
                })?;
                self.commands.block(&request.bug_ref, reason, true)
            }
            "close" => self.commands.close(&request.bug_ref, request.reason, true),
            "reopen" => self.commands.open(&request.bug_ref, true),
            _ => {
                return Err(McpError {
                    code: ErrorCode(-32602),
                    message: Cow::from(format!("Invalid status: {}", request.status)),
                    data: None,
                })
            }
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
                code: ErrorCode(-32603),
                message: Cow::from(format!("Failed to update status: {}", e)),
                data: None,
            })
    }

    #[tool(description = "Show full issue details")]
    async fn issues_show(
        &self,
        Parameters(request): Parameters<ShowRequest>,
    ) -> Result<CallToolResult, McpError> {
        let bug_num = self
            .storage
            .resolve_bug_ref(&request.bug_ref)
            .map_err(|e| McpError {
                code: ErrorCode(-32602),
                message: Cow::from(format!("Invalid bug ref: {}", e)),
                data: None,
            })?;

        let issue = self.storage.load_issue(bug_num).map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to load issue: {}", e)),
            data: None,
        })?;

        Ok(CallToolResult::success(vec![Content::text(
            issue.to_mdx(),
        )]))
    }

    #[tool(description = "Add checkpoint/progress note to an issue")]
    async fn issues_checkpoint(
        &self,
        Parameters(request): Parameters<CheckpointRequest>,
    ) -> Result<CallToolResult, McpError> {
        self.commands
            .checkpoint(&request.bug_ref, request.message.clone(), true)
            .map_err(|e| McpError {
                code: ErrorCode(-32603),
                message: Cow::from(format!("Failed to add checkpoint: {}", e)),
                data: None,
            })?;

        let bug_num = self
            .storage
            .resolve_bug_ref(&request.bug_ref)
            .map_err(|e| McpError {
                code: ErrorCode(-32602),
                message: Cow::from(format!("Invalid bug ref: {}", e)),
                data: None,
            })?;

        let result = serde_json::json!({
            "success": true,
            "bug_num": bug_num,
            "message": request.message,
        });

        Ok(CallToolResult::success(vec![Content::text(
            serde_json::to_string_pretty(&result).unwrap(),
        )]))
    }

    #[tool(description = "Find quick-win tasks under effort threshold")]
    async fn issues_quick_wins(
        &self,
        Parameters(request): Parameters<QuickWinsRequest>,
    ) -> Result<CallToolResult, McpError> {
        let threshold = request.threshold.as_deref().unwrap_or("1h");

        let threshold_minutes = crate::utils::parse_effort(threshold).map_err(|e| McpError {
            code: ErrorCode(-32602),
            message: Cow::from(format!("Invalid threshold: {}", e)),
            data: None,
        })?;

        let issues = self.storage.list_open_issues().map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to list issues: {}", e)),
            data: None,
        })?;

        let quick: Vec<_> = issues
            .iter()
            .filter(|i| {
                i.metadata
                    .effort
                    .as_ref()
                    .and_then(|e| crate::utils::parse_effort(e).ok())
                    .map(|m| m <= threshold_minutes)
                    .unwrap_or(false)
            })
            .map(|issue| {
                serde_json::json!({
                    "num": issue.metadata.id,
                    "title": issue.metadata.title,
                    "priority": issue.metadata.priority.to_string(),
                    "effort": issue.metadata.effort,
                    "status": issue.metadata.status.to_string(),
                    "files": issue.metadata.files,
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

    #[tool(description = "Query issues with filters (status, priority, effort, files)")]
    async fn issues_query(
        &self,
        Parameters(request): Parameters<QueryRequest>,
    ) -> Result<CallToolResult, McpError> {
        let issues = self.storage.list_open_issues().map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to list issues: {}", e)),
            data: None,
        })?;

        let max_effort_minutes = if let Some(ref max_effort) = request.max_effort {
            Some(crate::utils::parse_effort(max_effort).map_err(|e| McpError {
                code: ErrorCode(-32602),
                message: Cow::from(format!("Invalid max_effort: {}", e)),
                data: None,
            })?)
        } else {
            None
        };

        let filtered: Vec<_> = issues
            .iter()
            .filter(|issue| {
                // Filter by status
                if let Some(ref status_filter) = request.status {
                    let status_str = issue.metadata.status.to_string();
                    if status_str != *status_filter {
                        return false;
                    }
                }

                // Filter by priority
                if let Some(ref priority_filter) = request.priority {
                    let priority_str = issue.metadata.priority.to_string();
                    if priority_str != *priority_filter {
                        return false;
                    }
                }

                // Filter by effort
                if let Some(max_effort) = max_effort_minutes {
                    if let Some(ref effort) = issue.metadata.effort {
                        if let Ok(effort_minutes) = crate::utils::parse_effort(effort) {
                            if effort_minutes > max_effort {
                                return false;
                            }
                        }
                    } else {
                        // No effort specified - exclude if filtering by effort
                        return false;
                    }
                }

                // Filter by file path
                if let Some(ref file_filter) = request.file_contains {
                    if !issue
                        .metadata
                        .files
                        .iter()
                        .any(|f| f.contains(file_filter))
                    {
                        return false;
                    }
                }

                true
            })
            .take(request.limit.unwrap_or(100))
            .map(|issue| {
                serde_json::json!({
                    "num": issue.metadata.id,
                    "title": issue.metadata.title,
                    "priority": issue.metadata.priority.to_string(),
                    "status": issue.metadata.status.to_string(),
                    "effort": issue.metadata.effort,
                    "files": issue.metadata.files,
                })
            })
            .collect();

        let result = serde_json::json!({
            "filters": {
                "status": request.status,
                "priority": request.priority,
                "max_effort": request.max_effort,
                "file_contains": request.file_contains,
            },
            "issues": filtered,
            "count": filtered.len(),
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
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .enable_prompts()
                .build(),
            server_info: Implementation {
                name: "agentx-mcp".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                title: None,
                website_url: None,
                icons: None,
            },
            instructions: Some(
                "Issue tracker MCP server providing tools for managing tasks and bugs. \
                 Use issues/context to see current work, issues/create to add tasks, \
                 issues/checkpoint for progress notes, issues/query for advanced filtering, \
                 and issues/quick_wins to find low-effort tasks. \
                 \
                 Resources: issue://<num> for MDX content. \
                 Prompts: triage, standup, weekly-review."
                    .to_string(),
            ),
        }
    }

    async fn list_resources(&self) -> Result<ListResourcesResult, McpError> {
        let issues = self.storage.list_open_issues().map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to list issues: {}", e)),
            data: None,
        })?;

        let resources: Vec<Resource> = issues
            .into_iter()
            .map(|issue| Resource {
                uri: format!("issue://{}", issue.metadata.id).into(),
                name: format!("BUG-{}: {}", issue.metadata.id, issue.metadata.title),
                description: Some(format!(
                    "Priority: {}, Status: {}",
                    issue.metadata.priority, issue.metadata.status
                )),
                mime_type: Some("text/markdown".into()),
            })
            .collect();

        Ok(ListResourcesResult { resources })
    }

    async fn read_resource(&self, uri: &str) -> Result<ReadResourceResult, McpError> {
        // Parse URI: issue://<num>
        let bug_num_str = uri
            .strip_prefix("issue://")
            .ok_or_else(|| McpError {
                code: ErrorCode(-32602),
                message: Cow::from(format!("Invalid resource URI: {uri}")),
                data: None,
            })?;

        let bug_num: u32 = bug_num_str.parse().map_err(|_| McpError {
            code: ErrorCode(-32602),
            message: Cow::from(format!("Invalid bug number in URI: {bug_num_str}")),
            data: None,
        })?;

        let issue = self.storage.load_issue(bug_num).map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to load issue: {}", e)),
            data: None,
        })?;

        Ok(ReadResourceResult {
            contents: vec![ResourceContents::Text(TextContent {
                uri: uri.into(),
                mime_type: Some("text/markdown".into()),
                text: issue.to_mdx(),
            })],
        })
    }

    async fn list_prompts(&self) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult {
            prompts: vec![
                Prompt {
                    name: "triage".into(),
                    description: Some(
                        "Triage and prioritize current tasks based on context".into(),
                    ),
                    arguments: None,
                },
                Prompt {
                    name: "standup".into(),
                    description: Some("Generate daily standup summary".into()),
                    arguments: None,
                },
                Prompt {
                    name: "weekly-review".into(),
                    description: Some("Generate weekly review of completed and pending work".into()),
                    arguments: None,
                },
            ],
        })
    }

    async fn get_prompt(
        &self,
        name: &str,
        _arguments: Option<std::collections::HashMap<String, String>>,
    ) -> Result<GetPromptResult, McpError> {
        let issues = self.storage.list_open_issues().map_err(|e| McpError {
            code: ErrorCode(-32603),
            message: Cow::from(format!("Failed to list issues: {}", e)),
            data: None,
        })?;

        let context_summary = serde_json::json!({
            "total_open": issues.len(),
            "in_progress": issues.iter().filter(|i| i.metadata.status == Status::InProgress).count(),
            "blocked": issues.iter().filter(|i| i.metadata.status == Status::Blocked).count(),
            "critical": issues.iter().filter(|i| i.metadata.priority == Priority::Critical).count(),
        });

        let prompt_text = match name {
            "triage" => format!(
                "# Task Triage\n\n\
                Current context: {} open issues\n\
                - {} in progress\n\
                - {} blocked\n\
                - {} critical priority\n\n\
                Review all open issues and suggest:\n\
                1. Priority adjustments based on impact and dependencies\n\
                2. Tasks that should be started next\n\
                3. Blocked tasks that need unblocking\n\
                4. Issues that could be quick wins\n\n\
                Use issues/query to filter and analyze tasks.",
                context_summary["total_open"],
                context_summary["in_progress"],
                context_summary["blocked"],
                context_summary["critical"]
            ),
            "standup" => format!(
                "# Daily Standup Summary\n\n\
                Generate a standup summary covering:\n\n\
                **Yesterday:**\n\
                - Use issues/query with filters to find recently updated issues\n\
                - Summarize completed checkpoints and status changes\n\n\
                **Today:**\n\
                - List in-progress tasks (status=in_progress)\n\
                - Identify next priorities from critical/high items\n\n\
                **Blockers:**\n\
                - List all blocked issues (status=blocked)\n\
                - Note reasons and required actions\n\n\
                Current state: {} open issues, {} in progress, {} blocked",
                context_summary["total_open"],
                context_summary["in_progress"],
                context_summary["blocked"]
            ),
            "weekly-review" => format!(
                "# Weekly Review\n\n\
                Conduct a comprehensive weekly review:\n\n\
                **Completed This Week:**\n\
                - Check closed issues in the last 7 days\n\
                - Highlight major accomplishments\n\n\
                **In Progress:**\n\
                - Review all in-progress tasks\n\
                - Identify tasks at risk of missing deadlines\n\n\
                **Upcoming Priorities:**\n\
                - List critical and high-priority tasks\n\
                - Suggest quick wins for the week ahead\n\n\
                **Metrics:**\n\
                - Total issues opened vs closed\n\
                - Average time to close\n\
                - Backlog growth/reduction\n\n\
                Current state: {} open issues",
                context_summary["total_open"]
            ),
            _ => {
                return Err(McpError {
                    code: ErrorCode(-32602),
                    message: Cow::from(format!("Unknown prompt: {name}")),
                    data: None,
                })
            }
        };

        Ok(GetPromptResult {
            description: None,
            messages: vec![PromptMessage::User {
                content: Content::text(prompt_text),
            }],
        })
    }
}
