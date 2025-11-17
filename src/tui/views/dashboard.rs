use ratatui::{
   buffer::Buffer,
   layout::{Constraint, Direction, Layout, Rect},
   text::{Line, Span},
   widgets::{Block, Borders, Gauge, Paragraph, Widget},
};

use crate::{
   config::Config,
   issue::IssueWithId,
   tui::{
      theme::Theme,
      widgets::{DependencyGraph, KanbanBoard, MiniChart},
   },
};

pub struct DashboardView<'a> {
   issues:              &'a [IssueWithId],
   theme:               Theme,
   config:              &'a Config,
   selected_pane:       usize,
   selected_column:     usize,
   selected_item:       usize,
   scroll_offset:       usize,
   column_scroll_state: [usize; 5],
   search_query:        Option<&'a str>,
   search_count:        Option<(usize, usize)>,
   sort_by:             Option<&'a str>,
   filter_by:           Option<&'a str>,
}

impl<'a> DashboardView<'a> {
   pub fn new(issues: &'a [IssueWithId], theme: Theme, config: &'a Config) -> Self {
      Self {
         issues,
         theme,
         config,
         selected_pane: 0,
         selected_column: 1,
         selected_item: 0,
         scroll_offset: 0,
         column_scroll_state: [0; 5],
         search_query: None,
         search_count: None,
         sort_by: None,
         filter_by: None,
      }
   }

   pub fn selected_pane(mut self, pane: usize) -> Self {
      self.selected_pane = pane;
      self
   }

   pub fn selection(mut self, column: usize, item: usize) -> Self {
      self.selected_column = column;
      self.selected_item = item;
      self
   }

   pub fn scroll_state(mut self, offset: usize, column_state: [usize; 5]) -> Self {
      self.scroll_offset = offset;
      self.column_scroll_state = column_state;
      self
   }

   pub fn search_state(mut self, query: Option<&'a str>, count: Option<(usize, usize)>) -> Self {
      self.search_query = query;
      self.search_count = count;
      self
   }

   pub fn sort_filter_state(mut self, sort: Option<&'a str>, filter: Option<&'a str>) -> Self {
      self.sort_by = sort;
      self.filter_by = filter;
      self
   }

   fn render_header(&self, area: Rect, buf: &mut Buffer) {
      let total = self.issues.len();
      let critical = self
         .issues
         .iter()
         .filter(|i| i.issue.metadata.priority.to_string() == "Critical")
         .count();
      let high = self
         .issues
         .iter()
         .filter(|i| i.issue.metadata.priority.to_string() == "High")
         .count();
      let done = self
         .issues
         .iter()
         .filter(|i| {
            matches!(
               i.issue.metadata.status,
               crate::issue::Status::Done | crate::issue::Status::Closed
            )
         })
         .count();

      let mut lines = vec![
         Line::from(vec![
            Span::raw("  "),
            Span::styled("AgentX", self.theme.title_style()),
            Span::raw("  "),
            Span::styled("AI-Native Issue Dashboard", self.theme.dim_style()),
         ]),
         Line::from(vec![
            Span::raw("  "),
            Span::styled(
               format!(" CRIT {} ", critical),
               self.theme.status_critical().bg(self.theme.bg()),
            ),
            Span::raw("  "),
            Span::styled(format!(" HIGH {} ", high), self.theme.status_high().bg(self.theme.bg())),
            Span::raw("  "),
            Span::styled(format!(" DONE {} ", done), self.theme.status_done().bg(self.theme.bg())),
            Span::raw("  "),
            Span::styled(format!("Total {}", total), self.theme.dim_style()),
         ]),
      ];

      if let Some(q) = self.search_query {
         let mut search_line = vec![
            Span::raw("  "),
            Span::styled("/ ", self.theme.dim_style()),
            Span::styled(q, self.theme.title_style()),
            Span::raw("_"),
         ];

         if let Some((current, total)) = self.search_count {
            search_line.push(Span::raw("  "));
            search_line
               .push(Span::styled(format!("[{}/{}]", current, total), self.theme.success()));
         } else if !q.is_empty() {
            search_line.push(Span::raw("  "));
            search_line.push(Span::styled("[0 results]", self.theme.dim_style()));
         }

         lines.push(Line::from(search_line));
      }

      let block = Block::default()
         .borders(Borders::NONE)
         .style(self.theme.header_style());

      block.render(area, buf);
      Paragraph::new(lines).render(area, buf);
   }

   fn render_footer(&self, area: Rect, buf: &mut Buffer) {
      let mut footer_spans = if self.search_query.is_some() {
         vec![
            Span::raw("  "),
            Span::styled("[Search Mode]", self.theme.title_style()),
            Span::raw("  "),
            Span::styled("↑↓/Tab", self.theme.dim_style()),
            Span::raw(" Next/Prev  "),
            Span::styled("Enter", self.theme.dim_style()),
            Span::raw(" Jump  "),
            Span::styled("Esc", self.theme.dim_style()),
            Span::raw(" Cancel"),
         ]
      } else {
         vec![
            Span::raw("  "),
            Span::styled("↑↓/hjkl", self.theme.dim_style()),
            Span::raw(" Nav  "),
            Span::styled("/", self.theme.dim_style()),
            Span::raw(" Search  "),
            Span::styled("F2", self.theme.dim_style()),
            Span::raw(" Filter  "),
            Span::styled("F3", self.theme.dim_style()),
            Span::raw(" Sort  "),
            Span::styled("Alt+1-5", self.theme.dim_style()),
            Span::raw(" Jump  "),
            Span::styled("q", self.theme.dim_style()),
            Span::raw(" Quit"),
         ]
      };

      if let Some(sort) = self.sort_by {
         footer_spans.push(Span::raw("  "));
         footer_spans.push(Span::styled(format!("📊 {}", sort), self.theme.warning()));
      }

      if let Some(filter) = self.filter_by {
         footer_spans.push(Span::raw("  "));
         footer_spans.push(Span::styled(format!("🔍 {}", filter), self.theme.success()));
      }

      Paragraph::new(Line::from(footer_spans))
         .style(self.theme.dim_style())
         .render(area, buf);
   }

