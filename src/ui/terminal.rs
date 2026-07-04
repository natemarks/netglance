//! Terminal setup and cleanup utilities.

use anyhow::{Context, Result};
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::{self, Stdout};
use std::panic;

/// Type alias for the terminal backend we use
pub type TerminalBackend = CrosstermBackend<Stdout>;

/// Set up the terminal for TUI rendering.
///
/// This function:
/// - Enables raw mode (disables line buffering, echo, etc.)
/// - Enters the alternate screen buffer
/// - Sets up panic handler to restore terminal on panic
///
/// # Errors
///
/// Returns an error if terminal operations fail.
pub fn setup_terminal() -> Result<Terminal<TerminalBackend>> {
    // Set up panic handler to restore terminal
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = restore_terminal();
        original_hook(panic_info);
    }));

    enable_raw_mode().context("Failed to enable raw mode")?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).context("Failed to enter alternate screen")?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend).context("Failed to create terminal")?;

    Ok(terminal)
}

/// Restore the terminal to its original state.
///
/// This function:
/// - Leaves the alternate screen buffer
/// - Disables raw mode
///
/// # Errors
///
/// Returns an error if terminal operations fail.
pub fn restore_terminal() -> Result<()> {
    disable_raw_mode().context("Failed to disable raw mode")?;
    execute!(io::stdout(), LeaveAlternateScreen).context("Failed to leave alternate screen")?;
    Ok(())
}
