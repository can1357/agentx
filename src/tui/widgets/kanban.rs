use ratatui::{
   buffer::Buffer,
   layout::{Constraint, Direction, Layout, Rect},
   style::Modifier,
   text::{Line, Span},
   widgets::{Block, Borders, List, ListItem, Widget},
};

use crate::{
   config::Config,
   issue::{IssueWithId, Status},
   tui::theme::Theme,
};

pub struct KanbanBoard<'a> {
   issues:          &'a [IssueWithId],
   theme:           Theme,
   config:          &'a Config,
   selected_column: usize,
   selected_item:   usize,
}

impl<'a> KanbanBoard<'a> {
   pub fn new(issues: &'a [IssueWithId], theme: Theme, config: &'a Config) -> Self {
      Self { issues, theme, config, selected_column: 0, selected_item: 0 }
   }

   pub fn selected_column(mut self, column: usize) -> Self {
      self.selected_column = column;
      self
   }

   pub fn selected_item(mut self, item: usize) -> Self {
      self.selected_item = item;
      self
   }

   fn get_issues_by_status(&self, status: Status) -> Vec<&IssueWithId> {
      self
         .issues
         .iter()
         .filter(|i| i.issue.metadata.status == status)
         .collect()
   }

   fn render_column(
      &self,
      area: Rect,
      buf: &mut Buffer,
      title: &str,
      status: Status,
      column_idx: usize,
   ) {
      let issues = self.get_issues_by_status(status);
      let count = issues.len();

      let is_selected = self.selected_column == column_idx;
      let border_style = if is_selected {
         self.theme.active_border_style()
      } else {
         self.theme.border_style()
      };

      let block = Block::default()
         .borders(Borders::ALL)
         .border_type(self.theme.border_type())
         .border_style(border_style)
         .padding(ratatui::widgets::Padding::horizontal(1)) // Add horizontal padding
         .title(Line::from(vec![
            Span::raw(" "), // Add space before title
            Span::styled(title, self.theme.title_style()),
            Span::styled(format!(" ({count})"), self.theme.dim_style()),
         ]));

      let inner = block.inner(area);
      block.render(area, buf);

      let items: Vec<ListItem> = issues
         .iter()
         .enumerate()
         .map(|(idx, issue)| {
            let is_item_selected = is_selected && idx == self.selected_item;
            let style = if is_item_selected {
               self.theme.selected_style()
            } else {
               self.theme.normal_style()
            };

            let priority_indicator = match issue.issue.metadata.priority.to_string().as_str() {
               "Critical" => "🔴",
               "High" => "🟡",
               "Medium" => "🟢",
               "Low" => "⚪",
               _ => "○",
            };

            let title = truncate(&issue.issue.metadata.title, 25); // Reduced from 30 to give more space

            // Card-style with spacing
            let mut lines = Vec::new();
            lines.push(Line::from("")); // Top spacing

            // ID line
            lines.push(Line::from(vec![
               Span::raw(" "),
               Span::raw(priority_indicator),
               Span::raw(" "),
               Span::styled(
                  self.config.format_issue_ref(issue.id),
                  style.add_modifier(Modifier::BOLD),
               ),
            ]));

            // Title line with tags
            let mut title_spans = vec![Span::raw("   "), Span::styled(title, style)];

            // Add tags if present
            if !issue.issue.metadata.tags.is_empty() {
               let tags = issue
                  .issue
                  .metadata
                  .tags
                  .iter()
                  .map(|t| format!("#{}", t))
                  .collect::<Vec<_>>()
                  .join(" ");
               title_spans.push(Span::raw(" "));
               title_spans.push(Span::styled(tags, self.theme.dim_style()));
            }

            lines.push(Line::from(title_spans));

            // Effort line (if present)
            if let Some(effort) = &issue.issue.metadata.effort {
               lines.push(Line::from(vec![
                  Span::raw("   "),
                  Span::styled(format!("⏱ {effort}"), self.theme.dim_style()),
               ]));
            }

            lines.push(Line::from("")); // Bottom spacing

            ListItem::new(lines).style(style)
         })
         .collect();

      let list = List::new(items);
      list.render(inner, buf);
   }
}

impl Widget for KanbanBoard<'_> {
   fn render(self, area: Rect, buf: &mut Buffer) {
      let columns = Layout::default()
         .direction(Direction::Horizontal)
         .constraints([
            Constraint::Percentage(20), // Backlog
            Constraint::Percentage(20), // Ready
            Constraint::Percentage(20), // In Progress
            Constraint::Percentage(20), // Blocked
            Constraint::Percentage(20), // Done
         ])
         .split(area);

      self.render_column(columns[0], buf, "Backlog", Status::Backlog, 0);
      self.render_column(columns[1], buf, "Ready", Status::NotStarted, 1);
      self.render_column(columns[2], buf, "In Progress", Status::InProgress, 2);
      self.render_column(columns[3], buf, "Blocked", Status::Blocked, 3);
      self.render_column(columns[4], buf, "Done", Status::Done, 4);
   }
}

fn truncate(s: &str, max_len: usize) -> String {
   if s.len() > max_len {
      format!("{}...", &s[..max_len - 3])
   } else {
      s.to_string()
   }
}
