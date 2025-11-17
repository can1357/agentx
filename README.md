# AgentX - AI-Native Issue Tracker

Structured task/bug tracking optimized for agentic workflows. A high-performance Rust CLI with MCP server integration, interactive TUI dashboard, dependency management, and velocity metrics.

## Features

### Core Tracking
- **Structured issues**: Track concrete, actionable work items with metadata
- **Priority management**: Critical, high, medium, low priorities
- **Status tracking**: open, active, blocked, done, closed, backlog
- **Effort estimation**: Tag issues with time estimates (30m, 2h, 1d, etc.)
- **MDX storage**: Issues stored as MDX files with human-readable YAML frontmatter (RFC3339 timestamps)

### Advanced Features
- **MCP Server**: First-class AI agent integration via Model Context Protocol
- **Dependency Management**: Track issue dependencies with cycle detection
- **Interactive TUI**: Real-time dashboard with kanban board, dependency graph, and velocity metrics
- **Git Integration**: Auto-create branches, track commits, detect stale work
- **Metrics Tracking**: Velocity charts (pts/day), burndown tracking, completion stats
- **Backlog Management**: Defer/activate issues to manage scope
- **Aliasing**: Use memorable names instead of bug numbers
- **Context Commands**: Quick overview of active work, blockers, priorities
- **Bulk Operations**: Start or close multiple issues at once
- **Auto-status Detection**: Checkpoint messages can auto-update issue status
- **Shell Completion**: Bash, zsh, fish support

## Installation

```bash
cargo build --release
cargo install --path .

# Generate shell completions
agentx completion bash > /etc/bash_completion.d/agentx
agentx completion zsh > /usr/local/share/zsh/site-functions/_agentx
agentx completion fish > ~/.config/fish/completions/agentx.fish
```

## Quick Start

```bash
# See agent usage guide
agentx guide

# Launch interactive TUI dashboard
agentx tui

# Create an issue with dependencies
agentx new \
  --title "Fix authentication bug" \
  --priority high \
  --effort "2h" \
  --file src/auth.rs \
  --depends-on 5 \
  --issue "Users cannot login after password reset" \
  --impact "Blocks user re-authentication flow" \
  --acceptance "Users can login successfully after password reset"

# Check your current work context
agentx context

# Start working (auto-creates git branch if configured)
agentx start 1

# Add progress notes (supports auto-status detection)
agentx checkpoint 1 "Implemented password reset flow"
agentx checkpoint 1 "BLOCKED: Waiting for security review"  # Auto-blocks issue
agentx checkpoint 1 "FIXED: All tests passing"  # Auto-marks as done

# Close when done
agentx close 1 -m "Fixed by updating auth token generation"
```

## MCP Server

AgentX includes a built-in MCP server for AI agent integration:

```bash
# Start MCP server (stdio transport)
agentx serve
```

### Available MCP Tools

- `issues/context` - Get current work context (active, blocked, high-priority tasks)
- `issues/create` - Create new issues with full metadata
- `issues/update_status` - Change issue status (start, block, done, close, reopen)
- `issues/show` - Show full issue details in MDX format
- `issues/query` - Advanced filtering by status, priority, effort, files
- `issues/checkpoint` - Add timestamped progress notes
- `issues/dependencies` - Manage issue dependencies with cycle detection

### Claude Desktop Configuration

Add to `claude_desktop_config.json`:

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

See [MCP_SERVER.md](./MCP_SERVER.md) for detailed documentation.

## Interactive TUI

Launch the terminal dashboard:

```bash
agentx tui
```

Features:
- **Kanban board**: Backlog, Ready, Active, Blocked, Done columns
- **Dependency graph**: Visual tree of issue dependencies
- **Metrics panel**: Real-time velocity (pts/day), burndown tracking
- **Live updates**: Reflects changes as you work

## Commands Reference

### Issue Management

```bash
# Create issues
agentx new --title "..." --priority high --effort 2h --issue "..." --impact "..." --acceptance "..."

# List and view
agentx list                    # List open issues
agentx list --status closed    # List closed issues
agentx show 1                  # Show full details
agentx show --json 1          # JSON output

# Update status
agentx start 1                 # Mark as active
agentx block 1 --reason "..."  # Mark as blocked
agentx close 1 -m "..."        # Close with message
agentx open 1                  # Reopen closed issue
agentx checkpoint 1 "note"     # Add progress note

# Bulk operations
agentx bulk-start 1 2 3
agentx bulk-close 1 2 3 -m "Completed during sprint"
```

### Dependencies

```bash
# Add dependency (1 depends on 5)
agentx depends add 1 5

# Remove dependency
agentx depends remove 1 5

# Show dependency tree
agentx depends tree 1

# List dependents (what depends on this?)
agentx depends list 1

# Validate all dependencies (check for cycles)
agentx depends validate
```

### Backlog Management

```bash
# Move to backlog
agentx defer 1

# Activate from backlog
agentx activate 1
```

### Context & Focus

```bash
agentx context        # Current work context
agentx focus          # Top priority tasks
agentx blocked        # All blocked tasks
agentx ready          # Tasks ready to start
agentx quick-wins     # Low-effort tasks (≤1h)
agentx quick-wins --threshold 30m
```

