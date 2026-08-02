pub use super::quote::StripeStyle;
use crate::render::{ansi, block::Block, widget::Widget};

#[cfg(feature = "highlight")]
use crate::{highlight::*, theme::CodeTheme};

use crossterm::style::{Color, Stylize};
use std::{fmt::Display, io};

/// A widget block that stores source code, language metadata, and styling information.
pub struct CodeBlock {
    /// Raw source code contents to display.
    code: String,
    /// Optional programming language name or file extension for highlighting.
    lang: Option<String>,
    /// Visual style of the side stripe/prefix.
    style: StripeStyle,
    /// Accent color applied to the side stripe/prefix.
    color: Option<Color>,
    /// Syntax highlighting theme configuration.
    #[cfg(feature = "highlight")]
    theme: CodeTheme,
}

impl CodeBlock {
    /// Creates a new `CodeBlock` instance wrapped in a generic `Block`.
    pub fn new(code: impl Into<String>) -> Block<Self> {
        Block::new(Self {
            code: code.into(),
            lang: None,
            style: StripeStyle::Single,
            color: Some(Color::DarkGrey),
            #[cfg(feature = "highlight")]
            theme: CodeTheme::default(),
        })
    }
}

impl Block<CodeBlock> {
    /// Sets the programming language for syntax highlighting.
    pub fn lang(mut self, lang: impl Display) -> Self {
        self.inner.lang = Some(lang.to_string());
        self
    }

    /// Sets the stripe style for the block prefix.
    pub fn style(mut self, style: StripeStyle) -> Self {
        self.inner.style = style;
        self
    }

    /// Sets the accent color of the block prefix.
    pub fn color(mut self, color: Color) -> Self {
        self.inner.color = Some(color);
        self
    }

    /// Sets a custom syntax highlighting theme.
    #[cfg(feature = "highlight")]
    pub fn theme(mut self, theme: CodeTheme) -> Self {
        self.inner.theme = theme;
        self
    }

    /// Renders the code block asynchronously.
    pub async fn render(self) -> io::Result<()> {
        self.render_static()
    }
}

impl Widget for CodeBlock {
    type Output = ();

    fn render_content(&self, max_width: usize) -> Vec<String> {
        let bar_col = self.color.unwrap_or(Color::DarkGrey);

        let raw_prefix = format!("{} ", self.style.char());
        let prefix = raw_prefix.clone().with(bar_col).to_string();
        let prefix_w = ansi::visible_width(&raw_prefix);

        let available_width = max_width.saturating_sub(prefix_w);
        if available_width == 0 {
            return Vec::new();
        }

        let mut lines = Vec::new();

        // language header
        if let Some(ref lang) = self.lang {
            let trimmed_lang = if ansi::visible_width(lang) > available_width {
                lang.chars().take(available_width).collect::<String>()
            } else {
                lang.clone()
            };
            lines.push(format!("{}{}", prefix, trimmed_lang.with(bar_col)));
        }

        let clean_code = self.code.replace("\r\n", "\n").replace('\r', "\n");

        // highlight the entire code block using tree-sitter
        #[cfg(feature = "highlight")]
        let highlighted_code =
            SyntaxHighlighter::highlight(&clean_code, self.lang.as_deref(), &self.theme);

        #[cfg(not(feature = "highlight"))]
        let highlighted_code = clean_code;

        // split the highlighted code into lines and clip to terminal bounds
        for highlighted_line in highlighted_code.split('\n') {
            if highlighted_line.is_empty() {
                lines.push(prefix.clone());
                continue;
            }

            let line_vis_w = ansi::visible_width(highlighted_line);

            if line_vis_w <= available_width {
                lines.push(format!("{}{}", prefix, highlighted_line));
            } else {
                // clip line visually to fit terminal width
                let mut clipped = String::new();
                let mut current_w = 0;

                for ch in highlighted_line.chars() {
                    let ch_w = ansi::visible_width(&ch.to_string());
                    if current_w + ch_w > available_width {
                        break;
                    }
                    clipped.push(ch);
                    current_w += ch_w;
                }

                lines.push(format!("{}{}", prefix, clipped));
            }
        }

        if lines.is_empty() {
            lines.push(prefix);
        }

        lines
    }
}
