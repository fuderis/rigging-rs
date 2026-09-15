use crossterm::{cursor, queue, terminal};
use std::io::{self, Write};

/// An RAII guard for managing terminal state and cursor visibility.
pub(crate) struct TerminalGuard;

impl TerminalGuard {
    /// Creates a new `TerminalGuard` instance and configures initial terminal mode.
    pub(crate) fn new(show_cursor: bool) -> Self {
        let mut stdout = io::stdout();

        let _ = terminal::enable_raw_mode();
        let _ = queue!(stdout, terminal::DisableLineWrap);

        if show_cursor {
            let _ = queue!(stdout, cursor::Show);
        } else {
            let _ = queue!(stdout, cursor::Hide);
        }

        let _ = stdout.flush();
        Self
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut stdout = io::stdout();
        let _ = terminal::disable_raw_mode();

        let _ = queue!(stdout, crossterm::terminal::EnableLineWrap);
        let _ = queue!(stdout, cursor::Show);

        let _ = stdout.flush();
    }
}
