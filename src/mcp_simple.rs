use crate::{commands::Commands, config::Config, storage::Storage};
use anyhow::Result;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

pub struct SimpleMcpServer {
    commands: Commands,
}

impl SimpleMcpServer {
    pub fn new() -> Self {
        let config = Config::load();
        let issues_dir = config.resolve_issues_directory();
        let storage = Storage::new(issues_dir);
        let commands = Commands::new(storage);

        Self { commands }
    }

    async fn handle_request(&self, request: Value) -> Value {
        let method = request["method"].as_str().unwrap_or("");
        let params = &request["params"];
        let id = &request["id"];

        if method == "notifications/initialized" {
            return Value::Null;
        }

        let result = match method {
            "initialize" => self.handle_initialize(),
            "tools/list" => self.handle_list_tools(),
            "tools/call" => self.handle_tool_call(params).await,
            _ => json!({
                "error": {
                    "code": -32601,
                    "message": "Method not found"
                }
            }),
        };

        json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        })
    }

    fn handle_initialize(&self) -> Value {
        json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "agentx-mcp",
                "version": "0.1.0"
            },
            "instructions": "Issue tracker MCP server providing tools for managing tasks and bugs. Use issues_context to see current work, issues_create to add tasks, issues_status to update status (start, block, close, defer, activate), issues_checkpoint for progress notes, issues_search for full-text search, issues_query for advanced filtering, and issues_wins to find quick-win tasks. Defer non-urgent tasks to backlog with 'defer' status."
        })
    }

    fn handle_list_tools(&self) -> Value {
        json!({
            "tools": [
                {
                    "name": "issues_list",
                    "description": "List all issues with optional status filter",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "status": {
                                "type": "string",
                                "description": "Filter by status: 'open' or 'closed' (default: 'open')"
                            }
                        }
                    }
                },
                {
                    "name": "issues_context",
                    "description": "Get current work context - in-progress, blocked, and priority tasks",
                    "inputSchema": {
                        "type": "object",
                        "properties": {}
                    }
                },
                {
                    "name": "issues_create",
                    "description": "Create a new issue/task",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "title": {
                                "type": "string",
                                "description": "Issue title"
                            },
                            "issue": {
                                "type": "string",
                                "description": "Description of the issue/problem"
                            },
                            "impact": {
                                "type": "string",
                                "description": "Impact of the issue"
                            },
                            "acceptance": {
                                "type": "string",
                                "description": "Acceptance criteria for completion"
                            },
                            "priority": {
                                "type": "string",
                                "description": "Priority level",
                                "enum": ["critical", "high", "medium", "low"]
                            }
                        },
                        "required": ["title", "issue", "impact", "acceptance"]
                    }
                },
                {
                    "name": "issues_show",
                    "description": "Show full details of a specific issue",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "bug_ref": {
                                "type": "string",
                                "description": "Bug reference (number or alias)"
                            }
                        },
                        "required": ["bug_ref"]
                    }
                },
                {
                    "name": "issues_status",
                    "description": "Update issue status (start, block, done, close, defer, activate)",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "bug_ref": {
                                "type": "string",
                                "description": "Bug reference (number or alias)"
                            },
                            "status": {
                                "type": "string",
                                "description": "Status action to perform",
                                "enum": ["start", "block", "done", "close", "reopen", "defer", "activate"]
                            },
                            "reason": {
                                "type": "string",
                                "description": "Reason (required for 'block', optional for 'close')"
                            }
                        },
                        "required": ["bug_ref", "status"]
                    }
                }
            ]
        })
    }

    async fn handle_tool_call(&self, params: &Value) -> Value {
        let name = params["name"].as_str().unwrap_or("");
        let arguments = &params["arguments"];

        let content = match name {
            "issues_list" => {
                let status = arguments["status"].as_str().unwrap_or("open");
                match self.commands.list(status, false, true) {
                    Ok(_) => format!("Listed {} issues", status),
                    Err(e) => format!("Error: {}", e),
                }
            }
            "issues_context" => match self.commands.context(true) {
                Ok(_) => "Context retrieved".to_string(),
                Err(e) => format!("Error: {}", e),
            },
            "issues_create" => {
                let title = arguments["title"].as_str().unwrap_or("");
                let issue = arguments["issue"].as_str().unwrap_or("");
                let impact = arguments["impact"].as_str().unwrap_or("");
                let acceptance = arguments["acceptance"].as_str().unwrap_or("");
                let priority = arguments["priority"].as_str().unwrap_or("medium");

                match self.commands.create_issue(
                    title.to_string(),
                    priority,
                    vec![],
                    vec![],
                    issue.to_string(),
                    impact.to_string(),
                    acceptance.to_string(),
                    None,
                    None,
                    true,
                ) {
                    Ok(_) => format!("Created issue: {}", title),
                    Err(e) => format!("Error: {}", e),
                }
            }
            "issues_show" => {
                let bug_ref = arguments["bug_ref"].as_str().unwrap_or("");
                match self.commands.show(bug_ref, true) {
                    Ok(_) => format!("Showed issue: {}", bug_ref),
                    Err(e) => format!("Error: {}", e),
                }
            }
            "issues_status" => {
                let bug_ref = arguments["bug_ref"].as_str().unwrap_or("");
                let status = arguments["status"].as_str().unwrap_or("");
                let reason = arguments["reason"].as_str().map(|s| s.to_string());

                match status {
                    "start" => self.commands.start(bug_ref, false, false, true),
                    "block" => self.commands.block(bug_ref, reason.unwrap_or_default(), true),
                    "done" | "close" => self.commands.close(bug_ref, reason, false, false, true),
                    "reopen" => self.commands.open(bug_ref, true),
                    "defer" => self.commands.defer(bug_ref, true),
                    "activate" => self.commands.activate(bug_ref, true),
                    _ => Err(anyhow::anyhow!("Unknown status: {}", status)),
                }
                .map(|_| format!("Updated status to {} for {}", status, bug_ref))
                .unwrap_or_else(|e| format!("Error: {}", e))
            }
            _ => format!("Unknown tool: {}", name),
        };

        json!({
            "content": [{
                "type": "text",
                "text": content
            }]
        })
    }

    pub async fn serve_stdio() -> Result<()> {
        eprintln!("Starting agentx MCP server on stdio...");

        let server = Self::new();

        let stdin = tokio::io::stdin();
        let mut stdin = BufReader::new(stdin);
        let mut stdout = tokio::io::stdout();

        let mut line = String::new();

        loop {
            line.clear();

            match stdin.read_line(&mut line).await {
                Ok(0) => break,
                Ok(_) => {
                    if line.trim().is_empty() {
                        continue;
                    }

                    match serde_json::from_str::<Value>(&line) {
                        Ok(request) => {
                            let response = server.handle_request(request).await;
                            if !response.is_null() {
                                if let Ok(response_str) = serde_json::to_string(&response) {
                                    stdout.write_all(response_str.as_bytes()).await?;
                                    stdout.write_all(b"\n").await?;
                                    stdout.flush().await?;
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("Failed to parse JSON: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error reading stdin: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}
