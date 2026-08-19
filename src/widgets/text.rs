use crate::{
    render::{ansi, block::Block, widget::Widget},
    style::{LineStyle, SpinnerStyle, StripeStyle},
};
use crossterm::event::{KeyCode, KeyEvent};
use crossterm::style::{Color, Stylize};
use std::sync::{
    Arc, RwLock,
    atomic::{AtomicBool, Ordering},
};
use tokio::sync::{Notify, mpsc};

#[cfg(feature = "highlight")]
use crate::theme::CodeTheme;
#[cfg(feature = "markdown")]
use crate::{render::markdown::Markdown, style::BulletStyle};

/// returns the visual character representing the specified stripe style.
pub(crate) fn get_stripe_char(style: StripeStyle) -> Option<char> {
    match style {
        StripeStyle::None => None,
        StripeStyle::Single => Some('│'),
        StripeStyle::Double => Some('║'),
        StripeStyle::Thick => Some('▌'),
        StripeStyle::Dotted => Some('┊'),
        StripeStyle::Custom(ch) => Some(ch),
    }
}

/// A handle for sending text updates to the associated [`Text`] widget.
#[derive(Clone)]
pub struct UpdateHandle {
    /// Unbounded channel sender used to dispatch state updates to the background worker.
    sender: mpsc::UnboundedSender<String>,
}

impl UpdateHandle {
    /// sends a new text update to the active spinner widget.
    pub fn update(&self, text: impl Into<String>) {
        let _ = self.sender.send(text.into());
    }
}

/// A dynamic terminal UI component that displays an optional spinner alongside updating text.
pub struct Text {
    /// The static prefix specified during creation (for example, "Upload: ").
    pub(crate) static_prefix: String,
    /// Shared text state rendered next to the spinner.
    pub(crate) state: Arc<RwLock<String>>,
    /// Flag indicating whether the underlying asynchronous task has finished or if no task is attached.
    pub(crate) is_done: Arc<RwLock<bool>>,
    /// Current index within the spinner frame array.
    pub(crate) frame_idx: Arc<RwLock<usize>>,
    /// Notification handle used to signal completion to dynamic renderers.
    pub(crate) notify: Arc<Notify>,
    /// Visual style of the spinner animation.
    pub(crate) spinner_style: SpinnerStyle,
    /// Optional custom color applied to the spinner animation icon.
    pub(crate) spinner_color: Option<Color>,

    /// Visual style of the vertical stripe displayed alongside the prefix.
    pub(crate) prefix_stripe: StripeStyle,
    /// Visual style of the horizontal line underneath the prefix.
    pub(crate) prefix_line: LineStyle,
    /// Optional color applied to the prefix vertical stripe & underline.
    pub(crate) prefix_color: Option<Color>,
    /// Number of empty lines separating prefix/line from content.
    pub(crate) prefix_margin: usize,

    /// Current vertical scroll offset when content exceeds available height.
    pub(crate) scroll_offset: Arc<RwLock<usize>>,

    /// Atomic flag indicating content mutations to skip redundant redraws.
    pub(crate) is_changed: Arc<AtomicBool>,

    /// Configuration for Markdown parsing and rendering.
    #[cfg(feature = "markdown")]
    pub(crate) markdown: Markdown,
}

impl Text {
    /// creates a new [`Text`] widget wrapped in a UI block.
    pub fn new(static_prefix: impl Into<String>) -> Block<Self> {
        let static_prefix = static_prefix.into();
        let state = Default::default();
        let is_done = Arc::new(RwLock::new(true));
        let frame_idx = Arc::new(RwLock::new(0));
        let notify = Arc::new(Notify::new());
        let scroll_offset = Arc::new(RwLock::new(0));

        notify.notify_one();

        Block::new(Self {
            static_prefix,
            state,
            is_done,
            frame_idx,
            notify,
            spinner_style: SpinnerStyle::Dots,
            spinner_color: Some(Color::Cyan),

            prefix_stripe: StripeStyle::None,
            prefix_line: LineStyle::default(),
            prefix_color: None,
            prefix_margin: 1,

            scroll_offset,

            is_changed: Arc::new(AtomicBool::new(true)),

            #[cfg(feature = "markdown")]
            markdown: Markdown::default(),
        })
    }
}

