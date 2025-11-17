use crate::issue::Issue;
use crate::tui::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Widget, Wrap},
};

pub struct DetailView<'a> {
    issue: &'a Issue,
    theme: Theme,
}

impl<'a> DetailView<'a> {
    pub fn new(issue: &'a Issue, theme: Theme) -> Self {
        Self { issue, theme }
    }

    fn format_metadata(&self) -> Vec<Line<'a>> {
        let mut lines = Vec::new();

        // ID and Title
        lines.push(Line::from(vec![
            Span::styled(
                format!("BUG-{}", self.issue.metadata.id),
                self.theme.title_style(),
            ),
            Span::raw(": "),
            Span::styled(
                &self.issue.metadata.title,
                self.theme.normal_style(),
            ),
        ]));
        lines.push(Line::from(""));

        // Status
        use crate::issue::Status;
        let status_style = match self.issue.metadata.status {
            Status::Done | Status::Closed => self.theme.status_done(),
            Status::Blocked => self.theme.status_critical(),
            Status::InProgress => self.theme.status_high(),
            _ => self.theme.normal_style(),
        };
        lines.push(Line::from(vec![
            Span::styled("Status: ", self.theme.dim_style()),
            Span::styled(
                self.issue.metadata.status.to_string(),
                status_style,
            ),
        ]));

        // Priority
        let priority_style = match self.issue.metadata.priority.to_string().as_str() {
            "Critical" => self.theme.status_critical(),
            "High" => self.theme.status_high(),
            "Medium" => self.theme.status_medium(),
            "Low" => self.theme.status_low(),
            _ => self.theme.normal_style(),
        };
        lines.push(Line::from(vec![
            Span::styled("Priority: ", self.theme.dim_style()),
            Span::styled(
                self.issue.metadata.priority.to_string(),
                priority_style,
            ),
        ]));

        // Created
        lines.push(Line::from(vec![
            Span::styled("Created: ", self.theme.dim_style()),
            Span::styled(
                self.issue.metadata.created.format("%Y-%m-%d %H:%M").to_string(),
                self.theme.normal_style(),
            ),
        ]));

        // Effort (if present)
        if let Some(effort) = &self.issue.metadata.effort {
            lines.push(Line::from(vec![
                Span::styled("Effort: ", self.theme.dim_style()),
                Span::styled(effort, self.theme.normal_style()),
            ]));
        }

        // Related files
        if !self.issue.metadata.files.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Related Files:",
                self.theme.dim_style(),
            )));
            for file in &self.issue.metadata.files {
                lines.push(Line::from(vec![
                    Span::raw("  • "),
                    Span::styled(file, self.theme.normal_style()),
                ]));
            }
        }

        // Dependencies
        if !self.issue.metadata.depends_on.is_empty() {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                "Depends On:",
                self.theme.dim_style(),
            )));
            for dep in &self.issue.metadata.depends_on {
                lines.push(Line::from(vec![
                    Span::raw("  → "),
                    Span::styled(format!("BUG-{}", dep), self.theme.title_style()),
                ]));
            }
        }

        lines
    }

    fn format_content(&self) -> Vec<Line<'a>> {
        let mut lines = Vec::new();

        // Body content
        lines.push(Line::from(Span::styled(
            "Description:",
            self.theme.title_style(),
        )));
        lines.push(Line::from(""));
        for line in self.issue.body.lines() {
            lines.push(Line::from(Span::styled(
                line.to_string(),
                self.theme.normal_style(),
            )));
        }

        lines
    }
}

impl Widget for DetailView<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.border_style())
            .title(format!("Issue Detail - BUG-{}", self.issue.metadata.id))
            .title_style(self.theme.title_style());

        let inner = block.inner(area);
        block.render(area, buf);

        // Split into metadata and content sections
        let sections = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
            .split(inner);

        // Metadata pane
        let metadata_block = Block::default()
            .borders(Borders::RIGHT)
            .border_style(self.theme.dim_style());

        let metadata_inner = metadata_block.inner(sections[0]);
        metadata_block.render(sections[0], buf);

        let metadata = Paragraph::new(self.format_metadata()).wrap(Wrap { trim: true });
        metadata.render(metadata_inner, buf);

        // Content pane
        let content = Paragraph::new(self.format_content())
            .wrap(Wrap { trim: true })
            .scroll((0, 0)); // TODO: Add scroll position

        content.render(sections[1], buf);
    }
}
