# Issues - Rust Issue Tracker CLI

Structured task/bug tracking optimized for agentic workflows.

## Features

- **Structured tracking**: Track concrete, actionable work items with metadata
- **Priority management**: Critical, high, medium, low priorities
- **Status tracking**: Not started, in progress, blocked, done, closed
- **Aliasing**: Use memorable names instead of bug numbers
- **JSON output**: Machine-readable output for all commands
- **MDX storage**: Issues stored as MDX files with YAML frontmatter
- **Context awareness**: Quick overview commands for workflow orientation

## Installation

```bash
cargo build --release
cargo install --path .
```

## Usage

### Create a new issue

```bash
issues new \
  --title "Fix authentication bug" \
  --priority high \
  --file src/auth.rs \
  --issue "Users cannot login after password reset" \
  --impact "Blocks user re-authentication flow" \
  --acceptance "Users can login successfully after password reset"
```

### List issues

```bash
# List open issues
issues list

# List closed issues
issues list --status closed

# Verbose output with file paths
issues list -v
```

### Show issue details

```bash
issues show 1
issues show <bug-number>
```

### Update issue status

```bash
# Start working on an issue
issues start 1

# Block an issue
issues block 1 --reason "Waiting for API design"

# Add a checkpoint
issues checkpoint 1 "Completed initial implementation"

# Close an issue
issues close 1 -m "Fixed by updating auth flow"

# Reopen a closed issue
issues open 1
```

### Context commands

```bash
# Show current work context
issues context

# Show top priority tasks
issues focus

# Show blocked tasks
issues blocked

# Show tasks ready to start
issues ready
```

### Alias management

```bash
# Add an alias
issues alias add 1 auth-bug

# Use alias instead of number
issues show auth-bug

# List all aliases
issues alias list

# Remove an alias
issues alias remove auth-bug
```

### Import issues from YAML

```yaml
# issues.yaml
- title: Implement user registration
  priority: high
  files:
    - src/user.rs
    - src/handlers/register.rs
  issue: Need user registration endpoint
  impact: Blocks user onboarding
  acceptance: Users can register with email/password

- title: Add input validation
  priority: medium
  issue: Form inputs lack validation
  impact: Security risk
  acceptance: All inputs validated
```

```bash
issues import --file issues.yaml
```

### JSON output

All commands support `--json` flag for machine-readable output:

```bash
issues list --json
issues show 1 --json
issues context --json
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

### Issue Format (MDX)

```mdx
---
id: 1
title: Fix authentication bug
priority: high
status: in_progress
created: 1763372178
files:
  - src/auth.rs
started: 1763372221
---

# BUG-1: Fix authentication bug

**Issue**: Users cannot login after password reset

**Impact**: Blocks user re-authentication flow

**Acceptance**: Users can login successfully after password reset

**Checkpoint** (2025-11-17 09:37): Completed initial investigation
```

## Workflow Guide

### Starting a session

```bash
# Quick context overview
issues context

# See top priorities
issues focus
```

### Working on issues

```bash
# Start an issue
issues start 1

# Add progress notes
issues checkpoint 1 "Implemented password reset flow"

# Block if needed
issues block 1 --reason "Waiting for security review"
```

### Completing work

```bash
# Close when done
issues close 1 -m "Fixed by updating auth token generation"

# Verify closed
issues list --status closed
```

## Commands Reference

| Command | Description |
|---------|-------------|
| `new` | Create a new issue |
| `list` | List issues (open/closed) |
| `show` | Show full issue details |
| `start` | Mark issue as in-progress |
| `block` | Mark issue as blocked |
| `close` | Close an issue |
| `open` | Reopen a closed issue |
| `checkpoint` | Add timestamped progress note |
| `context` | Show current work context |
| `focus` | Show top priority tasks |
| `blocked` | Show all blocked tasks |
| `ready` | Show tasks ready to start |
| `import` | Import issues from YAML |
| `alias` | Manage bug number aliases |

## License

MIT
