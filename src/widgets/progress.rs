use crate::render::{
    block::Block,
    widget::{DynamicWidget, Widget},
};
use std::{
    future::Future,
    sync::{Arc, Mutex},
};
use tokio::sync::{Notify, mpsc};

/// Represents the internal state of a progress bar.
#[derive(Debug, Clone)]
pub struct ProgressState {
    /// Current completion value.
    pub current: usize,
    /// Target total value.
    pub total: usize,
    /// Optional status or descriptive text label.
    pub label: Option<String>,
}

/// Enum representing operations for updating the progress state.
pub enum ProgressOp {
    /// Set current and total progress values.
    Set(usize, usize),
    /// Set progress values along with a new label.
    SetWithLabel(usize, usize, String),
}

/// A thread-safe handle used to trigger progress updates from background tasks.
pub struct ProgressHandle {
    /// Channel sender for dispatching progress operations.
    sender: mpsc::UnboundedSender<ProgressOp>,
}

impl ProgressHandle {
    /// Updates the current and total progress values.
    pub fn update(&self, current: usize, total: usize) {
        // dispatch progress update command
        let _ = self.sender.send(ProgressOp::Set(current, total));
    }

    /// Updates current and total progress values along with a status label.
    pub fn update_with_label(&self, current: usize, total: usize, label: impl Into<String>) {
        // dispatch progress update command with custom label
        let _ = self
            .sender
            .send(ProgressOp::SetWithLabel(current, total, label.into()));
    }
}

/// A customizable progress bar widget supporting dynamic updates.
pub struct ProgressBar {
    /// Shared state containing current progress metrics.
    pub(crate) state: Arc<Mutex<ProgressState>>,
    /// Character representing the filled portion of the bar.
    pub(crate) filled_char: char,
    /// Character representing the remaining unfilled portion.
    pub(crate) empty_char: char,
    /// Flag indicating whether percentage output should be rendered.
    pub(crate) show_percentage: bool,
    /// Signal to notify when the associated background processing completes.
    pub(crate) done_signal: Arc<Notify>,
}

impl ProgressBar {
    /// Creates a new `ProgressBar` wrapped in a `Block`.
    pub fn new(current: usize, total: usize) -> Block<Self> {
        let done_signal = Arc::new(Notify::new());

        // notify immediately by default so static renders complete without waiting
        done_signal.notify_one();

        Block::new(Self {
            state: Arc::new(Mutex::new(ProgressState {
                current,
                total,
                label: None,
            })),
            filled_char: '█',
            empty_char: '░',
            show_percentage: true,
            done_signal,
        })
    }
}

impl Block<ProgressBar> {
    /// Attaches an asynchronous handler function to execute background operations for the progress bar.
    pub fn handler<F, Fut>(self, task: F) -> Self
    where
        F: FnOnce(ProgressHandle) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        // initialize completion signal for background task execution
        let done_signal = Arc::new(Notify::new());
        let done_notifier = Arc::clone(&done_signal);

        let (tx, mut rx) = mpsc::unbounded_channel::<ProgressOp>();

        // spawn user background execution task
        tokio::spawn(async move {
            let handle = ProgressHandle { sender: tx };
            task(handle).await;
        });

        // spawn background event loop for state sync
        let state_writer = Arc::clone(&self.inner.state);
        tokio::spawn(async move {
            while let Some(op) = rx.recv().await {
                if let Ok(mut lock) = state_writer.lock() {
                    match op {
                        ProgressOp::Set(curr, tot) => {
                            lock.current = curr;
                            lock.total = tot;
                        }
                        ProgressOp::SetWithLabel(curr, tot, label) => {
                            lock.current = curr;
                            lock.total = tot;
                            lock.label = Some(label);
                        }
                    }
                }
            }
            // notify render loop upon channel closure
            done_notifier.notify_one();
        });

        // assign active completion signal to block inner state
        let mut block = self;
        block.inner.done_signal = done_signal;
        block
    }

    /// Sets the character used for filled progress segments.
    pub fn filled_char(mut self, ch: char) -> Self {
        self.inner.filled_char = ch;
        self
    }

    /// Sets the character used for empty progress segments.
    pub fn empty_char(mut self, ch: char) -> Self {
        self.inner.empty_char = ch;
        self
    }

    /// Configures whether percentage text is visible.
    pub fn show_percentage(mut self, show: bool) -> Self {
        self.inner.show_percentage = show;
        self
    }

    /// Sets an initial text label for the progress bar.
    pub fn label(self, label: impl Into<String>) -> Self {
        if let Ok(mut lock) = self.inner.state.lock() {
            lock.label = Some(label.into());
        }
        self
    }
}

impl Widget for ProgressBar {
    type Output = ();

    /// Renders the visual representation of the progress bar based on available width.
    fn render_content(&self, width: usize) -> Vec<String> {
        if width == 0 {
            return vec![String::new()];
        }

        // safely retrieve state snapshot
        let lock = self.state.lock().unwrap();
        let current = lock.current;
        let total = lock.total;
        let label = lock.label.clone();
        drop(lock);

        // calculate completion ratio
        let ratio = if total == 0 {
            1.0
        } else {
            (current as f64 / total as f64).clamp(0.0, 1.0)
        };

        // assemble suffix content
        let mut suffix = String::new();
        if self.show_percentage {
            let percent = (ratio * 100.0) as usize;
            suffix.push_str(&format!(" {:>3}%", percent));
        }
        if let Some(ref l) = label {
            suffix.push_str(&format!(" {}", l));
        }

        let suffix_width = unicode_width::UnicodeWidthStr::width(suffix.as_str());
        let bar_width = width.saturating_sub(suffix_width);

        // fallback if render space is constrained to suffix only
        if bar_width == 0 {
            return vec![suffix];
        }

        // compute bar fill segment lengths
        let filled_len = (bar_width as f64 * ratio).round() as usize;
        let empty_len = bar_width.saturating_sub(filled_len);

        let bar: String = std::iter::repeat(self.filled_char)
            .take(filled_len)
            .chain(std::iter::repeat(self.empty_char).take(empty_len))
            .collect();

        vec![format!("{}{}", bar, suffix)]
    }
}

impl DynamicWidget for ProgressBar {
    /// Provides access to the completion signal handle.
    fn completion_signal(&self) -> Arc<Notify> {
        Arc::clone(&self.done_signal)
    }

    /// Extracts execution output upon widget finalization.
    fn extract_output(self) -> Self::Output {
        ()
    }
}
