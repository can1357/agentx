use crate::issue::{Issue, Status};
use crate::tui::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget},
};

pub struct KanbanBoard<'a> {
    issues: &'a [Issue],
    theme: Theme,
    selected_column: usize,
    selected_item: usize,
}

impl<'a> KanbanBoard<'a> {
    pub fn new(issues: &'a [Issue], theme: Theme) -> Self {
        Self {
            issues,
            theme,
            selected_column: 0,
            selected_item: 0,
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

    fn get_issues_by_status(&self, status: Status) -> Vec<&Issue> {
        self.issues
            .iter()
            .filter(|i| i.metadata.status == status)
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

                let priority_indicator = match issue.metadata.priority.to_string().as_str() {
                    "Critical" => "🔴",
                    "High" => "🟡",
                    "Medium" => "🟢",
                    "Low" => "⚪",
                    _ => "○",
                };

                let title = truncate(&issue.metadata.title, 25); // Reduced from 30 to give more space

                // Add padding around each item
                let content = vec![
                    Line::from(vec![
                        Span::raw(" "), // Leading space
                        Span::raw(priority_indicator),
                        Span::raw(" "),
                        Span::styled(format!("BUG-{}", issue.metadata.id), style.add_modifier(Modifier::BOLD)),
                    ]),
                    Line::from(vec![
                        Span::raw("   "), // Indent the title
                        Span::styled(title, style),
                    ]),
                ];

                ListItem::new(content).style(style)
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
