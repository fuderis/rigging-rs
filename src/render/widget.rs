use crossterm::event::KeyEvent;
use std::sync::Arc;

/// A trait for interactive terminal widgets using a reactive, snapshot-based model.
///
/// # Architecture & Key Principles
///
/// ### 1. State Isolation & Unidirectional Data Flow
///
/// The widget **never mutates** `State`. Application state is fully owned by the caller/framework.
/// During `render_frame`, the widget receives read-only snapshots (`Arc<Self::State>`).
///
/// * **Why:** Prevents lock contention and deadlocks during rendering loops.
/// * **Data Change Tracking:** The rendering engine tracks `State` updates externally.
///   Custom event handlers block the rendering loop during execution, so state changes
///   are naturally observed by the host engine without widget involvement.
///
/// ### 2. Widget-Local Lifecycle (`is_changed` & `is_finished`)
///
/// The `is_changed` and `is_finished` methods signal the **internal operational status of the widget itself**,
/// independent of user state:
///
/// * `is_changed()`: Returns `true` only if internal widget mechanics (e.g., local animations, cursor blinking)
///   require a repaint. If the widget relies purely on user state, returning `false` is optimal.
/// * `is_finished()`: Signals completion of internal processing. If no background widget logic is running,
///   it safely defaults to `true`.
///
/// ### 3. Non-Destructive Output Extraction
///
/// `extract_output(&mut self)` takes `&mut self` instead of consuming `self`.
///
/// * **Why:** Allows the widget instance to remain active in memory (e.g., rendered statically in terminal history)
///   while still returning its final result to the host application.
pub trait Widget {
    /// External state type owned by the application (read-only snapshot source).
    type State: Default + Clone + Sync + Send + 'static;

    /// The result type emitted when the widget completes its lifecycle.
    type Output;

    /// Internal event type dispatched via `Context::send_event`.
    type Event: Send + 'static;

    /// Indicates whether *internal widget-local state* changed and requires a redraw.
    ///
    /// User `State` updates are tracked externally by the renderer, so widgets without
    /// internal visual logic (e.g., animations) should return `false`.
    fn is_changed(&self) -> bool {
        false
    }

    /// Indicates whether the internal processing of the widget is complete.
    ///
    /// Custom handlers run synchronously before frame evaluation, so widgets without
    /// complex background logic can safely return `true`.
    fn is_finished(&self) -> bool {
        true
    }

    /// Renders a frame using the latest immutable snapshot of `State`.
    ///
    /// # Arguments
    /// * `state` - Read-only snapshot of the shared state.
    /// * `max_width` - Optional horizontal bounding constraint.
    /// * `max_height` - Optional vertical bounding constraint.
    /// * `is_final` - `true` if this is the final render pass before termination.
    fn render_frame(
        &mut self,
        state: &Arc<Self::State>,
        max_width: Option<usize>,
        max_height: Option<usize>,
        is_final: bool,
    ) -> Vec<String>;

    /// Handles raw keyboard input events from the terminal.
    fn handle_key(&mut self, _key: KeyEvent) {}

    /// Handles custom domain events emitted from background tasks or `Context`.
    fn handle_event(&mut self, _event: Self::Event) {}

    /// Responds to terminal or container resize notifications.
    fn on_resize(&mut self, _cols: u16, _rows: u16) {}

    /// Returns relative cursor coordinates `(column, row)` within the widget, if active.
    fn cursor_position(&self) -> Option<(usize, usize)> {
        None
    }

    /// Extracts the execution result without consuming `self`.
    ///
    /// Allows the widget instance to persist in memory (e.g., inside terminal history)
    /// while yielding its output to the engine.
    fn extract_output(&mut self) -> Self::Output;
}
