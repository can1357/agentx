pub mod validators;
pub mod wizard;
pub mod wizards;

use anyhow::Result;
use console::Term;

/// Trait for commands that support interactive mode
pub trait Interactive {
    /// Run the command in interactive mode
    fn run_interactive(&self) -> Result<()>;
}

/// Check if we're running in an interactive terminal
pub fn is_interactive_terminal() -> bool {
    Term::stdout().is_term() && atty::is(atty::Stream::Stdin)
}

/// Check if interactive mode should be enabled based on:
/// - Explicit --interactive flag
/// - Missing required arguments
/// - Terminal capabilities
pub fn should_use_interactive(force: bool, has_required_args: bool) -> bool {
    if force {
        return is_interactive_terminal();
    }

    // Auto-enable if terminal is interactive and required args are missing
    !has_required_args && is_interactive_terminal()
}