### Metrics & Reporting

```bash
agentx metrics        # Show velocity, burndown, completion stats
agentx summary        # Recent activity (last 24h)
agentx summary --hours 8
```

### Aliases

```bash
agentx alias add 1 auth-bug
agentx alias list
agentx alias remove auth-bug

# Use aliases anywhere
agentx show auth-bug
agentx start auth-bug
```

### Configuration

```bash
# Show current config
agentx config show

# Edit config file
agentx config edit

# Set values
agentx config set git.auto_branch true
agentx config set git.branch_prefix "feature/"
```

Config file location: `.agentx.toml`

Example:
```toml
[git]
auto_branch = true
branch_prefix = "bug/"
auto_commit = false

[metrics]
velocity_window_days = 14
track_time = true
```

### Import/Export

```bash
# Import from YAML
agentx import --file issues.yaml

# Export active issues
agentx list --json > export.json
```

## File Structure

```
.agentx.toml            # Configuration
issues/
├── open/
│   ├── 01-fix-auth-bug.mdx
│   └── 02-add-validation.mdx
├── closed/
│   └── 00-initial-setup.mdx
└── .aliases.yaml
```

## Issue Format

```mdx
---
id: 1
title: Fix authentication bug
priority: high
status: active
created: 2025-11-17T09:44:00.448771488+00:00
files:
  - src/auth.rs
effort: 2h
started: 2025-11-17T09:45:12.123456789+00:00
depends_on: [5]
---

# BUG-1: Fix authentication bug

**Issue**: Users cannot login after password reset

**Impact**: Blocks user re-authentication flow

**Acceptance**: Users can login successfully after password reset

**Checkpoint** (2025-11-17 09:47): Completed initial investigation
**Checkpoint** (2025-11-17 10:15): Implemented password reset flow
```

## Status Values

- `open` - Not started yet (was `not_started`)
- `active` - Currently being worked on (was `in_progress`)
- `blocked` - Blocked by external dependency
- `done` - Work completed, needs verification
- `closed` - Verified and archived
- `backlog` - Deferred for later

## Auto-Status Detection

Checkpoint messages automatically update issue status:

- `BLOCKED: <reason>` → Sets status to blocked with reason
- `FIXED: <note>` → Sets status to done
- `DONE: <note>` → Sets status to done

```bash
agentx checkpoint 1 "BLOCKED: Waiting for security review"
# Output: ✓ Added checkpoint to BUG-1
#         Status updated to: blocked
```

## Effort Estimation

Supported formats:
- Short: `30m`, `2h`, `1d`
- Decimal: `1.5h`, `0.5d`
- Long: `2 hours`, `30 minutes`, `1 day`

Used by:
- `quick-wins` command
- Velocity metrics (converts to story points)
- Burndown tracking

## Performance Benefits

- **Fast startup**: No interpreter overhead (~2ms)
- **Compiled regexes**: Pattern matching compiled once
- **Efficient I/O**: Zero-copy operations
- **Parallel potential**: Rust's zero-cost concurrency
- **Small binary**: Single ~3MB executable

Perfect for agent workflows calling the CLI hundreds of times per session.

## Workflow Examples

### Starting a Session

```bash
# Quick context
agentx context

# Or launch TUI dashboard
agentx tui

# See top priorities
agentx focus

# Find quick tasks
agentx quick-wins
```

### Working on Issues

```bash
# Start (auto-creates git branch if configured)
agentx start 1

# Add progress notes
agentx checkpoint 1 "Implemented password reset flow"

# Block if needed
agentx checkpoint 1 "BLOCKED: Waiting for security review"
```

### Tracking Progress

```bash
# View metrics
agentx metrics

# See recent activity
agentx summary

# Check dependency graph
agentx depends tree
```

### Managing Dependencies

```bash
# Issue 10 depends on 5 and 7
agentx depends add 10 5
agentx depends add 10 7

# Show what 10 depends on
agentx depends tree 10

# Show what depends on 5
agentx depends list 5

# Validate no cycles
agentx depends validate
```

## Best Practices for AI Agents

1. **Session start**: Run `agentx context` to load work state
2. **Before working**: Run `agentx show <num>` for full context
3. **During work**: Use `agentx checkpoint` with descriptive notes
4. **Status changes**: Use prefixes (`BLOCKED:`, `FIXED:`) for auto-status
5. **Dependencies**: Track blockers with `depends add`
6. **Quick wins**: Use `agentx quick-wins` for small tasks
7. **Bulk updates**: Use `bulk-start`/`bulk-close` for batches
8. **MCP integration**: Use MCP tools for structured access

## Architecture

- **CLI**: `clap` for argument parsing
- **Storage**: File-based MDX with YAML frontmatter
- **Serialization**: `serde` + `serde_yaml` + RFC3339 timestamps
- **MCP Server**: `rmcp` SDK with `tokio` async runtime
- **TUI**: `ratatui` for terminal UI
- **Git**: `git2` for repository operations
- **Metrics**: In-memory aggregation with file-based history

## Development

```bash
# Build
cargo build --release

# Run tests
cargo test

# Format
cargo fmt

# Lint
cargo clippy

# Run locally
cargo run -- <command>
```

## License

MIT
