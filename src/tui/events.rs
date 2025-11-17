use std::time::Duration;

use anyhow::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
   Tick,
   Key(KeyEvent),
   Mouse,
   Resize,
}

pub struct EventHandler {
   tick_rate: Duration,
}

impl EventHandler {
   pub fn new(tick_rate: Duration) -> Self {
      Self { tick_rate }
   }

   pub fn next(&self) -> Result<Event> {
      if event::poll(self.tick_rate)? {
         match event::read()? {
            CrosstermEvent::Key(key) => Ok(Event::Key(key)),
            CrosstermEvent::Mouse(_) => Ok(Event::Mouse),
            CrosstermEvent::Resize(..) => Ok(Event::Resize),
            _ => Ok(Event::Tick),
         }
      } else {
         Ok(Event::Tick)
      }
   }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
   Quit,
   Up,
   Down,
   Left,
   Right,
   Select,
   Back,
   Help,
   Refresh,
   Filter,
   Sort,
   Search,
   New,
   Edit,
   Delete,
   NextPane,
   PrevPane,
   SwitchView(ViewMode),
   None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
   Dashboard,
   Kanban,
   List,
   Metrics,
   Graph,
}

pub fn key_to_action(key: KeyEvent) -> Action {
   match key.code {
      KeyCode::Char('q') | KeyCode::Esc if key.modifiers.contains(KeyModifiers::NONE) => {
         Action::Quit
      },
      KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Quit,

      // Navigation
      KeyCode::Up | KeyCode::Char('k') => Action::Up,
      KeyCode::Down | KeyCode::Char('j') => Action::Down,
      KeyCode::Left | KeyCode::Char('h') => Action::Left,
      KeyCode::Right | KeyCode::Char('l') => Action::Right,

      // Actions
      KeyCode::Enter | KeyCode::Char(' ') => Action::Select,
      KeyCode::Backspace => Action::Back,
      KeyCode::F(1) => Action::Help,
      KeyCode::F(2) => Action::Filter,
      KeyCode::F(3) => Action::Sort,
      KeyCode::F(5) | KeyCode::Char('r') if key.modifiers.contains(KeyModifiers::CONTROL) => {
         Action::Refresh
      },

      // Pane switching
      KeyCode::Tab => Action::NextPane,
      KeyCode::BackTab => Action::PrevPane,

      // Command palette
      KeyCode::Char('/') | KeyCode::Char(':') => Action::Search,

      // Quick actions
      KeyCode::Char('n') => Action::New,
      KeyCode::Char('e') => Action::Edit,
      KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => Action::Delete,

      // View switching
      KeyCode::Char('1') => Action::SwitchView(ViewMode::Dashboard),
      KeyCode::Char('2') => Action::SwitchView(ViewMode::Kanban),
      KeyCode::Char('3') => Action::SwitchView(ViewMode::List),
      KeyCode::Char('4') => Action::SwitchView(ViewMode::Metrics),
      KeyCode::Char('5') => Action::SwitchView(ViewMode::Graph),

      _ => Action::None,
   }
}
