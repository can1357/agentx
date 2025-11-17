use ratatui::{
   buffer::Buffer,
   layout::Rect,
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
   issues:              &'a [IssueWithId],
   theme:               Theme,
   config:              &'a Config,
   selected_column:     usize,
   selected_item:       usize,
   scroll_offset:       usize,
   column_scroll_state: [usize; 5],
}

impl<'a> KanbanBoard<'a> {
   pub fn new(issues: &'a [IssueWithId], theme: Theme, config: &'a Config) -> Self {
      Self {
         issues,
         theme,
         config,
         selected_column: 0,
         selected_item: 0,
         scroll_offset: 0,
         column_scroll_state: [0; 5],
      }
   }

   pub fn selected_column(mut self, column: usize) -> Self {
      self.selected_column = column;
      self
   }

   pub fn selected_item(mut self, item: usize) -> Self {
      self.selected_item = item;
      self
   }

   pub fn scroll_state(mut self, offset: usize, column_state: [usize; 5]) -> Self {
      self.scroll_offset = offset;
      self.column_scroll_state = column_state;
      self
   }

   fn get_issues_by_status(&self, status: Status) -> Vec<&IssueWithId> {
      self
         .issues
         .iter()
         .filter(|i| i.issue.metadata.status == status)
         .collect()
   }
}

impl Widget for KanbanBoard<'_> {
   fn render(self, area: Rect, buf: &mut Buffer) {
      let block = Block::default()
         .borders(Borders::ALL)
         .border_type(self.theme.border_type())
         .border_style(self.theme.active_border_style())
         .title(" All Issues ");

      let inner = block.inner(area);
      block.render(area, buf);

      let statuses = [
         (Status::Backlog, "BACKLOG"),
         (Status::NotStarted, "READY"),
         (Status::InProgress, "IN PROGRESS"),
         (Status::Blocked, "BLOCKED"),
         (Status::Done, "DONE"),
      ];

      let mut all_items = Vec::new();

      for (status, status_name) in &statuses {
         let issues = self.get_issues_by_status(*status);

         if !issues.is_empty() {
            all_items.push((None, status_name.to_string()));

            for issue in issues {
               all_items.push((Some(issue), String::new()));
            }
         }
      }

      let scroll_offset = self.column_scroll_state[self.selected_column];
      let visible_height = inner.height as usize;
      let lines_per_item = 5;
      let max_visible_items = (visible_height / lines_per_item).max(1);

      let visible_items: Vec<_> = all_items
         .iter()
         .skip(scroll_offset)
         .take(max_visible_items)
         .collect();

      let items: Vec<ListItem> = visible_items
         .iter()
         .enumerate()
         .flat_map(|(visible_idx, (issue_opt, status_name))| {
            let actual_idx = scroll_offset + visible_idx;

            if let Some(issue) = issue_opt {
               let is_item_selected = actual_idx == self.selected_item;
               let (style, marker) = if is_item_selected {
                  (self.theme.selected_style(), "▶ ")
               } else {
                  (self.theme.normal_style(), "  ")
               };

               let priority_indicator = match issue.issue.metadata.priority.to_string().as_str() {
                  "Critical" => "🔴",
                  "High" => "🟡",
                  "Medium" => "🟢",
                  "Low" => "⚪",
                  _ => "○",
               };

               let title = truncate(&issue.issue.metadata.title, 80);

               let mut lines = Vec::new();
               lines.push(Line::from(""));

               lines.push(Line::from(vec![
                  Span::raw(marker),
                  Span::raw(priority_indicator),
                  Span::raw(" "),
                  Span::styled(
                     self.config.format_issue_ref(issue.id),
                     style.add_modifier(Modifier::BOLD),
                  ),
               ]));

               let mut title_spans = vec![Span::raw("   "), Span::styled(title, style)];

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

               if let Some(effort) = &issue.issue.metadata.effort {
                  lines.push(Line::from(vec![
                     Span::raw("   "),
                     Span::styled(format!("⏱ {effort}"), self.theme.dim_style()),
                  ]));
               }

               lines.push(Line::from(""));

               Some(ListItem::new(lines).style(style))
            } else {
               let mut lines = Vec::new();
               lines.push(Line::from(""));
               lines.push(Line::from(vec![
                  Span::raw("  "),
                  Span::styled(
                     format!("━━━ {} ━━━", status_name),
                     self.theme.title_style().add_modifier(Modifier::BOLD),
                  ),
               ]));
               lines.push(Line::from(""));

               Some(ListItem::new(lines).style(self.theme.dim_style()))
            }
         })
         .collect();

      let list = List::new(items);
      list.render(inner, buf);
   }
}

fn truncate(s: &str, max_len: usize) -> String {
   if s.len() > max_len {
      format!("{}...", &s[..max_len - 3])
   } else {
      s.to_string()
   }
}
