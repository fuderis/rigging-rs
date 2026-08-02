use crate::render::{StripStyle, ansi, block::Block, widget::Widget};

use crossterm::style::{Color, Stylize};
use std::io;

/// A UI widget designed to display block quotes with customizable visual stripes.
pub struct Quote {
    /// The body text of the quote.
    text: String,
    /// The stripe style used for the left border.
    style: StripeStyle,
    /// Optional color override for the left stripe.
    color: Option<Color>,
}

impl Quote {
    /// Creates a new quote block with default styling (single stripe, dark grey color).
    pub fn new(text: impl Into<String>) -> Block<Self> {
        Block::new(Self {
            text: text.into(),
            style: StripeStyle::Single,
            color: Some(Color::DarkGrey),
        })
    }
}

impl Block<Quote> {
    /// Sets the style of the left vertical stripe.
    pub fn style(mut self, style: StripeStyle) -> Self {
        self.inner.style = style;
        self
    }

    /// Sets the color of the left vertical stripe.
    pub fn color(mut self, color: Color) -> Self {
        self.inner.color = Some(color);
        self
    }

    /// Asynchronously renders the quote block.
    pub async fn render(self) -> io::Result<()> {
        self.render_static()
    }
}

impl Widget for Quote {
    type Output = ();

    /// Renders the quote text into wrapped, styled terminal lines based on the available width.
    fn render_content(&self, max_width: usize) -> Vec<String> {
        let bar_col = self.color.unwrap_or(Color::DarkGrey);

        // format styled prefix based on the stripe character
        let raw_prefix = format!("{} ", self.style.char());
        let prefix = raw_prefix.clone().with(bar_col).to_string();
        let prefix_w = ansi::visible_width(&raw_prefix);

        let available_width = max_width.saturating_sub(prefix_w);
        if available_width == 0 {
            return Vec::new();
        }

        let mut lines = Vec::new();
        let clean_text = self.text.replace("\r\n", "\n").replace('\r', "\n");

        for paragraph in clean_text.split('\n') {
            if paragraph.is_empty() {
                lines.push(prefix.clone());
                continue;
            }

            let mut current_line = String::new();
            let mut current_color_prefix = String::new();

            for word in paragraph.split_whitespace() {
                let word_vis_w = ansi::visible_width(word);

                // split character by character if the word exceeds available width
                if word_vis_w > available_width {
                    if !current_line.is_empty() {
                        lines.push(format!("{}{}", prefix, current_line));
                        current_line = current_color_prefix.clone();
                    }

                    for ch in word.chars() {
                        let ch_vis_w = ansi::visible_width(&ch.to_string());
                        let line_vis_w = ansi::visible_width(&current_line);

                        if line_vis_w + ch_vis_w > available_width {
                            lines.push(format!("{}{}", prefix, current_line));
                            current_line = current_color_prefix.clone();
                        }
                        current_line.push(ch);
                    }
                    continue;
                }

                let current_vis_w = ansi::visible_width(&current_line);
                let space_needed =
                    if current_line.is_empty() || current_line == current_color_prefix {
                        0
                    } else {
                        1
                    };

                if current_vis_w + space_needed + word_vis_w <= available_width {
                    if !current_line.is_empty() && current_line != current_color_prefix {
                        current_line.push(' ');
                    }
                    current_line.push_str(word);
                } else {
                    lines.push(format!("{}{}", prefix, current_line));
                    current_line = format!("{}{}", current_color_prefix, word);
                }

                // preserve active ansi color for line wrap continuity
                if let Some(active_col) = ansi::get_active_text_color(&current_line) {
                    current_color_prefix = active_col;
                }
            }

            if !current_line.is_empty() {
                lines.push(format!("{}{}", prefix, current_line));
            }
        }

        if lines.is_empty() {
            lines.push(prefix);
        }

        lines
    }
}