   fn render_metrics(&self, area: Rect, buf: &mut Buffer) {
      let block = Block::default()
         .borders(Borders::ALL)
         .border_type(self.theme.border_type())
         .border_style(if self.selected_pane == 2 {
            self.theme.active_border_style()
         } else {
            self.theme.border_style()
         })
         .padding(ratatui::widgets::Padding::uniform(1)) // Add padding
         .title(" Metrics ") // Add space around title
         .title_style(self.theme.title_style());

      let inner = block.inner(area);
      block.render(area, buf);

      // Sample metrics data (in real implementation, calculate from issues)
      let burndown_data = [20, 18, 15, 13, 10, 8, 5];

      let metrics_layout = Layout::default()
         .direction(Direction::Vertical)
         .constraints([
            Constraint::Length(3), // Velocity gauge
            Constraint::Length(1), // Spacer
            Constraint::Length(3), // Burndown
            Constraint::Length(1), // Spacer
            Constraint::Min(0),    // Stats
         ])
         .split(inner);

      // Velocity gauge
      let velocity = 8u64;
      let velocity_gauge = Gauge::default()
         .ratio((velocity as f64 / 10.0).min(1.0))
         .label(format!(" Velocity {} pts/day ", velocity))
         .use_unicode(true)
         .style(self.theme.normal_style())
         .gauge_style(
            self
               .theme
               .title_style()
               .bg(self.theme.bg())
               .add_modifier(ratatui::style::Modifier::BOLD),
         );
      velocity_gauge.render(metrics_layout[0], buf);

      // Burndown chart
      MiniChart::new("Burndown", 5, &burndown_data, " pts", self.theme)
         .render(metrics_layout[2], buf);

      // Quick stats with better formatting
      use crate::issue::Status;
      let done_count = self
         .issues
         .iter()
         .filter(|i| matches!(i.issue.metadata.status, Status::Done | Status::Closed))
         .count();
      let wip_count = self
         .issues
         .iter()
         .filter(|i| matches!(i.issue.metadata.status, Status::InProgress))
         .count();
      let blocked_count = self
         .issues
         .iter()
         .filter(|i| matches!(i.issue.metadata.status, Status::Blocked))
         .count();

      let stats = vec![
         Line::from(""),
         Line::from(vec![
            Span::styled("  Done:    ", self.theme.dim_style()),
            Span::styled(format!("{}", done_count), self.theme.success()),
         ]),
         Line::from(""),
         Line::from(vec![
            Span::styled("  WIP:     ", self.theme.dim_style()),
            Span::styled(format!("{}", wip_count), self.theme.warning()),
         ]),
         Line::from(""),
         Line::from(vec![
            Span::styled("  Blocked: ", self.theme.dim_style()),
            Span::styled(format!("{}", blocked_count), self.theme.error()),
         ]),
      ];

      Paragraph::new(stats).render(metrics_layout[4], buf);
   }
}

impl Widget for DashboardView<'_> {
   fn render(self, area: Rect, buf: &mut Buffer) {
      let header_height = if self.search_query.is_some() { 3 } else { 2 };

      let main_layout = Layout::default()
         .direction(Direction::Vertical)
         .constraints([
            Constraint::Length(header_height), // Header
            Constraint::Min(0),                // Main content
            Constraint::Length(1),             // Footer
         ])
         .split(area);

      // Render header and footer
      self.render_header(main_layout[0], buf);
      self.render_footer(main_layout[2], buf);

      // Main content area - 3 column layout with better proportions
      let content_layout = Layout::default()
         .direction(Direction::Horizontal)
         .constraints([
            Constraint::Percentage(50), // Kanban - reduced from 60%
            Constraint::Percentage(30), // Detail/Graph - increased from 20%
            Constraint::Percentage(20), // Metrics
         ])
         .margin(1) // Add margin around the content
         .split(main_layout[1]);

      // Kanban board (left pane)
      KanbanBoard::new(self.issues, self.theme, self.config)
         .selected_column(self.selected_column)
         .selected_item(self.selected_item)
         .scroll_state(self.scroll_offset, self.column_scroll_state)
         .render(content_layout[0], buf);

      // Dependency graph (middle pane)
      let graph_border_style = if self.selected_pane == 1 {
         self.theme.active_border_style()
      } else {
         self.theme.border_style()
      };

      let graph_block = Block::default()
         .borders(Borders::ALL)
         .border_type(self.theme.border_type())
         .border_style(graph_border_style)
         .padding(ratatui::widgets::Padding::uniform(1)) // Add padding
         .title(" Dependencies ") // Add space around title
         .title_style(self.theme.title_style());

      let graph_inner = graph_block.inner(content_layout[1]);
      graph_block.render(content_layout[1], buf);

      DependencyGraph::new(self.issues, self.theme, self.config).render(graph_inner, buf);

      // Metrics (right pane)
      self.render_metrics(content_layout[2], buf);
   }
}
