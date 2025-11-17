pub const AGENT_USAGE_GUIDE: &str = r#"
=== AGENT USAGE GUIDE ===

PURPOSE: Structured task/bug tracking optimized for agentic workflows. Use this to:
- Track concrete, actionable work items
- Maintain state across sessions
- Organize work by severity/priority
- Document decisions and progress

WHEN TO CREATE ISSUES:
- Any bug requiring >1 fix or multi-step resolution
- Tasks that span multiple files/modules
- Technical debt that needs tracking
- Unclear problems that need investigation
→ Do NOT create issues for trivial fixes you can do immediately

WORKFLOW PATTERN:
1. Start session: `issues context` to see current state
2. Before fixing: `issues show <num>` to understand context
3. Start work: `issues start <num>` to mark in-progress
4. During work: `issues checkpoint <num> <update>` to document progress
5. After fix: `issues close <num> -m "resolution details"`

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
- Check both open AND closed for historical context

INTEGRATION NOTES:
- Issues live in issues/{open,closed}/ as MDX with YAML frontmatter
- Bug numbers are stable across open/close cycles
- Use --json flag for programmatic access
- Use aliases for semantic references (e.g., "msg-handler" instead of "21")

ADVANCED FEATURES:
- Dependencies: Track which issues block others
- Effort estimation: Mark issues with time estimates for quick wins
- Bulk operations: Start/close multiple issues at once
- Session summaries: See what changed in your last work session
"#;

pub fn print_guide() {
    println!("{}", AGENT_USAGE_GUIDE);
}
