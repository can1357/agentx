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
   issues:        &'a [IssueWithId],
   theme:         Theme,
   config:        &'a Config,
   selected_pane: usize,
}

impl<'a> DashboardView<'a> {
   pub fn new(issues: &'a [IssueWithId], theme: Theme, config: &'a Config) -> Self {
      Self { issues, theme, config, selected_pane: 0 }
   }

   pub fn selected_pane(mut self, pane: usize) -> Self {
      self.selected_pane = pane;
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

      let lines = vec![
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

      let block = Block::default()
         .borders(Borders::NONE)
         .style(self.theme.header_style());

      block.render(area, buf);
      Paragraph::new(lines).render(area, buf);
   }

   fn render_footer(&self, area: Rect, buf: &mut Buffer) {
      let footer = Line::from(vec![
         Span::raw("  "), // Leading space
         Span::styled("[F1]", self.theme.dim_style()),
         Span::raw(" Help  "),
         Span::styled("[F2]", self.theme.dim_style()),
         Span::raw(" Filter  "),
         Span::styled("[F3]", self.theme.dim_style()),
         Span::raw(" Sort  "),
         Span::styled("[/]", self.theme.dim_style()),
         Span::raw(" Search  "),
         Span::styled("[n]", self.theme.dim_style()),
         Span::raw(" New  "),
         Span::styled("[q]", self.theme.dim_style()),
         Span::raw(" Quit"),
      ]);

      Paragraph::new(footer)
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
      let main_layout = Layout::default()
         .direction(Direction::Vertical)
         .constraints([
            Constraint::Length(2), // Header
            Constraint::Min(0),    // Main content
            Constraint::Length(1), // Footer
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
         .selected_column(if self.selected_pane == 0 {
            0
         } else {
            usize::MAX
         })
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
