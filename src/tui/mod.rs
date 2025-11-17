pub mod events;
pub mod theme;
pub mod views;
pub mod widgets;

use crate::issue::Issue;
use crate::storage::Storage;
use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use events::{key_to_action, Action, Event, EventHandler, ViewMode};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::time::Duration;
use theme::Theme;
use views::DashboardView;

pub struct App {
    storage: Storage,
    issues: Vec<Issue>,
    theme: Theme,
    current_view: ViewMode,
    selected_pane: usize,
    selected_issue_idx: Option<usize>,
    should_quit: bool,
}

impl App {
    pub fn new(storage: Storage) -> Result<Self> {
        let mut issues = storage.list_open_issues()?;
        issues.extend(storage.list_closed_issues()?);

        Ok(Self {
            storage,
            issues,
            theme: Theme::default(),
            current_view: ViewMode::Dashboard,
            selected_pane: 0,
            selected_issue_idx: None,
            should_quit: false,
        })
    }

    pub fn handle_action(&mut self, action: Action) -> Result<()> {
        match action {
            Action::Quit => self.should_quit = true,
            Action::Refresh => {
                let mut issues = self.storage.list_open_issues()?;
                issues.extend(self.storage.list_closed_issues()?);
                self.issues = issues;
            }
            Action::SwitchView(view) => {
                self.current_view = view;
            }
            Action::NextPane => {
                self.selected_pane = (self.selected_pane + 1) % 3;
            }
            Action::PrevPane => {
                self.selected_pane = if self.selected_pane == 0 {
                    2
                } else {
                    self.selected_pane - 1
                };
            }
            Action::Up => {
                // Navigate up in current pane
            }
            Action::Down => {
                // Navigate down in current pane
            }
            _ => {}
        }

        Ok(())
    }

    pub fn run(&mut self) -> Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Event handler
        let event_handler = EventHandler::new(Duration::from_millis(250));

        // Main loop
        while !self.should_quit {
            terminal.draw(|f| {
                let size = f.area();

                match self.current_view {
                    ViewMode::Dashboard => {
                        let dashboard = DashboardView::new(&self.issues, self.theme)
                            .selected_pane(self.selected_pane);
                        f.render_widget(dashboard, size);
                    }
                    ViewMode::Kanban => {
                        let kanban = widgets::KanbanBoard::new(&self.issues, self.theme);
                        f.render_widget(kanban, size);
                    }
                    _ => {
                        // Other views not implemented yet
                        use ratatui::text::Line;
                        use ratatui::widgets::Paragraph;

                        let message = Paragraph::new(vec![
                            Line::from("View not yet implemented"),
                            Line::from("Press 'q' to quit"),
                        ]);
                        f.render_widget(message, size);
                    }
                }
            })?;

            // Handle events
            match event_handler.next()? {
                Event::Key(key) => {
                    let action = key_to_action(key);
                    self.handle_action(action)?;
                }
                Event::Resize => {
                    // Terminal was resized, will redraw on next iteration
                }
                _ => {}
            }
        }

        // Restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )?;
        terminal.show_cursor()?;

        Ok(())
    }
}

/// Launch the TUI dashboard
pub fn launch_dashboard(storage: Storage) -> Result<()> {
    let mut app = App::new(storage)?;
    app.run()
}
