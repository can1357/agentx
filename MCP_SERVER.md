# MCP Server for AgentX Issue Tracker

The AgentX issue tracker now includes an MCP (Model Context Protocol) server that exposes issue management functionality as a first-class tool for AI agents like Claude.

## Starting the Server

```bash
agentx serve
```

The server runs on stdio by default, making it compatible with MCP clients like Claude Desktop.

## Available Tools

### `issues/context`
Get current work context showing in-progress, blocked, and high-priority tasks.

**Parameters:**
- `format` (optional): Output format - 'summary', 'detailed', or 'json' (default: 'summary')

**Returns:** JSON with categorized issues

### `issues/create`
Create a new issue/task.

**Parameters:**
- `title` (required): Issue title
- `priority` (optional): 'critical', 'high', 'medium', or 'low' (default: 'medium')
- `files` (optional): Array of related file paths
- `issue` (required): Description of the problem
- `impact` (required): Impact description
- `acceptance` (required): Acceptance criteria
- `effort` (optional): Effort estimate (e.g., '30m', '2h', '1d')
- `context` (optional): Additional context

**Returns:** Bug number and confirmation message

### `issues/update_status`
Change issue status.

**Parameters:**
- `bug_ref` (required): Bug number or alias
- `status` (required): 'start', 'block', 'done', 'close', or 'reopen'
- `reason` (optional): Required for 'block', optional for 'close'

**Returns:** Success confirmation

### `issues/show`
Show full issue details in MDX format.

**Parameters:**
- `bug_ref` (required): Bug number or alias

**Returns:** Complete issue content

## Configuration for Claude Desktop

Add to your `claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "agentx": {
      "command": "/path/to/agentx",
      "args": ["serve"]
    }
  }
}
```

## Architecture

The MCP server is built using the official `rmcp` (Rust MCP) SDK:
- **Tools**: Issue management operations exposed via `#[tool]` macro
- **Type Safety**: Request/response schemas validated with `schemars`
- **Async**: Built on Tokio for efficient I/O
- **stdio Transport**: Standard input/output for MCP protocol communication

## Implementation Details

- **Source**: `src/mcp/mod.rs`
- **Dependencies**:
  - `rmcp = { version = "0.8", features = ["server", "transport-io"] }`
  - `tokio = { version = "1", features = ["full"] }`
  - `schemars = "1.0"`

The server wraps the existing `Commands` and `Storage` layers, providing a clean MCP interface without duplicating business logic.

## Benefits for Agents

1. **Structured Interface**: No parsing stdout - get JSON directly
2. **Type Safety**: Schema-validated requests
3. **Stateful**: MCP maintains connection
4. **Discoverable**: Tools auto-documented via MCP protocol
5. **Native Integration**: Works with any MCP client
