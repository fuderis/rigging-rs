use crate::render::{ansi, block::Block, widget::Widget};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::mpsc;

/// Represents the internal synchronized state of a progress bar.
#[derive(Debug, Clone)]
pub struct ProgressState {
    /// Current completion value.
    pub current: usize,
    /// Target total value.
    pub total: usize,
    /// Optional status or descriptive text label.
    pub label: Option<String>,
}

/// Operations for dynamically updating the progress state from external channels.
pub enum ProgressOp {
    /// Update current progress value.
    SetCurrent(usize),
    /// Update current progress value and status label.
    SetCurrentWithLabel(usize, String),
    /// Dynamically adjust the total target value if needed.
    SetTotal(usize),
}

/// A thread-safe handle used to trigger progress updates from background tasks.
pub struct ProgressHandle {
    sender: mpsc::UnboundedSender<ProgressOp>,
}

impl ProgressHandle {
    /// Updates only the current progress value.
    pub fn update(&self, current: usize) {
        let _ = self.sender.send(ProgressOp::SetCurrent(current));
    }

    /// Updates current progress value and replaces the status label.
    pub fn update_with_label(&self, current: usize, label: impl Into<String>) {
        let _ = self
            .sender
            .send(ProgressOp::SetCurrentWithLabel(current, label.into()));
    }

    /// Dynamically updates the total progress limit.
    pub fn set_total(&self, total: usize) {
        let _ = self.sender.send(ProgressOp::SetTotal(total));
    }
}

/// A customizable progress bar widget supporting dynamic async updates.
pub struct ProgressBar {
    /// Shared state containing current progress metrics.
    pub(crate) state: Arc<Mutex<ProgressState>>,
    /// Character representing the filled portion of the bar.
    pub(crate) filled_char: char,
    /// Character representing the remaining unfilled portion.
    pub(crate) empty_char: char,
    /// Flag indicating whether percentage output should be rendered.
    pub(crate) show_percentage: bool,
    /// Atomic flag indicating whether the background worker has finished execution.
    pub(crate) is_finished: Arc<AtomicBool>,
    /// Atomic flag indicating whether state was updated and requires a re-render.
    pub(crate) is_changed: Arc<AtomicBool>,
}

impl ProgressBar {
    /// Creates a new `ProgressBar` builder wrapped in a `Block`.
    pub fn new(current: usize, total: usize) -> Block<Self> {
        Block::new(Self {
            state: Arc::new(Mutex::new(ProgressState {
                current,
                total,
                label: None,
            })),
            filled_char: '█',
            empty_char: '░',
            show_percentage: true,
            // Defaults to finished for static usage without a background task
            is_finished: Arc::new(AtomicBool::new(true)),
            // Initial render is required
            is_changed: Arc::new(AtomicBool::new(true)),
        })
    }
}

impl Block<ProgressBar> {
    /// Attaches an asynchronous handler function to execute background operations for the progress bar.
    pub fn handler<F, Fut>(mut self, task: F) -> Self
    where
        F: FnOnce(ProgressHandle) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let is_finished = Arc::new(AtomicBool::new(false));
        let finished_flag = Arc::clone(&is_finished);

        let is_changed = Arc::new(AtomicBool::new(true));
        let changed_flag = Arc::clone(&is_changed);

        let (tx, mut rx) = mpsc::unbounded_channel::<ProgressOp>();

        // Spawn user-defined execution task
        tokio::spawn(async move {
            let handle = ProgressHandle { sender: tx };
            task(handle).await;
        });

        // Spawn state-synchronization loop
        let state_writer = Arc::clone(&self.inner.state);
        tokio::spawn(async move {
            while let Some(op) = rx.recv().await {
                if let Ok(mut lock) = state_writer.lock() {
                    match op {
                        ProgressOp::SetCurrent(curr) => {
                            lock.current = curr;
                        }
                        ProgressOp::SetCurrentWithLabel(curr, label) => {
                            lock.current = curr;
                            lock.label = Some(label);
                        }
                        ProgressOp::SetTotal(tot) => {
                            lock.total = tot;
                        }
                    }
                    changed_flag.store(true, Ordering::SeqCst);
                }
            }
            finished_flag.store(true, Ordering::SeqCst);
            changed_flag.store(true, Ordering::SeqCst);
        });

        self.inner.is_finished = is_finished;
        self.inner.is_changed = is_changed;
        self
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
            self.inner.is_changed.store(true, Ordering::SeqCst);
        }
        self
    }
}

impl Widget for ProgressBar {
    type Output = ();

    fn is_changed(&self) -> bool {
        self.is_changed.load(Ordering::SeqCst)
    }

    fn render_content(
        &mut self,
        max_width: Option<usize>,
        _max_height: Option<usize>,
    ) -> Vec<String> {
        // Reset dirty flag upon frame rendering
        self.is_changed.store(false, Ordering::SeqCst);

        let width = max_width.unwrap_or(80);

        if width == 0 {
            return vec![String::new()];
        }

        // Safely retrieve state snapshot
        let (current, total, label) = {
            let lock = self.state.lock().unwrap();
            (lock.current, lock.total, lock.label.clone())
        };

        // Calculate completion ratio
        let ratio = if total == 0 {
            1.0
        } else {
            (current as f64 / total as f64).clamp(0.0, 1.0)
        };

        // 1. Prepare prefix (Label on the left)
        let prefix = match label {
            Some(ref l) if !l.is_empty() => format!("{} ", l),
            _ => String::new(),
        };

        // 2. Prepare suffix (Percentage on the right)
        let suffix = if self.show_percentage {
            let percent = (ratio * 100.0) as usize;
            format!(" {:>3}%", percent)
        } else {
            String::new()
        };

        let prefix_width = ansi::visible_width(&prefix);
        let suffix_width = ansi::visible_width(&suffix);
        let bar_width = width.saturating_sub(prefix_width + suffix_width);

        // Fallback if render space is too small for the progress bar itself
        if bar_width == 0 {
            return vec![format!("{}{}\x1b[0m", prefix, suffix)];
        }

        // 3. Compute bar fill segment lengths
        let filled_len = (bar_width as f64 * ratio).round() as usize;
        let empty_len = bar_width.saturating_sub(filled_len);

        let bar: String = std::iter::repeat(self.filled_char)
            .take(filled_len)
            .chain(std::iter::repeat(self.empty_char).take(empty_len))
            .collect();

        // Layout: [Prefix/Label] [ProgressBar] [Suffix/Percentage]
        vec![format!("{}{}{}\x1b[0m", prefix, bar, suffix)]
    }

    fn is_finished(&self) -> bool {
        self.is_finished.load(Ordering::SeqCst)
    }

    fn extract_output(self) -> Self::Output {}
}