impl Block<Text> {
    /// sets the style of the horizontal separator line underneath the prefix.
    pub fn prefix_line(mut self, style: LineStyle) -> Self {
        self.inner.prefix_line = style;
        self
    }

    /// sets the style of the vertical side stripe next to the prefix.
    pub fn prefix_stripe(mut self, style: StripeStyle) -> Self {
        self.inner.prefix_stripe = style;
        self
    }

    /// sets the color of the horizontal separator line underneath the prefix.
    pub fn prefix_color(mut self, color: Color) -> Self {
        self.inner.prefix_color = Some(color);
        self
    }

    /// sets the vertical margin (in empty lines) below the prefix / prefix line before dynamic content.
    pub fn prefix_margin(mut self, margin: usize) -> Self {
        self.inner.prefix_margin = margin;
        self
    }

    /// attaches an asynchronous background task that streams text updates to the widget.
    pub fn handler<F, Fut>(mut self, task: F) -> Self
    where
        F: FnOnce(UpdateHandle) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        self.inner.notify = Arc::new(Notify::new());

        if let Ok(mut lock) = self.inner.is_done.write() {
            *lock = false;
        }

        let state_writer = Arc::clone(&self.inner.state);
        let is_done_writer = Arc::clone(&self.inner.is_done);
        let notify_writer = Arc::clone(&self.inner.notify);
        let is_changed_writer = Arc::clone(&self.inner.is_changed);

        let (tx, mut rx) = mpsc::unbounded_channel::<String>();

        tokio::spawn(async move {
            let handle = UpdateHandle { sender: tx };
            task(handle).await;
        });

        tokio::spawn(async move {
            while let Some(new_text) = rx.recv().await {
                if let Ok(mut lock) = state_writer.write() {
                    *lock = new_text;
                    is_changed_writer.store(true, Ordering::Release);
                }
            }
            if let Ok(mut lock) = is_done_writer.write() {
                *lock = true;
                is_changed_writer.store(true, Ordering::Release);
            }
            notify_writer.notify_one();
        });

        self
    }

    /// applies a unified color theme to the spinner and all Markdown elements.
    #[cfg(feature = "markdown")]
    pub fn color(mut self, color: Color) -> Self {
        self.inner.spinner_color = Some(color);
        self.inner.markdown = self
            .inner
            .markdown
            .stripe_color(color)
            .bullet_color(color)
            .code_color(color);
        self
    }

    // --- Spinner Styling Methods ---

    /// sets the visual style of the spinner animation.
    pub fn spinner_style(mut self, style: SpinnerStyle) -> Self {
        self.inner.spinner_style = style;
        self
    }

    /// sets the color of the spinner animation.
    pub fn spinner_color(mut self, color: Color) -> Self {
        self.inner.spinner_color = Some(color);
        self
    }

    // --- Markdown Styling Methods ---

    /// sets the stripe style for Markdown blockquotes.
    #[cfg(feature = "markdown")]
    pub fn stripe_style(mut self, style: StripeStyle) -> Self {
        self.inner.markdown = self.inner.markdown.stripe_style(style);
        self
    }

    /// sets the stripe color for Markdown blockquotes.
    #[cfg(feature = "markdown")]
    pub fn stripe_color(mut self, color: Color) -> Self {
        self.inner.markdown = self.inner.markdown.stripe_color(color);
        self
    }

    /// sets the bullet style for Markdown lists.
    #[cfg(feature = "markdown")]
    pub fn bullet_style(mut self, style: BulletStyle) -> Self {
        self.inner.markdown = self.inner.markdown.bullet_style(style);
        self
    }

    /// sets the bullet color for Markdown lists.
    #[cfg(feature = "markdown")]
    pub fn bullet_color(mut self, color: Color) -> Self {
        self.inner.markdown = self.inner.markdown.bullet_color(color);
        self
    }

    /// sets the color for inline code blocks in Markdown.
    #[cfg(feature = "markdown")]
    pub fn code_color(mut self, color: Color) -> Self {
        self.inner.markdown = self.inner.markdown.code_color(color);
        self
    }

    /// sets the syntax highlighting theme for code blocks in Markdown.
    #[cfg(all(feature = "markdown", feature = "highlight"))]
    pub fn code_theme(mut self, theme: CodeTheme) -> Self {
        self.inner.markdown = self.inner.markdown.theme(theme);
        self
    }
}

