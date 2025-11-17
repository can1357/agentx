# Issues - Rust Issue Tracker CLI

Structured task/bug tracking optimized for agentic workflows. A Rust port of the Python issue tracker with significant performance and usability enhancements.

## Features

- **Structured tracking**: Track concrete, actionable work items with metadata
- **Priority management**: Critical, high, medium, low priorities
- **Status tracking**: Not started, in progress, blocked, done, closed
- **Effort estimation**: Tag issues with time estimates (30m, 2h, 1d, etc.)
- **Dependencies**: Track which issues block others
- **Aliasing**: Use memorable names instead of bug numbers
- **JSON output**: Machine-readable output for all commands
- **MDX storage**: Issues stored as MDX files with human-readable YAML frontmatter (RFC3339 timestamps)
- **Context awareness**: Quick overview commands for workflow orientation
- **Bulk operations**: Start or close multiple issues at once
- **Auto-status detection**: Checkpoint messages can auto-update issue status
- **Quick wins**: Find low-effort tasks for quick productivity

## Installation

```bash
cargo build --release
cargo install --path .
```

## Quick Start

```bash
# See agent usage guide
issues guide

# Create an issue with effort estimate
issues new \
  --title "Fix authentication bug" \
  --priority high \
  --effort "2h" \
  --file src/auth.rs \
  --issue "Users cannot login after password reset" \
  --impact "Blocks user re-authentication flow" \
  --acceptance "Users can login successfully after password reset"

# Check your current work context
issues context

# Find quick wins (tasks ≤ 1 hour)
issues quick-wins

# Start working
issues start 1

# Add progress notes (supports auto-status detection)
issues checkpoint 1 "Implemented password reset flow"
issues checkpoint 1 "BLOCKED: Waiting for security review"  # Auto-blocks issue
issues checkpoint 1 "FIXED: All tests passing"  # Auto-marks as done

# Close when done
issues close 1 -m "Fixed by updating auth token generation"
```

## Usage

### Creating Issues

```bash
# Basic issue
issues new \
  --title "Fix authentication bug" \
  --priority high \
  --file src/auth.rs \
  --issue "Users cannot login after password reset" \
  --impact "Blocks user re-authentication flow" \
  --acceptance "Users can login successfully after password reset"

# With effort estimate and context
issues new \
  --title "Add logging" \
  --priority low \
  --effort "30m" \
  --context "Part of observability initiative" \
  --issue "Need better debug logs" \
  --impact "Hard to debug production issues" \
  --acceptance "All critical paths logged"
```

### Listing and Viewing Issues

```bash
# List open issues
issues list

# List closed issues
issues list --status closed

# Verbose output with file paths
issues list -v

# Show issue details
issues show 1
issues show <bug-number>

# JSON output for any command
issues list --json
issues show 1 --json
```

### Managing Issue Status

```bash
# Start working on an issue
issues start 1

# Block an issue
issues block 1 --reason "Waiting for API design"

# Add a checkpoint (auto-detects status changes)
issues checkpoint 1 "Completed initial implementation"
issues checkpoint 1 "BLOCKED: Need code review"  # Auto-updates status
issues checkpoint 1 "FIXED: Tests passing"       # Auto-marks as done

# Close an issue
issues close 1 -m "Fixed by updating auth flow"

# Reopen a closed issue
issues open 1
```

### Bulk Operations

```bash
# Start multiple issues at once
issues bulk-start 1 2 3

# Close multiple issues
issues bulk-close 1 2 3 -m "Completed during sprint"
```

### Finding Work

```bash
# Show current work context
issues context

# Show top priority tasks
issues focus

# Show blocked tasks
issues blocked

# Show tasks ready to start
issues ready

# Find quick wins (low-effort tasks)
issues quick-wins                    # Default: ≤ 1 hour
issues quick-wins --threshold 30m    # ≤ 30 minutes
issues quick-wins --threshold 2h     # ≤ 2 hours
```

### Session Summaries

```bash
# See what changed in last 24 hours
issues summary

# Custom timeframe
issues summary --hours 8   # Last 8 hours
```

### Alias Management

```bash
# Add an alias
issues alias add 1 auth-bug

# Use alias instead of number
issues show auth-bug
issues start auth-bug
issues checkpoint auth-bug "Making progress"

# List all aliases
issues alias list

# Remove an alias
issues alias remove auth-bug
```

### Importing Issues

Create a YAML file:

```yaml
# issues.yaml
- title: Implement user registration
  priority: high
  effort: "4h"
  files:
    - src/user.rs
    - src/handlers/register.rs
  issue: Need user registration endpoint
  impact: Blocks user onboarding
  acceptance: Users can register with email/password

- title: Add input validation
  priority: medium
  effort: "1h"
  issue: Form inputs lack validation
  impact: Security risk
  acceptance: All inputs validated
```

