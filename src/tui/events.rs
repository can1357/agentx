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
   PageUp,
   PageDown,
   Home,
   End,
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
   JumpToStatus(usize),
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
      KeyCode::PageUp => Action::PageUp,
      KeyCode::PageDown => Action::PageDown,
      KeyCode::Home | KeyCode::Char('g') => Action::Home,
      KeyCode::End | KeyCode::Char('G') => Action::End,

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

      // Status jumps (Alt+1 through Alt+5)
      KeyCode::Char('1') if key.modifiers.contains(KeyModifiers::ALT) => Action::JumpToStatus(0),
      KeyCode::Char('2') if key.modifiers.contains(KeyModifiers::ALT) => Action::JumpToStatus(1),
      KeyCode::Char('3') if key.modifiers.contains(KeyModifiers::ALT) => Action::JumpToStatus(2),
      KeyCode::Char('4') if key.modifiers.contains(KeyModifiers::ALT) => Action::JumpToStatus(3),
      KeyCode::Char('5') if key.modifiers.contains(KeyModifiers::ALT) => Action::JumpToStatus(4),

      // View switching (only when not using modifiers)
      KeyCode::Char('1')
         if !key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::ALT) =>
      {
         Action::SwitchView(ViewMode::Dashboard)
      },
      KeyCode::Char('2')
         if !key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::ALT) =>
      {
         Action::SwitchView(ViewMode::Kanban)
      },
      KeyCode::Char('3')
         if !key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::ALT) =>
      {
         Action::SwitchView(ViewMode::List)
      },
      KeyCode::Char('4')
         if !key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::ALT) =>
      {
         Action::SwitchView(ViewMode::Metrics)
      },
      KeyCode::Char('5')
         if !key.modifiers.contains(KeyModifiers::CONTROL)
            && !key.modifiers.contains(KeyModifiers::ALT) =>
      {
         Action::SwitchView(ViewMode::Graph)
      },

      _ => Action::None,
   }
}