impl Widget for Text {
    type Output = String;

    /// Checks whether the widget needs to be re-rendered on screen.
    fn is_changed(&self) -> bool {
        // 1. Content, scroll, or window resize changes
        if self.is_changed.load(Ordering::Acquire) {
            return true;
        }

        // 2. If spinner is active — a frame is required to advance frame_idx
        let is_done = *self.is_done.read().unwrap_or_else(|e| e.into_inner());
        !is_done && self.spinner_style != SpinnerStyle::None
    }

    /// Handles terminal window resize events by signaling a state change.
    fn on_resize(&mut self, _rows: u16, _cols: u16) {
        self.is_changed.store(true, Ordering::Release);
    }

    /// Renders the prefix, dynamic content, and spinner animation into terminal line strings.
    fn render_content(
        &mut self,
        max_width: Option<usize>,
        _max_height: Option<usize>,
    ) -> Vec<String> {
        // reset change flag during actual rendering
        self.is_changed.store(false, Ordering::Release);

        let is_done = *self.is_done.read().unwrap_or_else(|e| e.into_inner());
        let spinner_active = !is_done && self.spinner_style != SpinnerStyle::None;

        let raw_dynamic_text = self
            .state
            .read()
            .map(|s| s.clone())
            .unwrap_or_else(|e| e.into_inner().clone());

        // construct spinner icon string
        let spinner_str = if spinner_active {
            let frames = self.spinner_style.frames();
            if frames.is_empty() {
                String::new()
            } else {
                let mut frame_idx = self.frame_idx.write().unwrap_or_else(|e| e.into_inner());
                let raw_icon = frames[*frame_idx % frames.len()];
                *frame_idx = frame_idx.wrapping_add(1);

                let styled_icon = if let Some(color) = self.spinner_color {
                    raw_icon.with(color).to_string()
                } else {
                    raw_icon.to_string()
                };

                format!("{} ", styled_icon)
            }
        } else {
            String::new()
        };

        let spinner_width = ansi::visible_width(&spinner_str);
        let spinner_indent = " ".repeat(spinner_width);

        let mut lines = Vec::new();
        let mut prefix_max_width = 0;
        let mut has_prefix_content = false;

        // a helper function for generating a prefix strip for empty/indented strings.
        let get_sideline_prefix = || {
            if let Some(ch) = get_stripe_char(self.prefix_stripe) {
                let s = format!("{} ", ch);
                let color = self.prefix_color.or(self.spinner_color);
                if let Some(c) = color {
                    s.with(c).to_string()
                } else {
                    s
                }
            } else {
                String::new()
            }
        };

        // 1. Render static prefix (plain text)
        if !self.static_prefix.is_empty() {
            let sideline_prefix = get_sideline_prefix();
            let sideline_w = ansi::visible_width(&sideline_prefix);

            let prefix_lines: Vec<String> = match max_width.map(|w| w.saturating_sub(sideline_w)) {
                Some(w) if w > 0 => {
                    let normalized = self
                        .static_prefix
                        .replace("\r\n", "\n")
                        .replace('\r', "\n")
                        .replace('\t', "    ");
                    let mut acc = Vec::new();
                    let mut active_ansi = String::new();

                    for line in normalized.split('\n') {
                        if line.is_empty() {
                            acc.push(active_ansi.clone());
                        } else {
                            let line_with_color = format!("{}{}", active_ansi, line);
                            let wrapped = ansi::wrap_terminal_text(&line_with_color, w);
                            if let Some(last_line) = wrapped.last() {
                                active_ansi =
                                    ansi::get_active_text_color(last_line).unwrap_or_default();
                            }
                            acc.extend(wrapped);
                        }
                    }
                    acc
                }
                _ => self.static_prefix.lines().map(|s| s.to_string()).collect(),
            };

            if !prefix_lines.is_empty() {
                has_prefix_content = true;
            }

            for p_line in prefix_lines {
                let full_line = format!("{}{}\x1b[0m", sideline_prefix, p_line);
                let vis_w = ansi::visible_width(&full_line);
                if vis_w > prefix_max_width {
                    prefix_max_width = vis_w;
                }
                lines.push(full_line);
            }
        }

        // 1.5. Render separator line
        if self.prefix_line != LineStyle::None
            && !self.static_prefix.is_empty()
            && prefix_max_width > 0
        {
            has_prefix_content = true;
            let line_symbol = self.prefix_line.as_char();

            // if max_width is specified, limit the length of the delimiter; otherwise, use the width of the prefix.
            let underline_len = match max_width {
                Some(w) => prefix_max_width.min(w),
                None => prefix_max_width,
            };

            let line_str = if get_stripe_char(self.prefix_stripe).is_some() {
                let sideline_prefix = get_sideline_prefix();
                let sideline_w = ansi::visible_width(&sideline_prefix);
                let remaining_len = underline_len.saturating_sub(sideline_w);

                let mut repeated = line_symbol.to_string().repeat(remaining_len);
                let line_color = self.prefix_color.or(self.spinner_color);
                if let Some(color) = line_color {
                    repeated = repeated.with(color).dim().to_string();
                }
                format!("{}{}", sideline_prefix, repeated)
            } else {
                let mut repeated = line_symbol.to_string().repeat(underline_len);
                let line_color = self.prefix_color.or(self.spinner_color);
                if let Some(color) = line_color {
                    repeated = repeated.with(color).dim().to_string();
                }
                repeated
            };

            lines.push(line_str);
        }

        // 1.8. Render prefix_margin
        if has_prefix_content && self.prefix_margin > 0 {
            let margin_line = get_sideline_prefix();
            for _ in 0..self.prefix_margin {
                lines.push(margin_line.clone());
            }
        }

        // 2. Render spinner and dynamic text
        let content_lines: Vec<String> = match max_width {
            Some(w) => {
                let available_content_width = w.saturating_sub(spinner_width);
                if available_content_width == 0 {
                    Vec::new()
                } else {
                    #[cfg(feature = "markdown")]
                    {
                        let rendered = self
                            .markdown
                            .render(&raw_dynamic_text, available_content_width);
                        let trimmed = rendered.trim_end_matches(|c| c == '\r' || c == '\n');
                        if trimmed.is_empty() {
                            Vec::new()
                        } else {
                            trimmed.lines().map(|s| s.to_string()).collect()
                        }
                    }
                    #[cfg(not(feature = "markdown"))]
                    {
                        ansi::wrap_terminal_text(&raw_dynamic_text, available_content_width)
                    }
                }
            }
            None => {
                // max_width == None: don’t wrap the text to fit the width; simply split it at line breaks \n
                raw_dynamic_text.lines().map(|s| s.to_string()).collect()
            }
        };

        let mut dyn_rendered_lines = Vec::new();
        for (idx, line) in content_lines.into_iter().enumerate() {
            if idx == 0 {
                dyn_rendered_lines.push(format!("{}{}\x1b[0m", spinner_str, line));
            } else {
                dyn_rendered_lines.push(format!("{}{}\x1b[0m", spinner_indent, line));
            }
        }

        if dyn_rendered_lines.is_empty() {
            dyn_rendered_lines.push(spinner_str);
        }

        lines.extend(dyn_rendered_lines);

        while lines
            .last()
            .map_or(false, |line| ansi::visible_width(line) == 0)
        {
            lines.pop();
        }

        lines
    }

    /// Processes keyboard navigation events for scrolling the content.
    fn handle_key(&mut self, key: KeyEvent) {
        let mut scroll = self
            .scroll_offset
            .write()
            .unwrap_or_else(|e| e.into_inner());
        let mut moved = false;

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if *scroll > 0 {
                    *scroll -= 1;
                    moved = true;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                *scroll += 1;
                moved = true;
            }
            KeyCode::PageUp => {
                if *scroll > 0 {
                    *scroll = scroll.saturating_sub(5);
                    moved = true;
                }
            }
            KeyCode::PageDown => {
                *scroll += 5;
                moved = true;
            }
            _ => {}
        }

        if moved {
            self.is_changed.store(true, Ordering::Release);
        }
    }

    /// returns whether the background execution task has completed.
    fn is_finished(&self) -> bool {
        *self.is_done.read().unwrap_or_else(|e| e.into_inner())
    }

    /// extracts the final string payload stored inside the widget.
    fn extract_output(self) -> Self::Output {
        self.state
            .read()
            .map(|s| s.clone())
            .unwrap_or_else(|e| e.into_inner().clone())
    }

    /// indicates whether the terminal cursor should be visible during rendering.
    fn show_cursor(&self) -> bool {
        false
    }
}
