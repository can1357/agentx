pub mod events;
pub mod theme;
pub mod views;
pub mod widgets;

use std::{io, time::Duration};

use anyhow::Result;
use crossterm::{
   event::{DisableMouseCapture, EnableMouseCapture, KeyCode, KeyEvent},
   execute,
   terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use events::{Action, Event, EventHandler, ViewMode, key_to_action};
use ratatui::{Terminal, backend::CrosstermBackend};
use theme::Theme;
use views::DashboardView;

use crate::{config::Config, issue::IssueWithId, storage::Storage};

pub struct App {
   storage:             Storage,
   issues:              Vec<IssueWithId>,
   theme:               Theme,
   config:              Config,
   current_view:        ViewMode,
   selected_pane:       usize,
   selected_column:     usize,
   selected_item:       usize,
   scroll_offset:       usize,
   column_scroll_state: [usize; 5],
   mode:                AppMode,
   search_query:        String,
   search_results:      Vec<(usize, usize)>,
   current_search_idx:  usize,
   sort_mode:           SortMode,
   filter_priority:     Option<String>,
   should_quit:         bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SortMode {
   Status,
   Priority,
   Effort,
   Created,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppMode {
   Normal,
   Search,
}

impl App {
   pub fn new(storage: Storage) -> Result<Self> {
      let mut issues = storage.list_open_issues()?;
      issues.extend(storage.list_closed_issues()?);

      Ok(Self {
         storage,
         issues,
         theme: Theme::default(),
         config: Config::load(),
         current_view: ViewMode::Dashboard,
         selected_pane: 0,
         selected_column: 1,
         selected_item: 0,
         scroll_offset: 0,
         column_scroll_state: [0; 5],
         mode: AppMode::Normal,
         search_query: String::new(),
         search_results: Vec::new(),
         current_search_idx: 0,
         sort_mode: SortMode::Status,
         filter_priority: None,
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
         },
         Action::SwitchView(view) => {
            self.current_view = view;
         },
         Action::NextPane => {
            self.selected_pane = (self.selected_pane + 1) % 3;
         },
         Action::PrevPane => {
            self.selected_pane = if self.selected_pane == 0 {
               2
            } else {
               self.selected_pane - 1
            };
         },
         Action::Up => {
            self.move_selection_vertical(-1);
         },
         Action::Down => {
            self.move_selection_vertical(1);
         },
         Action::Left => {
            self.move_selection_horizontal(-1);
         },
         Action::Right => {
            self.move_selection_horizontal(1);
         },
         Action::PageUp => {
            self.move_selection_vertical(-5);
         },
         Action::PageDown => {
            self.move_selection_vertical(5);
         },
         Action::Home => {
            if self.current_view == ViewMode::Dashboard && self.selected_pane == 0 {
               let all_items = self.all_issues_flattened();
               for (idx, (issue, _)) in all_items.iter().enumerate() {
                  if issue.is_some() {
                     self.selected_item = idx;
                     self.column_scroll_state[self.selected_column] = 0;
                     break;
                  }
               }
            }
         },
         Action::End => {
            if self.current_view == ViewMode::Dashboard && self.selected_pane == 0 {
               let all_items = self.all_issues_flattened();
               for (idx, (issue, _)) in all_items.iter().enumerate().rev() {
                  if issue.is_some() {
                     self.selected_item = idx;
                     self.update_scroll_for_item();
                     break;
                  }
               }
            }
         },
         Action::Search => {
            self.mode = AppMode::Search;
            self.search_query.clear();
         },
         Action::Select => {
            if self.current_view == ViewMode::Dashboard && self.selected_pane == 0 {
               let all_items = self.all_issues_flattened();
               if let Some((Some(issue), _)) = all_items.get(self.selected_item) {
                  // TODO: Open issue detail view
                  eprintln!("Selected issue: {}", issue.id);
               }
            }
         },
         Action::JumpToStatus(status_idx) => {
            if self.current_view == ViewMode::Dashboard && self.selected_pane == 0 {
               self.jump_to_status_section(status_idx);
            }
         },
         Action::Sort => {
            self.cycle_sort_mode();
         },
         Action::Filter => {
            self.cycle_filter_priority();
         },
         _ => {},
      }

      Ok(())
   }

   fn all_issues_flattened(&self) -> Vec<(Option<&IssueWithId>, String)> {
      use crate::issue::Status;

      let statuses = [
         (Status::Backlog, "BACKLOG"),
         (Status::NotStarted, "READY"),
         (Status::InProgress, "IN PROGRESS"),
         (Status::Blocked, "BLOCKED"),
         (Status::Done, "DONE"),
      ];

      let mut result = Vec::new();

      for (status, status_name) in &statuses {
         let mut issues: Vec<_> = self
            .issues
            .iter()
            .filter(|i| i.issue.metadata.status == *status)
            .collect();

         if let Some(ref priority_filter) = self.filter_priority {
            issues.retain(|i| i.issue.metadata.priority.to_string() == *priority_filter);
         }

         if self.sort_mode != SortMode::Status {
            issues.sort_by(|a, b| match self.sort_mode {
               SortMode::Priority => {
                  let priority_order = |p: &str| match p {
                     "Critical" => 0,
                     "High" => 1,
                     "Medium" => 2,
                     "Low" => 3,
                     _ => 4,
                  };
                  priority_order(&a.issue.metadata.priority.to_string())
                     .cmp(&priority_order(&b.issue.metadata.priority.to_string()))
               },
               SortMode::Effort => {
                  let effort_hours = |e: &Option<smol_str::SmolStr>| {
                     e.as_ref()
                        .and_then(|s| {
                           let s = s.as_str();
                           if s.ends_with('h') {
                              s.trim_end_matches('h').parse::<u32>().ok()
                           } else if s.ends_with('d') {
                              s.trim_end_matches('d').parse::<u32>().map(|d| d * 8).ok()
                           } else if s.ends_with('w') {
                              s.trim_end_matches('w').parse::<u32>().map(|w| w * 40).ok()
                           } else {
                              None
                           }
                        })
                        .unwrap_or(0)
                  };
                  effort_hours(&a.issue.metadata.effort)
                     .cmp(&effort_hours(&b.issue.metadata.effort))
               },
               SortMode::Created => a.issue.metadata.created.cmp(&b.issue.metadata.created),
               SortMode::Status => std::cmp::Ordering::Equal,
            });
         }

         if !issues.is_empty() {
            result.push((None, status_name.to_string()));
            for issue in issues {
               result.push((Some(issue), String::new()));
            }
         }
      }

      result
   }

   fn move_selection_vertical(&mut self, delta: i32) {
      if self.current_view != ViewMode::Dashboard || self.selected_pane != 0 {
         return;
      }

      let all_items = self.all_issues_flattened();
      if all_items.is_empty() {
         self.selected_item = 0;
         self.column_scroll_state[self.selected_column] = 0;
         return;
      }

      let len = all_items.len() as i32;
      let mut idx = self.selected_item as i32 + delta;

      loop {
         if idx < 0 {
            idx = 0;
            break;
         }
         if idx >= len {
            idx = len - 1;
            break;
         }

         if all_items
            .get(idx as usize)
            .and_then(|(issue, _)| *issue)
            .is_some()
         {
            break;
         }

         idx += delta.signum();
      }

      self.selected_item = idx as usize;
      self.update_scroll_for_item();
   }

   fn update_scroll_for_item(&mut self) {
      const VISIBLE_ITEMS_PER_SECTION: usize = 3;

      let scroll = &mut self.column_scroll_state[self.selected_column];

      if self.selected_item < *scroll {
         *scroll = self.selected_item;
      } else if self.selected_item >= *scroll + VISIBLE_ITEMS_PER_SECTION {
         *scroll = self
            .selected_item
            .saturating_sub(VISIBLE_ITEMS_PER_SECTION - 1);
      }
   }

   fn move_selection_horizontal(&mut self, _delta: i32) {
      // Not used in unified list view
   }

   fn jump_to_status_section(&mut self, status_idx: usize) {
      use crate::issue::Status;

      let target_status = match status_idx {
         0 => Status::Backlog,
         1 => Status::NotStarted,
         2 => Status::InProgress,
         3 => Status::Blocked,
         4 => Status::Done,
         _ => return,
      };

      let all_items = self.all_issues_flattened();
      for (idx, (issue_opt, _)) in all_items.iter().enumerate() {
         if let Some(issue) = issue_opt
            && issue.issue.metadata.status == target_status
         {
            self.selected_item = idx;
            self.column_scroll_state[self.selected_column] = 0;
            self.update_scroll_for_item();
            break;
         }
      }
   }

   fn cycle_sort_mode(&mut self) {
      self.sort_mode = match self.sort_mode {
         SortMode::Status => SortMode::Priority,
         SortMode::Priority => SortMode::Effort,
         SortMode::Effort => SortMode::Created,
         SortMode::Created => SortMode::Status,
      };
   }

   fn cycle_filter_priority(&mut self) {
      self.filter_priority = match &self.filter_priority {
         None => Some("Critical".to_string()),
         Some(p) if p == "Critical" => Some("High".to_string()),
         Some(p) if p == "High" => Some("Medium".to_string()),
         Some(p) if p == "Medium" => Some("Low".to_string()),
         _ => None,
      };
   }

   fn handle_search_key(&mut self, key: KeyEvent) -> Result<()> {
      match key.code {
         KeyCode::Esc => {
            self.mode = AppMode::Normal;
            self.search_results.clear();
            self.current_search_idx = 0;
         },
         KeyCode::Enter => {
            if !self.search_results.is_empty() {
               let (col, idx) = self.search_results[self.current_search_idx];
               self.selected_column = col;
               self.selected_item = idx;
               self.update_scroll_for_item();
               self.current_search_idx = (self.current_search_idx + 1) % self.search_results.len();
            }
         },
         KeyCode::Backspace => {
            self.search_query.pop();
            self.update_search_results();
         },
         KeyCode::Char(c) => {
            self.search_query.push(c);
            self.update_search_results();
         },
         KeyCode::Down | KeyCode::Tab => {
            if !self.search_results.is_empty() {
               self.current_search_idx = (self.current_search_idx + 1) % self.search_results.len();
               let (col, idx) = self.search_results[self.current_search_idx];
               self.selected_column = col;
               self.selected_item = idx;
               self.update_scroll_for_item();
            }
         },
         KeyCode::Up | KeyCode::BackTab => {
            if !self.search_results.is_empty() {
               self.current_search_idx = if self.current_search_idx == 0 {
                  self.search_results.len() - 1
               } else {
                  self.current_search_idx - 1
               };
               let (col, idx) = self.search_results[self.current_search_idx];
               self.selected_column = col;
               self.selected_item = idx;
               self.update_scroll_for_item();
            }
         },
         _ => {},
      }
      Ok(())
   }

   fn update_search_results(&mut self) {
      self.search_results = self.find_all_matching(&self.search_query);
      self.current_search_idx = 0;
      if !self.search_results.is_empty() {
         let (col, idx) = self.search_results[0];
         self.selected_column = col;
         self.selected_item = idx;
         self.update_scroll_for_item();
      }
   }

   fn find_all_matching(&self, query: &str) -> Vec<(usize, usize)> {
      if query.is_empty() {
         return Vec::new();
      }

      let q = query.to_lowercase();
      let mut results = Vec::new();
      let all_items = self.all_issues_flattened();

      for (idx, (issue_opt, _)) in all_items.iter().enumerate() {
         if let Some(issue) = issue_opt
            && (issue.issue.metadata.title.to_lowercase().contains(&q)
               || self
                  .config
                  .format_issue_ref(issue.id)
                  .to_lowercase()
                  .contains(&q)
               || issue
                  .issue
                  .metadata
                  .tags
                  .iter()
                  .any(|t| t.to_lowercase().contains(&q)))
         {
            results.push((0, idx));
         }
      }

      results
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
                  let (search_query, search_count) = if self.mode == AppMode::Search {
                     (
                        Some(self.search_query.as_str()),
                        if self.search_results.is_empty() {
                           None
                        } else {
                           Some((self.current_search_idx + 1, self.search_results.len()))
                        },
                     )
                  } else {
                     (None, None)
                  };

                  let sort_info = match self.sort_mode {
                     SortMode::Status => None,
                     SortMode::Priority => Some("Priority"),
                     SortMode::Effort => Some("Effort"),
                     SortMode::Created => Some("Created"),
                  };

                  let filter_info = self.filter_priority.as_deref();

                  let dashboard = DashboardView::new(&self.issues, self.theme, &self.config)
                     .selected_pane(self.selected_pane)
                     .selection(self.selected_column, self.selected_item)
                     .scroll_state(self.scroll_offset, self.column_scroll_state)
                     .search_state(search_query, search_count)
                     .sort_filter_state(sort_info, filter_info);
                  f.render_widget(dashboard, size);
               },
               ViewMode::Kanban => {
                  let kanban = widgets::KanbanBoard::new(&self.issues, self.theme, &self.config);
                  f.render_widget(kanban, size);
               },
               _ => {
                  // Other views not implemented yet
                  use ratatui::{text::Line, widgets::Paragraph};

                  let message = Paragraph::new(vec![
                     Line::from("View not yet implemented"),
                     Line::from("Press 'q' to quit"),
                  ]);
                  f.render_widget(message, size);
               },
            }
         })?;

         // Handle events
         match event_handler.next()? {
            Event::Key(key) => match self.mode {
               AppMode::Normal => {
                  let action = key_to_action(key);
                  self.handle_action(action)?;
               },
               AppMode::Search => {
                  self.handle_search_key(key)?;
               },
            },
            Event::Resize => {
               // Terminal was resized, will redraw on next iteration
            },
            _ => {},
         }
      }

      // Restore terminal
      disable_raw_mode()?;
      execute!(terminal.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
      terminal.show_cursor()?;

      Ok(())
   }
}

/// Launch the TUI dashboard
pub fn launch_dashboard(storage: Storage) -> Result<()> {
   let mut app = App::new(storage)?;
   app.run()
}
