use crate::{
    render::{block::Block, widget::Widget},
    style::{LineStyle, SpinnerStyle, StripeStyle},
    utils::ansi,
};

use crossterm::event::{KeyCode, KeyEvent};
use crossterm::style::{Color, Stylize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(feature = "highlight")]
use crate::theme::CodeTheme;
#[cfg(feature = "markdown")]
use crate::{render::markdown::Markdown, style::BulletStyle};

/// Returns the visual character representing the specified stripe style.
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

/// A dynamic terminal UI component that displays an optional spinner alongside updating user state.
pub struct Text {
    /// Static prefix (e.g., "Upload: ").
    pub(crate) static_prefix: String,

    /// Visual style of the spinner animation.
    pub(crate) spinner_style: SpinnerStyle,
    /// Current frame index of the spinner.
    pub(crate) frame_idx: usize,
    /// Optional custom color applied to the spinner icon.
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
    pub(crate) scroll_offset: usize,

    /// Atomic flag indicating content/layout changes to trigger redraws.
    pub(crate) is_changed: AtomicBool,

    /// Configuration for Markdown parsing and rendering.
    #[cfg(feature = "markdown")]
    pub(crate) markdown: Option<Markdown>,
}

impl Text {
    /// Creates a new [`Text`] widget wrapped in a [`Block`] with user string state.
    pub fn new(static_prefix: impl Into<String>) -> Block<Self> {
        Block::new(
            Self {
                static_prefix: static_prefix.into(),
                spinner_style: SpinnerStyle::Dots,
                frame_idx: 0,
                spinner_color: Some(Color::Cyan),

                prefix_stripe: StripeStyle::None,
                prefix_line: LineStyle::default(),
                prefix_color: None,
                prefix_margin: 1,

                scroll_offset: 0,
                is_changed: AtomicBool::new(true),

                #[cfg(feature = "markdown")]
                markdown: Some(Markdown::default()),
            },
            String::new(),
        )
    }
}

impl Block<Text> {
    /// Sets the style of the horizontal separator line underneath the prefix.
    pub fn prefix_line(mut self, style: LineStyle) -> Self {
        self.inner.prefix_line = style;
        self
    }

    /// Sets the style of the vertical side stripe next to the prefix.
    pub fn prefix_stripe(mut self, style: StripeStyle) -> Self {
        self.inner.prefix_stripe = style;
        self
    }

    /// Sets the color of the horizontal separator line underneath the prefix.
    pub fn prefix_color(mut self, color: Color) -> Self {
        self.inner.prefix_color = Some(color);
        self
    }

    /// Sets the vertical margin (in empty lines) below the prefix line.
    pub fn prefix_margin(mut self, margin: usize) -> Self {
        self.inner.prefix_margin = margin;
        self
    }

    /// Sets the visual style of the spinner animation.
    pub fn spinner_style(mut self, style: SpinnerStyle) -> Self {
        self.inner.spinner_style = style;
        self
    }

    /// Sets the color of the spinner animation.
    pub fn spinner_color(mut self, color: Color) -> Self {
        self.inner.spinner_color = Some(color);
        self
    }

    #[cfg(feature = "markdown")]
    pub fn markdown(mut self, enable: bool) -> Self {
        if enable && self.inner.markdown.is_none() {
            self.inner.markdown = Some(Markdown::default());
        }
        self
    }

    #[cfg(feature = "markdown")]
    pub fn accent_color(mut self, color: Color) -> Self {
        if let Some(md) = self.inner.markdown {
            self.inner.markdown =
                Some(md.stripe_color(color).bullet_color(color).code_color(color));
        }
        self.inner.spinner_color = Some(color);
        self
    }

    #[cfg(feature = "markdown")]
    pub fn stripe_style(mut self, style: StripeStyle) -> Self {
        let md = self.inner.markdown.unwrap_or_default();
        self.inner.markdown = Some(md.stripe_style(style));
        self
    }

    #[cfg(feature = "markdown")]
    pub fn stripe_color(mut self, color: Color) -> Self {
        let md = self.inner.markdown.unwrap_or_default();
        self.inner.markdown = Some(md.stripe_color(color));
        self
    }

    #[cfg(feature = "markdown")]
    pub fn bullet_style(mut self, style: BulletStyle) -> Self {
        let md = self.inner.markdown.unwrap_or_default();
        self.inner.markdown = Some(md.bullet_style(style));
        self
    }

    #[cfg(feature = "markdown")]
    pub fn bullet_color(mut self, color: Color) -> Self {
        let md = self.inner.markdown.unwrap_or_default();
        self.inner.markdown = Some(md.bullet_color(color));
        self
    }

    #[cfg(feature = "markdown")]
    pub fn code_color(mut self, color: Color) -> Self {
        let md = self.inner.markdown.unwrap_or_default();
        self.inner.markdown = Some(md.code_color(color));
        self
    }