Import:

```bash
issues import --file issues.yaml
```

## File Structure

Issues are stored in the `issues/` directory:

```
issues/
├── open/
│   ├── 01-fix-auth-bug.mdx
│   └── 02-add-validation.mdx
├── closed/
│   └── 00-initial-setup.mdx
└── .aliases.yaml
```

### Issue Format (MDX with RFC3339 Timestamps)

```mdx
---
id: 1
title: Fix authentication bug
priority: high
status: in_progress
created: 2025-11-17T09:44:00.448771488+00:00
files:
  - src/auth.rs
effort: 2h
started: 2025-11-17T09:45:12.123456789+00:00
---

# BUG-1: Fix authentication bug

**Issue**: Users cannot login after password reset

**Impact**: Blocks user re-authentication flow

**Acceptance**: Users can login successfully after password reset

**Checkpoint** (2025-11-17 09:47): Completed initial investigation
**Checkpoint** (2025-11-17 10:15): Implemented password reset flow
```

## Effort Estimation

Support for multiple time units:

- `30m`, `2h`, `1d` (minutes, hours, days)
- `1.5h`, `0.5d` (decimal values supported)
- Full words: `2 hours`, `30 minutes`, `1 day`

Used by `quick-wins` command to find low-effort tasks.

## Auto-Status Detection

Checkpoint messages can automatically update issue status:

- `BLOCKED: <reason>` → Sets status to blocked with reason
- `FIXED: <note>` → Sets status to done
- `DONE: <note>` → Sets status to done

Example:

```bash
issues checkpoint 1 "BLOCKED: Waiting for security review"
# Output: ✓ Added checkpoint to BUG-1
#         Status updated to: blocked
```

## Commands Reference

| Command | Description |
|---------|-------------|
| `guide` | Show comprehensive agent usage guide |
| `new` | Create a new issue |
| `list` | List issues (open/closed) |
| `show` | Show full issue details |
| `start` | Mark issue as in-progress |
| `block` | Mark issue as blocked |
| `close` | Close an issue |
| `open` | Reopen a closed issue |
| `checkpoint` | Add timestamped progress note (supports auto-status) |
| `context` | Show current work context |
| `focus` | Show top priority tasks |
| `blocked` | Show all blocked tasks |
| `ready` | Show tasks ready to start |
| `quick-wins` | Find low-effort tasks |
| `bulk-start` | Start multiple issues at once |
| `bulk-close` | Close multiple issues at once |
| `summary` | Show session summary (recent activity) |
| `import` | Import issues from YAML |
| `alias` | Manage bug number aliases |

## Performance Benefits

The Rust implementation offers significant performance advantages over Python:

- **Fast startup**: No Python interpreter overhead
- **Compiled regexes**: Pattern matching compiled once, reused across calls
- **Efficient I/O**: Zero-copy file operations where possible
- **Parallel potential**: Built on Rust's zero-cost concurrency primitives
- **Small binary**: Single ~2MB executable, no dependencies

Perfect for agent workflows where the CLI might be called hundreds of times per session.

## Workflow Guide

### Starting a session

```bash
# Quick context overview
issues context

# See top priorities
issues focus

# Find quick tasks
issues quick-wins
```

### Working on issues

```bash
# Start an issue
issues start 1

# Add progress notes
issues checkpoint 1 "Implemented password reset flow"

# Block if needed
issues checkpoint 1 "BLOCKED: Waiting for security review"  # Auto-blocks
```

### Completing work

```bash
# Mark as done
issues checkpoint 1 "FIXED: All tests passing"  # Auto-marks done

# Close when verified
issues close 1 -m "Fixed by updating auth token generation"

# See what you accomplished
issues summary
```

### Bulk workflows

```bash
# Start multiple related tasks
issues bulk-start 5 6 7

# Complete a batch of fixes
issues bulk-close 5 6 7 -m "Completed during bugfix sprint"
```

## Best Practices for Agents

1. **Session start**: Run `issues context` or `issues focus`
2. **Before working**: Run `issues show <num>` to load context
3. **During work**: Use `issues checkpoint` frequently with descriptive notes
4. **Status changes**: Use prefixes (`BLOCKED:`, `FIXED:`) for auto-status
5. **After work**: Run `issues summary` to track progress
6. **Quick wins**: Use `issues quick-wins` when looking for small tasks
7. **Bulk updates**: Use `bulk-start`/`bulk-close` for batch operations

## Human-Readable Storage

Unlike the Python version's Unix timestamps, all timestamps are stored in RFC3339 format:

```yaml
created: 2025-11-17T09:44:00.448771488+00:00  # Instead of: 1763372178
started: 2025-11-17T09:45:12.123456789+00:00
```

This makes issues readable and greppable without conversion tools.

## License

MIT
