use anyhow::Result;
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::{
   commands::Commands, config::Config, fuzzy::filter_by_tags, issue::IssueWithId, storage::Storage,
   utils::parse_effort,
};

pub struct SimpleMcpServer {
   commands: Commands,
}

impl Default for SimpleMcpServer {
   fn default() -> Self {
      Self::new()
   }
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
                              "type": "number",
                              "description": "Bug reference number"
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
                              "type": "number",
                              "description": "Bug reference number"
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
              },
              {
                  "name": "issues_checkpoint",
                  "description": "Add a progress checkpoint note to an issue",
                  "inputSchema": {
                      "type": "object",
                      "properties": {
                          "bug_ref": {
                              "type": "number",
                              "description": "Bug reference number"
                          },
                          "note": {
                              "type": "string",
                              "description": "Progress note (prefix with BLOCKED: or DONE: to auto-update status)"
                          }
                      },
                      "required": ["bug_ref", "note"]
                  }
              },
              {
                  "name": "issues_search",
                  "description": "Full-text search across issue titles and bodies",
                  "inputSchema": {
                      "type": "object",
                      "properties": {
                          "query": {
                              "type": "string",
                              "description": "Search query (case-insensitive)"
                          },
                          "status": {
                              "type": "string",
                              "description": "Filter by status: 'open', 'closed', or 'all' (default: 'open')"
                          }
                      },
                      "required": ["query"]
                  }
              },
              {
                  "name": "issues_query",
                  "description": "Advanced query with filters for tags, priority, and status",
                  "inputSchema": {
                      "type": "object",
                      "properties": {
                          "tags": {
                              "type": "array",
                              "items": { "type": "string" },
                              "description": "Filter by tags (fuzzy match, AND logic)"
                          },
                          "priority": {
                              "type": "string",
                              "description": "Filter by priority level",
                              "enum": ["critical", "high", "medium", "low"]
                          },
                          "status": {
                              "type": "string",
                              "description": "Filter by status",
                              "enum": ["open", "in_progress", "blocked", "backlog", "closed"]
                          }
                      }
                  }
              },
              {
                  "name": "issues_wins",
                  "description": "Find quick-win tasks based on effort estimate",
                  "inputSchema": {
                      "type": "object",
                      "properties": {
                          "threshold": {
                              "type": "string",
                              "description": "Maximum effort threshold (e.g., '30m', '1h', '2h'). Default: '1h'"
                          }
                      }
                  }
              }
          ]
      })
   }

   async fn handle_tool_call(&self, params: &Value) -> Value {
      let name = params["name"].as_str().unwrap_or("");
      let arguments = &params["arguments"];

      let result = match name {
         "issues_list" => {
            let status = arguments["status"].as_str().unwrap_or("open");
            self.commands.list_data(status).map(|r| serde_json::to_value(r).unwrap_or_else(|_| json!({"error": "serialization failed"})))
         },
         "issues_context" => {
            self.commands.context_data().map(|r| serde_json::to_value(r).unwrap_or_else(|_| json!({"error": "serialization failed"})))
         },
         "issues_create" => {
            let title = arguments["title"].as_str().unwrap_or("");
            let issue = arguments["issue"].as_str().unwrap_or("");
            let impact = arguments["impact"].as_str().unwrap_or("");
            let acceptance = arguments["acceptance"].as_str().unwrap_or("");
            let priority = arguments["priority"].as_str().unwrap_or("medium");

            self.commands.create_issue_data(
               title.to_string(),
               priority,
               vec![],
               vec![],
               issue.to_string(),
               impact.to_string(),
               acceptance.to_string(),
               None,
               None,
            ).map(|r| serde_json::to_value(r).unwrap_or_else(|_| json!({"error": "serialization failed"})))
         },
         "issues_show" => {
            let bug_ref = arguments["bug_ref"].as_u64().map(|n| n.to_string()).unwrap_or_default();
            self.commands.show_data(&bug_ref).map(|r| serde_json::to_value(r).unwrap_or_else(|_| json!({"error": "serialization failed"})))
         },
         "issues_status" => {
            let bug_ref = arguments["bug_ref"].as_u64().map(|n| n.to_string()).unwrap_or_default();
            let status = arguments["status"].as_str().unwrap_or("");
            let reason = arguments["reason"].as_str().map(|s| s.to_string());

            let data_result = match status {
               "start" => self.commands.start_data(&bug_ref),
               "block" => self.commands.block_data(&bug_ref, reason.unwrap_or_default()),
               "done" | "close" => self.commands.close_data(&bug_ref, reason),
               "reopen" => self.commands.open_data(&bug_ref),
               "defer" => self.commands.defer_data(&bug_ref),
               "activate" => self.commands.activate_data(&bug_ref),
               _ => Err(anyhow::anyhow!("Unknown status: {}", status)),
            };

            data_result.map(|r| serde_json::to_value(r).unwrap_or_else(|_| json!({"error": "serialization failed"})))
         },
         "issues_checkpoint" => {
            let bug_ref = arguments["bug_ref"].as_u64().map(|n| n.to_string()).unwrap_or_default();
            let note = arguments["note"].as_str().unwrap_or("");
            self.commands.checkpoint_data(&bug_ref, note.to_string()).map(|r| serde_json::to_value(r).unwrap_or_else(|_| json!({"error": "serialization failed"})))
         },
         "issues_search" => {
            let query = arguments["query"].as_str().unwrap_or("");
            let status = arguments["status"].as_str().unwrap_or("open");
            Ok(json!({"result": self.search_issues(query, status)}))
         },
         "issues_query" => {
            let tags: Vec<String> = arguments["tags"]
               .as_array()
               .map(|arr| {
                  arr.iter()
                     .filter_map(|v| v.as_str().map(String::from))
                     .collect()
               })
               .unwrap_or_default();
            let priority = arguments["priority"].as_str();
            let status = arguments["status"].as_str();
            Ok(json!({"result": self.query_issues(&tags, priority, status)}))
         },
         "issues_wins" => {
            let threshold = arguments["threshold"].as_str().unwrap_or("1h");
            Ok(json!({"result": self.find_quick_wins(threshold)}))
         },
         _ => Err(anyhow::anyhow!("Unknown tool: {}", name)),
      };

      match result {
         Ok(data) => json!({
            "content": [{
               "type": "text",
               "text": serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".to_string())
            }]
         }),
         Err(e) => json!({
            "content": [{
               "type": "text",
               "text": format!("Error: {}", e)
            }],
            "isError": true
         }),
      }
   }

   fn search_issues(&self, query: &str, status_filter: &str) -> String {
      let query_lower = query.to_lowercase();
      let config = Config::load();
      let issues_dir = config.resolve_issues_directory();
      let storage = Storage::new(issues_dir);

      let issues: Vec<IssueWithId> = match status_filter {
         "open" => storage.list_open_issues().unwrap_or_default(),
         "closed" => storage.list_closed_issues().unwrap_or_default(),
         "all" => {
            let mut all = storage.list_open_issues().unwrap_or_default();
            all.extend(storage.list_closed_issues().unwrap_or_default());
            all
         },
         _ => storage.list_open_issues().unwrap_or_default(),
      };

      let matches: Vec<_> = issues
         .into_iter()
         .filter(|issue| {
            issue
               .issue
               .metadata
               .title
               .to_lowercase()
               .contains(&query_lower)
               || issue.issue.body.to_lowercase().contains(&query_lower)
         })
         .collect();

      let results: Vec<_> = matches
         .iter()
         .map(|issue| {
            json!({
                "num": issue.id,
                "title": issue.issue.metadata.title,
                "priority": issue.issue.metadata.priority.to_string(),
                "status": issue.issue.metadata.status.to_string(),
            })
         })
         .collect();

      serde_json::to_string_pretty(&json!({
          "query": query,
          "count": results.len(),
          "results": results,
      }))
      .unwrap_or_else(|e| format!("Error: {}", e))
   }

   fn query_issues(&self, tags: &[String], priority: Option<&str>, status: Option<&str>) -> String {
      let config = Config::load();
      let issues_dir = config.resolve_issues_directory();
      let storage = Storage::new(issues_dir);

      let mut issues = storage.list_open_issues().unwrap_or_default();

      if !tags.is_empty() {
         issues = filter_by_tags(issues, tags);
      }

      if let Some(p) = priority {
         issues.retain(|issue| {
            issue.issue.metadata.priority.to_string().to_lowercase() == p.to_lowercase()
         });
      }

      if let Some(s) = status {
         issues.retain(|issue| {
            let status_str = match s {
               "open" => "not_started",
               "in_progress" => "in_progress",
               "blocked" => "blocked",
               "backlog" => "backlog",
               "closed" => "closed",
               _ => s,
            };
            issue.issue.metadata.status.to_string().to_lowercase() == status_str.to_lowercase()
               || issue
                  .issue
                  .metadata
                  .status
                  .to_string()
                  .to_lowercase()
                  .replace('_', " ")
                  == s.to_lowercase().replace('_', " ")
         });
      }

      let results: Vec<_> = issues
         .iter()
         .map(|issue| {
            json!({
                "num": issue.id,
                "title": issue.issue.metadata.title,
                "priority": issue.issue.metadata.priority.to_string(),
                "status": issue.issue.metadata.status.to_string(),
                "tags": issue.issue.metadata.tags,
            })
         })
         .collect();

      serde_json::to_string_pretty(&json!({
          "filters": {
              "tags": tags,
              "priority": priority,
              "status": status,
          },
          "count": results.len(),
          "results": results,
      }))
      .unwrap_or_else(|e| format!("Error: {}", e))
   }

   fn find_quick_wins(&self, threshold: &str) -> String {
      let threshold_minutes = match parse_effort(threshold) {
         Ok(m) => m,
         Err(e) => return format!("Error parsing threshold: {}", e),
      };

      let config = Config::load();
      let issues_dir = config.resolve_issues_directory();
      let storage = Storage::new(issues_dir);

      let issues = storage.list_open_issues().unwrap_or_default();

      let quick: Vec<_> = issues
         .into_iter()
         .filter(|issue| {
            issue
               .issue
               .metadata
               .effort
               .as_ref()
               .and_then(|e| parse_effort(e).ok())
               .map(|m| m <= threshold_minutes)
               .unwrap_or(false)
         })
         .collect();

      let results: Vec<_> = quick
         .iter()
         .map(|issue| {
            json!({
                "num": issue.id,
                "title": issue.issue.metadata.title,
                "priority": issue.issue.metadata.priority.to_string(),
                "effort": issue.issue.metadata.effort,
                "status": issue.issue.metadata.status.to_string(),
            })
         })
         .collect();

      serde_json::to_string_pretty(&json!({
          "threshold": threshold,
          "count": results.len(),
          "results": results,
      }))
      .unwrap_or_else(|e| format!("Error: {}", e))
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
                     if !response.is_null()
                        && let Ok(response_str) = serde_json::to_string(&response)
                     {
                        stdout.write_all(response_str.as_bytes()).await?;
                        stdout.write_all(b"\n").await?;
                        stdout.flush().await?;
                     }
                  },
                  Err(e) => {
                     eprintln!("Failed to parse JSON: {}", e);
                  },
               }
            },
            Err(e) => {
               eprintln!("Error reading stdin: {}", e);
               break;
            },
         }
      }

      Ok(())
   }
}
