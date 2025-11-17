use crate::tui::theme::Theme;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    symbols,
    widgets::{Block, Borders, Sparkline as RatatuiSparkline, Widget},
};

pub struct MetricsSparkline<'a> {
    title: &'a str,
    data: &'a [u64],
    theme: Theme,
    max_value: Option<u64>,
}

impl<'a> MetricsSparkline<'a> {
    pub fn new(title: &'a str, data: &'a [u64], theme: Theme) -> Self {
        Self {
            title,
            data,
            theme,
            max_value: None,
        }
    }

    pub fn max_value(mut self, max: u64) -> Self {
        self.max_value = Some(max);
        self
    }
}

impl Widget for MetricsSparkline<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(self.theme.border_style())
            .title(self.title)
            .title_style(self.theme.title_style());

        let inner = block.inner(area);
        block.render(area, buf);

        let sparkline = RatatuiSparkline::default()
            .data(self.data)
            .style(Style::default().fg(self.theme.primary()))
            .bar_set(symbols::bar::NINE_LEVELS);

        let sparkline = if let Some(max) = self.max_value {
            sparkline.max(max)
        } else {
            sparkline
        };

        sparkline.render(inner, buf);
    }
}

// Helper for creating a mini chart with labels
pub struct MiniChart<'a> {
    title: &'a str,
    current_value: u64,
    data: &'a [u64],
    unit: &'a str,
    theme: Theme,
}

impl<'a> MiniChart<'a> {
    pub fn new(title: &'a str, current_value: u64, data: &'a [u64], unit: &'a str, theme: Theme) -> Self {
        Self {
            title,
            current_value,
            data,
            unit,
            theme,
        }
    }
}

impl Widget for MiniChart<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use ratatui::layout::{Constraint, Direction, Layout};
        use ratatui::text::{Line, Span};
        use ratatui::widgets::Paragraph;

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(1), Constraint::Min(0)])
            .split(area);

        // Value line
        let value_line = Line::from(vec![
            Span::styled(self.title, self.theme.dim_style()),
            Span::raw(": "),
            Span::styled(
                format!("{}{}", self.current_value, self.unit),
                self.theme.title_style(),
            ),
        ]);

        Paragraph::new(value_line).render(chunks[0], buf);

        // Sparkline
        let sparkline = RatatuiSparkline::default()
            .data(self.data)
            .style(Style::default().fg(self.theme.primary()))
            .bar_set(symbols::bar::THREE_LEVELS);

        sparkline.render(chunks[1], buf);
    }
}
