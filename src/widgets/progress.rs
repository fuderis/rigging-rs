use crate::{
    render::{block::Block, widget::Widget},
    utils::ansi,
};
use std::sync::Arc;

/// Progress status.
#[derive(Debug, Clone, Default)]
pub struct Progress {
    /// Current value.
    pub current: usize,
    /// Total value.
    pub total: usize,
    /// Text label.
    pub label: Option<String>,
}

/// Progress bar widget.
pub struct ProgressBar {
    pub(crate) filled_char: char,
    pub(crate) empty_char: char,
    pub(crate) show_percentage: bool,
}

impl ProgressBar {
    pub fn new(current: usize, total: usize) -> Block<Self> {
        Block::new(
            Self {
                filled_char: '█',
                empty_char: '░',
                show_percentage: true,
            },
            Progress {
                current,
                total,
                label: None,
            },
        )
    }

    pub fn new_with_label(current: usize, total: usize, label: impl Into<String>) -> Block<Self> {
        Block::new(
            Self {
                filled_char: '█',
                empty_char: '░',
                show_percentage: true,
            },
            Progress {
                current,
                total,
                label: Some(label.into()),
            },
        )
    }
}

impl Block<ProgressBar> {
    pub fn filled_char(mut self, ch: char) -> Self {
        self.inner.filled_char = ch;
        self
    }

    pub fn empty_char(mut self, ch: char) -> Self {
        self.inner.empty_char = ch;
        self
    }

    pub fn show_percentage(mut self, show: bool) -> Self {
        self.inner.show_percentage = show;
        self
    }
}

impl Widget for ProgressBar {
    type State = Progress;
    type Output = ();
    type Event = ();

    fn is_changed(&self) -> bool {
        false
    }

    fn is_finished(&self) -> bool {
        true
    }

    fn render_frame(
        &mut self,
        state: &Arc<Self::State>,
        max_width: Option<usize>,
        _max_height: Option<usize>,
        _is_final: bool,
    ) -> Vec<String> {
        let width = max_width.unwrap_or(80);

        if width == 0 {
            return vec![String::new()];
        }

        let current = state.current;
        let total = state.total;

        let ratio = if total == 0 {
            1.0
        } else {
            (current as f64 / total as f64).clamp(0.0, 1.0)
        };

        let prefix = match &state.label {
            Some(l) if !l.is_empty() => format!("{l} "),
            _ => String::new(),
        };

        let suffix = if self.show_percentage {
            let percent = (ratio * 100.0) as usize;
            format!(" {percent:>3}%")
        } else {
            String::new()
        };

        let prefix_width = ansi::visible_width(&prefix);
        let suffix_width = ansi::visible_width(&suffix);
        let bar_width = width.saturating_sub(prefix_width + suffix_width);

        if bar_width == 0 {
            return vec![format!("{prefix}{suffix}\x1b[0m")];
        }

        let filled_len = (bar_width as f64 * ratio).round() as usize;
        let empty_len = bar_width.saturating_sub(filled_len);

        let bar: String = std::iter::repeat(self.filled_char)
            .take(filled_len)
            .chain(std::iter::repeat(self.empty_char).take(empty_len))
            .collect();

        vec![format!("{prefix}{bar}{suffix}\x1b[0m")]
    }

    fn extract_output(&mut self) -> Self::Output {}
}
