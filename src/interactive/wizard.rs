use std::path::PathBuf;

use anyhow::Result;
use console::Style;
use dialoguer::{Confirm, Editor, Input, MultiSelect, Select, theme::ColorfulTheme};
use fuzzy_matcher::{FuzzyMatcher, skim::SkimMatcherV2};

/// Create a styled theme for dialoguer prompts
pub fn create_theme() -> ColorfulTheme {
   ColorfulTheme {
      prompt_prefix: Style::new()
         .fg(console::Color::Cyan)
         .bold()
         .apply_to("❯".to_string()),
      prompt_suffix: Style::new()
         .fg(console::Color::Blue)
         .apply_to("›".to_string()),
      success_prefix: Style::new()
         .fg(console::Color::Green)
         .bold()
         .apply_to("✓".to_string()),
      error_prefix: Style::new()
         .fg(console::Color::Red)
         .bold()
         .apply_to("✗".to_string()),
      hint_style: Style::new().dim(),
      values_style: Style::new().fg(console::Color::Green),
      active_item_style: Style::new().fg(console::Color::Cyan).bold(),
      inactive_item_style: Style::new(),
      active_item_prefix: Style::new()
         .fg(console::Color::Cyan)
         .apply_to("❯".to_string()),
      inactive_item_prefix: Style::new().apply_to(" ".to_string()),
      checked_item_prefix: Style::new()
         .fg(console::Color::Green)
         .apply_to("✓".to_string()),
      unchecked_item_prefix: Style::new().apply_to("○".to_string()),
      picked_item_prefix: Style::new()
         .fg(console::Color::Green)
         .apply_to("✓".to_string()),
      unpicked_item_prefix: Style::new().apply_to("○".to_string()),
      ..Default::default()
   }
}

/// Prompt for a required text input with validation
pub fn prompt_required<F>(prompt: &str, validator: F) -> Result<String>
where
   F: Fn(&str) -> Result<()> + 'static,
{
   Input::with_theme(&create_theme())
      .with_prompt(prompt)
      .validate_with(move |input: &String| validator(input).map_err(|e| e.to_string()))
      .interact_text()
      .map_err(Into::into)
}

/// Prompt for optional text input
pub fn prompt_optional(prompt: &str, default: Option<&str>) -> Result<String> {
   let theme = create_theme();
   let mut input = Input::with_theme(&theme).with_prompt(prompt);

   if let Some(def) = default {
      input = input.default(def.to_string());
   }

   input.allow_empty(true).interact_text().map_err(Into::into)
}

/// Prompt for multi-line text using an editor
pub fn prompt_editor(prompt: &str, initial_text: Option<&str>) -> Result<Option<String>> {
   println!("{}", Style::new().bold().cyan().apply_to(prompt));
   Editor::new()
      .require_save(true)
      .edit(initial_text.unwrap_or(""))
      .map_err(Into::into)
}

/// Prompt for a selection from a list
pub fn prompt_select<T: ToString>(prompt: &str, items: &[T]) -> Result<usize> {
   Select::with_theme(&create_theme())
      .with_prompt(prompt)
      .items(items)
      .default(0)
      .interact()
      .map_err(Into::into)
}

/// Prompt for multiple selections from a list
pub fn prompt_multi_select<T: ToString>(
   prompt: &str,
   items: &[T],
   defaults: &[bool],
) -> Result<Vec<usize>> {
   MultiSelect::with_theme(&create_theme())
      .with_prompt(prompt)
      .items(items)
      .defaults(defaults)
      .interact()
      .map_err(Into::into)
}

/// Prompt for confirmation
pub fn prompt_confirm(prompt: &str, default: bool) -> Result<bool> {
   Confirm::with_theme(&create_theme())
      .with_prompt(prompt)
      .default(default)
      .interact()
      .map_err(Into::into)
}

/// Fuzzy search files in the current directory
pub fn fuzzy_search_files(query: &str, max_results: usize) -> Result<Vec<PathBuf>> {
   let matcher = SkimMatcherV2::default();
   let mut results = Vec::new();

   // Walk current directory
   if let Ok(entries) = std::fs::read_dir(".") {
      for entry in entries.flatten() {
         if let Ok(path) = entry.path().canonicalize()
            && let Some(path_str) = path.to_str()
            && matcher.fuzzy_match(path_str, query).is_some()
         {
            results.push(path);
         }
      }
   }

   // Sort by fuzzy match score
   results.sort_by_cached_key(|path| {
      path
         .to_str()
         .and_then(|s| matcher.fuzzy_match(s, query))
         .map(|score| -score)
         .unwrap_or(0)
   });

   results.truncate(max_results);
   Ok(results)
}

/// Display a preview box
pub fn display_preview(title: &str, content: &str) {
   println!();
   println!(
      "{}",
      Style::new()
         .bold()
         .cyan()
         .apply_to("╭────────────────────────────────────────────╮")
   );
   println!("{}", Style::new().bold().apply_to(format!("  {title}")));
   println!(
      "{}",
      Style::new()
         .dim()
         .apply_to("──────────────────────────────────────────────")
   );
   for line in content.lines() {
      println!("  {line}");
   }
   println!(
      "{}",
      Style::new()
         .dim()
         .apply_to("╰────────────────────────────────────────────╯")
   );
   println!();
}

/// Display a success message
pub fn success(message: &str) {
   println!("{} {}", Style::new().green().apply_to("✓"), Style::new().bold().apply_to(message));
}

/// Display an error message
pub fn error(message: &str) {
   eprintln!(
      "{} {}",
      Style::new().red().apply_to("✗"),
      Style::new().bold().red().apply_to(message)
   );
}

/// Display an info message
pub fn info(message: &str) {
   println!("{} {}", Style::new().cyan().apply_to("ℹ"), Style::new().dim().apply_to(message));
}

/// Display a section header
pub fn section(title: &str) {
   println!();
   println!("{}", Style::new().bold().cyan().apply_to(title));
   println!("{}", Style::new().dim().apply_to("─".repeat(title.len())));
}
