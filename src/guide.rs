pub const AGENT_USAGE_GUIDE: &str = r#"
=== QUICK START ===

Most Common Commands:
1. `agentx context`              - See what's happening now
2. `agentx list --status open`   - View all active tasks
3. `agentx new --title "..." --priority high --issue "..." --impact "..." --acceptance "..."`
4. `agentx start <num>`          - Begin working on a task
5. `agentx checkpoint <num> "progress note"` - Log progress
6. `agentx close <num> -m "done"` - Complete a task
7. `agentx defer <num>`          - Move task to backlog
8. `agentx activate <num>`       - Bring backlog task back

MCP Tools (for AI agents):
- issues_context   - Get current work context
- issues_create    - Create new issue
- issues_search    - Full-text search across all issues
- issues_show      - Get full issue details
- issues_checkpoint - Add progress notes
- issues_query     - Advanced filtering
- issues_wins      - Find quick-win tasks

=== DETAILED GUIDE ===

PURPOSE: Structured task/bug tracking optimized for agentic workflows. Use this to:
- Track concrete, actionable work items
- Maintain state across sessions
- Organize work by severity/priority
- Document decisions and progress
- Defer non-urgent work to backlog

WHEN TO CREATE ISSUES:
- Any bug requiring >1 fix or multi-step resolution
- Tasks that span multiple files/modules
- Technical debt that needs tracking
- Unclear problems that need investigation
→ Do NOT create issues for trivial fixes you can do immediately

WORKFLOW PATTERN:
1. Start session: `agentx context` to see current state
2. Before fixing: `agentx show <num>` to understand context
3. Start work: `agentx start <num>` to mark in-progress
4. During work: `agentx checkpoint <num> <update>` to document progress
5. After fix: `agentx close <num> -m "resolution details"`
6. Defer later: `agentx defer <num>` to move to backlog

BACKLOG MANAGEMENT:
- Use `defer <num>` to postpone non-urgent tasks (marked with 💤)
- Use `activate <num>` to bring backlog items back to active state
- Backlog items appear dimmed at the end of list output
- Perfect for tracking ideas, future improvements, or lower-priority work

ISSUE QUALITY CHECKLIST:
- Title: Specific, action-oriented (not vague like "fix bug")
- Files: All impacted paths listed for quick navigation
- Issue: What's broken? Observable symptoms.
- Impact: Why does this matter? What fails/breaks?
- Acceptance: Clear, testable completion criteria

SEARCH/FILTER STRATEGY:
- Use `context` for quick overview of current work
- Use `focus` to see top priorities
- Use `ready` to find actionable tasks
- Use `blocked` to see what's waiting
- Use MCP tool `issues_search` for full-text search across title/content
- Check both open AND closed for historical context

INTEGRATION NOTES:
- Issues live in issues/{open,closed}/ as MDX with YAML frontmatter
- Bug numbers are stable across open/close cycles
- Use --json flag for programmatic access
- Use aliases for semantic references (e.g., "msg-handler" instead of "21")
- Colored output can be toggled in .agentxrc.yaml (colored_output: true/false)

ADVANCED FEATURES:
- Dependencies: Track which issues block others
- Effort estimation: Mark issues with time estimates for quick wins
- Bulk operations: Start/close multiple issues at once
- Session summaries: See what changed in your last work session
- Critical path: Find longest dependency chain
"#;

pub fn print_guide() {
   println!("{}", AGENT_USAGE_GUIDE);
}