    #[cfg(all(feature = "markdown", feature = "highlight"))]
    pub fn code_theme(mut self, theme: CodeTheme) -> Self {
        let md = self.inner.markdown.unwrap_or_default();
        self.inner.markdown = Some(md.theme(theme));
        self
    }
}

impl Widget for Text {
    type State = String;
    type Output = ();
    type Event = ();

    fn on_resize(&mut self, _cols: u16, _rows: u16) {
        self.is_changed.store(true, Ordering::Release);
    }

    fn render_frame(
        &mut self,
        state: &Arc<Self::State>,
        max_width: Option<usize>,
        _max_height: Option<usize>,
        is_final: bool,
    ) -> Vec<String> {
        self.is_changed.store(false, Ordering::Release);

        let raw_dynamic_text = state.as_str();

        let spinner_str = if is_final {
            String::new()
        } else if self.spinner_style != SpinnerStyle::None {
            let frames = self.spinner_style.frames();
            if frames.is_empty() {
                String::new()
            } else {
                let raw_icon = frames[self.frame_idx % frames.len()];
                self.frame_idx = self.frame_idx.wrapping_add(1);

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

        // 1. Static Prefix
        if !self.static_prefix.is_empty() {
            let sideline_prefix = get_sideline_prefix();
            let sideline_w = ansi::visible_width(&sideline_prefix);

            let normalized = self
                .static_prefix
                .replace("\r\n", "\n")
                .replace('\r', "\n")
                .replace('\t', "    ");

            let target_width = max_width.map(|w| w.saturating_sub(sideline_w));
            let mut prefix_lines = Vec::new();
            let mut active_ansi = String::new();

            for raw_line in normalized.split('\n') {
                let line_with_color = format!("{}{}", active_ansi, raw_line);
                let wrapped_sublines = match target_width {
                    Some(w) if w > 0 => ansi::wrap_terminal_text(&line_with_color, w),
                    _ => vec![line_with_color],
                };

                for subline in wrapped_sublines {
                    if !subline.is_empty() {
                        if let Some(color) = ansi::get_active_text_color(&subline) {
                            active_ansi = color;
                        }
                        prefix_lines.push(subline);
                    } else {
                        prefix_lines.push(active_ansi.clone());
                    }
                }
            }

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

        // 1.5. Prefix Line
        if self.prefix_line != LineStyle::None
            && !self.static_prefix.is_empty()
            && prefix_max_width > 0
        {
            has_prefix_content = true;
            let line_symbol = self.prefix_line.as_char();

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

        // 1.8. Prefix Margin
        if has_prefix_content && self.prefix_margin > 0 {
            let margin_line = get_sideline_prefix();
            for _ in 0..self.prefix_margin {
                lines.push(margin_line.clone());
            }
        }

        // 2. Dynamic Content
        let content_lines: Vec<String> = match max_width {
            Some(w) => {
                let available_content_width = w.saturating_sub(spinner_width);
                if available_content_width == 0 {
                    Vec::new()
                } else {
                    #[cfg(feature = "markdown")]
                    {
                        if let Some(md) = &self.markdown {
                            let rendered = md.render(raw_dynamic_text, available_content_width);
                            let trimmed = rendered.trim_end_matches(|c| c == '\r' || c == '\n');
                            if trimmed.is_empty() {
                                Vec::new()
                            } else {
                                trimmed.lines().map(|s| s.to_string()).collect()
                            }
                        } else {
                            ansi::wrap_terminal_text(raw_dynamic_text, available_content_width)
                        }
                    }
                    #[cfg(not(feature = "markdown"))]
                    {
                        ansi::wrap_terminal_text(raw_dynamic_text, available_content_width)
                    }
                }
            }
            None => raw_dynamic_text.lines().map(|s| s.to_string()).collect(),
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

        if self.scroll_offset > 0 && self.scroll_offset < lines.len() {
            lines.drain(0..self.scroll_offset);
        }

        lines
    }

    fn handle_key(&mut self, key: KeyEvent) {
        let mut moved = false;

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.scroll_offset > 0 {
                    self.scroll_offset -= 1;
                    moved = true;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.scroll_offset += 1;
                moved = true;
            }
            KeyCode::PageUp => {
                if self.scroll_offset > 0 {
                    self.scroll_offset = self.scroll_offset.saturating_sub(5);
                    moved = true;
                }
            }
            KeyCode::PageDown => {
                self.scroll_offset += 5;
                moved = true;
            }
            _ => {}
        }

        if moved {
            self.is_changed.store(true, Ordering::Release);
        }
    }

    fn extract_output(&mut self) -> Self::Output {
        ()
    }

    fn is_changed(&self) -> bool {
        self.spinner_style != SpinnerStyle::None
    }
}
