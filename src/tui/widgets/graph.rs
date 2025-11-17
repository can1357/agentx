use crate::issue::Issue;
use crate::tui::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget},
};
use std::collections::{HashMap, HashSet};

pub struct DependencyGraph<'a> {
    issues: &'a [Issue],
    theme: Theme,
    focus_issue: Option<&'a str>,
}

impl<'a> DependencyGraph<'a> {
    pub fn new(issues: &'a [Issue], theme: Theme) -> Self {
        Self {
            issues,
            theme,
            focus_issue: None,
        }
    }

    pub fn focus(mut self, issue_id: &'a str) -> Self {
        self.focus_issue = Some(issue_id);
        self
    }

    fn build_graph_text(&self) -> Vec<Line> {
        let mut lines = Vec::new();

        // Build dependency map
        let mut dep_map: HashMap<u32, Vec<u32>> = HashMap::new();
        let mut reverse_dep_map: HashMap<u32, Vec<u32>> = HashMap::new();

        for issue in self.issues {
            let id = issue.metadata.id;
            dep_map.insert(id, Vec::new());

            for dep in &issue.metadata.depends_on {
                dep_map.get_mut(&id).unwrap().push(*dep);
                reverse_dep_map
                    .entry(*dep)
                    .or_default()
                    .push(id);
            }
        }

        // If focus issue is set, only show that issue and its dependencies
        let issues_to_show: Vec<u32> = if let Some(focus) = self.focus_issue {
            let focus_id: u32 = focus.trim_start_matches("BUG-").parse().unwrap_or(0);
            let mut to_show = HashSet::new();
            to_show.insert(focus_id);

            // Add dependencies (what it depends on)
            if let Some(deps) = dep_map.get(&focus_id) {
                for dep in deps {
                    to_show.insert(*dep);
                }
            }

            // Add reverse dependencies (what depends on it)
            if let Some(rdeps) = reverse_dep_map.get(&focus_id) {
                for rdep in rdeps {
                    to_show.insert(*rdep);
                }
            }

            to_show.into_iter().collect()
        } else {
            self.issues.iter().map(|i| i.metadata.id).collect()
        };

        // Only show issues with dependencies (or focused issue)
        let mut shown_count = 0;
        let max_to_show = 10;

        for issue_id in issues_to_show.iter() {
            // Skip if we're not in focus mode and this issue has no dependencies
            if self.focus_issue.is_none() {
                let has_deps = dep_map.get(issue_id).map_or(false, |d| !d.is_empty());
                let has_rdeps = reverse_dep_map.get(issue_id).map_or(false, |d| !d.is_empty());

                if !has_deps && !has_rdeps {
                    continue;
                }

                if shown_count >= max_to_show {
                    break;
                }
            }

            shown_count += 1;

            let issue_str = format!("BUG-{}", issue_id);
            let is_focus = self.focus_issue.map_or(false, |f| f == issue_str);
            let style = if is_focus {
                self.theme.selected_style()
            } else {
                self.theme.normal_style()
            };

            // Issue node with better spacing
            let node_line = Line::from(vec![
                Span::styled("  ┌─", self.theme.dim_style()),
                Span::styled(format!(" {} ", issue_str), style),
                Span::styled("─┐", self.theme.dim_style()),
            ]);
            lines.push(node_line);

            // Dependencies
            if let Some(deps) = dep_map.get(issue_id) {
                if !deps.is_empty() {
                    for (idx, dep) in deps.iter().enumerate() {
                        let is_last = idx == deps.len() - 1;
                        let connector = if is_last { "  └──>" } else { "  ├──>" };

                        let dep_line = Line::from(vec![
                            Span::styled(connector, self.theme.dim_style()),
                            Span::raw(" "),
                            Span::styled(format!("BUG-{}", dep), self.theme.title_style()),
                        ]);
                        lines.push(dep_line);
                    }
                }
            }

            lines.push(Line::from("")); // Blank line between issues
        }

        if lines.is_empty() {
            lines.push(Line::from(Span::styled(
                "No dependencies to display",
                self.theme.dim_style(),
            )));
        }

        lines
    }
}

impl Widget for DependencyGraph<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = if let Some(focus) = self.focus_issue {
            format!("Dependency Graph - {}", focus)
        } else {
            "Dependency Graph".to_string()
        };

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.border_style())
            .title(title)
            .title_style(self.theme.title_style());

        let inner = block.inner(area);
        block.render(area, buf);

        let graph_text = self.build_graph_text();
        let paragraph = Paragraph::new(graph_text);
        paragraph.render(inner, buf);
    }
}
