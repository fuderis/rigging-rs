use crate::render::{LineStyle, block::Block, widget::Widget};

use crossterm::style::{Color, Stylize};
use std::io;

/// Represents a UI separator line widget.
pub struct Line {
    /// Visual style/pattern of the line.
    style: LineStyle,
    /// Optional foreground color for the line.
    color: Option<Color>,
}

impl Line {
    /// Creates a new `Line` widget wrapped in a `Block` with default configuration.
    pub fn new() -> Block<Self> {
        // initialize default line with single style and dark grey color
        Block::new(Self {
            style: LineStyle::Solid,
            color: Some(Color::DarkGrey),
        })
    }
}

impl Block<Line> {
    /// Sets the foreground color of the line.
    pub fn color(mut self, color: Color) -> Self {
        // assign new color to the inner line structure
        self.inner.color = Some(color);
        self
    }

    /// Sets the visual style of the line.
    pub fn style(mut self, style: LineStyle) -> Self {
        // update inner line style
        self.inner.style = style;
        self
    }

    /// Renders the line block asynchronously.
    pub async fn render(self) -> io::Result<()> {
        // delegate to static render implementation
        self.render_static()
    }
}

impl Widget for Line {
    type Output = ();

    /// Generates rendered string lines constrained by the specified maximum width.
    fn render_content(&self, max_width: usize) -> Vec<String> {
        // fallback to dark grey if color is not explicitly set
        let border_col = self.color.unwrap_or(Color::DarkGrey);
        let ch = self.style.as_char();

        // construct styled string filled to max width
        let line_str = ch
            .to_string()
            .repeat(max_width + 1)
            .with(border_col)
            .to_string();

        vec![line_str]
    }
}
