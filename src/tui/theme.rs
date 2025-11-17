use ratatui::{
   style::{Color, Modifier, Style},
   widgets::BorderType,
};

#[derive(Debug, Clone, Copy, Default)]
pub enum Theme {
   Default,
   Dracula,
   #[default]
   Nord,
   Solarized,
}

impl Theme {
   pub fn bg(&self) -> Color {
      match self {
         Theme::Default => Color::Reset,
         Theme::Dracula => Color::Rgb(40, 42, 54),
         Theme::Nord => Color::Rgb(46, 52, 64),
         Theme::Solarized => Color::Rgb(0, 43, 54),
      }
   }

   pub fn fg(&self) -> Color {
      match self {
         Theme::Default => Color::White,
         Theme::Dracula => Color::Rgb(248, 248, 242),
         Theme::Nord => Color::Rgb(216, 222, 233),
         Theme::Solarized => Color::Rgb(131, 148, 150),
      }
   }

   pub fn primary(&self) -> Color {
      match self {
         Theme::Default => Color::Cyan,
         Theme::Dracula => Color::Rgb(139, 233, 253),
         Theme::Nord => Color::Rgb(136, 192, 208),
         Theme::Solarized => Color::Rgb(38, 139, 210),
      }
   }

   pub fn success(&self) -> Color {
      match self {
         Theme::Default => Color::Green,
         Theme::Dracula => Color::Rgb(80, 250, 123),
         Theme::Nord => Color::Rgb(163, 190, 140),
         Theme::Solarized => Color::Rgb(133, 153, 0),
      }
   }

   pub fn warning(&self) -> Color {
      match self {
         Theme::Default => Color::Yellow,
         Theme::Dracula => Color::Rgb(241, 250, 140),
         Theme::Nord => Color::Rgb(235, 203, 139),
         Theme::Solarized => Color::Rgb(181, 137, 0),
      }
   }

   pub fn error(&self) -> Color {
      match self {
         Theme::Default => Color::Red,
         Theme::Dracula => Color::Rgb(255, 85, 85),
         Theme::Nord => Color::Rgb(191, 97, 106),
         Theme::Solarized => Color::Rgb(220, 50, 47),
      }
   }

   pub fn highlight(&self) -> Color {
      match self {
         Theme::Default => Color::Blue,
         Theme::Dracula => Color::Rgb(189, 147, 249),
         Theme::Nord => Color::Rgb(129, 161, 193),
         Theme::Solarized => Color::Rgb(108, 113, 196),
      }
   }

   pub fn dim(&self) -> Color {
      match self {
         Theme::Default => Color::DarkGray,
         Theme::Dracula => Color::Rgb(98, 114, 164),
         Theme::Nord => Color::Rgb(76, 86, 106),
         Theme::Solarized => Color::Rgb(88, 110, 117),
      }
   }

   // Styled components
   pub fn title_style(&self) -> Style {
      Style::default()
         .fg(self.primary())
         .add_modifier(Modifier::BOLD)
   }

   pub fn header_style(&self) -> Style {
      Style::default()
         .fg(self.fg())
         .bg(self.dim())
         .add_modifier(Modifier::BOLD)
   }

   pub fn selected_style(&self) -> Style {
      Style::default()
         .fg(self.bg())
         .bg(self.primary())
         .add_modifier(Modifier::BOLD)
   }

   pub fn normal_style(&self) -> Style {
      Style::default().fg(self.fg()).bg(self.bg())
   }

   pub fn dim_style(&self) -> Style {
      Style::default().fg(self.dim())
   }

   pub fn status_critical(&self) -> Style {
      Style::default()
         .fg(self.error())
         .add_modifier(Modifier::BOLD)
   }

   pub fn status_high(&self) -> Style {
      Style::default().fg(self.warning())
   }

   pub fn status_medium(&self) -> Style {
      Style::default().fg(self.primary())
   }

   pub fn status_low(&self) -> Style {
      Style::default().fg(self.dim())
   }

   pub fn status_done(&self) -> Style {
      Style::default().fg(self.success())
   }

   pub fn border_style(&self) -> Style {
      Style::default().fg(self.dim())
   }

   pub fn active_border_style(&self) -> Style {
      Style::default().fg(self.primary())
   }

   pub fn border_type(&self) -> BorderType {
      BorderType::Rounded
   }

   pub fn header_block_style(&self) -> Style {
      Style::default()
         .fg(self.fg())
         .bg(self.dim())
         .add_modifier(Modifier::BOLD)
   }
}
