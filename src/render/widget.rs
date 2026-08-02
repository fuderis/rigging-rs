use crossterm::event::KeyEvent;

/// A trait representing an interactive terminal widget.
///
/// Implementors manage their own internal state, respond to terminal events,
/// and render their content as a vector of formatted strings.
pub trait Widget: Send + Sync {
    /// The output type returned when the widget completes its lifecycle.
    type Output;

    /// Returns `true` if the widget's state has changed and requires a re-render.
    fn is_changed(&self) -> bool;

    /// Renders the widget content to fit within the specified width.
    ///
    /// # Parameters
    ///
    /// * `width` - The available horizontal space in columns.
    fn render_content(&mut self, width: usize) -> Vec<String>;

    /// Handles incoming keyboard input events.
    ///
    /// The default implementation does nothing.
    fn handle_key(&mut self, _key: KeyEvent) {}

    /// Notifies the widget that the terminal or container size has changed.
    ///
    /// # Parameters
    ///
    /// * `_cols` - The new width in columns.
    /// * `_rows` - The new height in rows.
    fn on_resize(&mut self, _cols: u16, _rows: u16) {}

    /// Returns `true` if the widget has finished its execution.
    ///
    /// Defaults to returning `true`.
    fn is_finished(&self) -> bool {
        true
    }

    /// Consumes the widget and returns its final output value.
    fn extract_output(self) -> Self::Output;

    /// Returns the relative cursor coordinates `(column, row)` within the content area.
    ///
    /// Returns `None` if the cursor does not need explicit positioning.
    fn cursor_position(&self) -> Option<(usize, usize)> {
        None
    }

    /// Indicates whether the terminal cursor should be visible.
    ///
    /// By default, returns `true` if [`cursor_position`](Self::cursor_position) returns `Some`.
    fn show_cursor(&self) -> bool {
        self.cursor_position().is_some()
    }
}
