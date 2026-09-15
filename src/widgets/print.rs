use crate::{
    render::{block::Block, widget::Widget},
    utils::ansi,
};

use crossterm::style::{Color, Stylize};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Info,
    Warn,
    Success,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HeaderLevel {
    #[default]
    H1,
    H2,
    H3,
}

pub enum PrintBlock {
    Header {
        level: HeaderLevel,
        text: String,
        color: Option<Color>,
    },
    Message {
        kind: MessageKind,
        text: String,
        color: Option<Color>,
    },
    Bullet {
        text: String,
        color: Option<Color>,
    },
    Tree {
        text: String,
        color: Option<Color>,
    },
    Field {
        label: String,
        value: String,
        color: Option<Color>,
    },
    Line {
        color: Option<Color>,
    },
}

pub struct Print {
    pub(crate) blocks: Vec<PrintBlock>,
}

impl Print {
    pub fn h1(text: impl Into<String>) -> Block<Self> {
        Self::new().h1(text)
    }

    pub fn h2(text: impl Into<String>) -> Block<Self> {
        Self::new().h2(text)
    }

    pub fn h3(text: impl Into<String>) -> Block<Self> {
        Self::new().h3(text)
    }

    pub fn info(text: impl Into<String>) -> Block<Self> {
        Self::new().info(text)
    }

    pub fn warn(text: impl Into<String>) -> Block<Self> {
        Self::new().warn(text)
    }

    pub fn success(text: impl Into<String>) -> Block<Self> {
        Self::new().success(text)
    }

    pub fn error(text: impl Into<String>) -> Block<Self> {
        Self::new().error(text)
    }

    pub fn item(text: impl Into<String>) -> Block<Self> {
        Self::new().item(text)
    }

    pub fn tree_item(text: impl Into<String>) -> Block<Self> {
        Self::new().tree_item(text)
    }

    pub fn field(label: impl Into<String>, value: impl Into<String>) -> Block<Self> {
        Self::new().field(label, value)
    }

    pub fn line() -> Block<Self> {
        Self::new().line()
    }

    pub fn new() -> Block<Self> {
        Block::new(Self { blocks: Vec::new() }, ())
    }
}

impl Block<Print> {
    pub fn color(mut self, color: Color) -> Self {
        if let Some(last) = self.inner.blocks.last_mut() {
            match last {
                PrintBlock::Header { color: c, .. }
                | PrintBlock::Message { color: c, .. }
                | PrintBlock::Bullet { color: c, .. }
                | PrintBlock::Tree { color: c, .. }
                | PrintBlock::Field { color: c, .. }
                | PrintBlock::Line { color: c } => *c = Some(color),
            }
        }
        self
    }

    pub fn h1(mut self, text: impl Into<String>) -> Self {
        self.inner.blocks.push(PrintBlock::Header {
            level: HeaderLevel::H1,
            text: text.into(),
            color: None,
        });
        self
    }

    pub fn h2(mut self, text: impl Into<String>) -> Self {
        self.inner.blocks.push(PrintBlock::Header {
            level: HeaderLevel::H2,
            text: text.into(),
            color: None,
        });
        self
    }

    pub fn h3(mut self, text: impl Into<String>) -> Self {
        self.inner.blocks.push(PrintBlock::Header {
            level: HeaderLevel::H3,
            text: text.into(),
            color: None,
        });
        self
    }

    pub fn info(mut self, text: impl Into<String>) -> Self {
        self.inner.blocks.push(PrintBlock::Message {
            kind: MessageKind::Info,
            text: text.into(),
            color: None,
        });
        self
    }

    pub fn warn(mut self, text: impl Into<String>) -> Self {
        self.inner.blocks.push(PrintBlock::Message {
            kind: MessageKind::Warn,
            text: text.into(),
            color: None,
        });
        self
    }

    pub fn success(mut self, text: impl Into<String>) -> Self {
        self.inner.blocks.push(PrintBlock::Message {
            kind: MessageKind::Success,
            text: text.into(),
            color: None,
        });
        self
    }

    pub fn error(mut self, text: impl Into<String>) -> Self {
        self.inner.blocks.push(PrintBlock::Message {
            kind: MessageKind::Error,
            text: text.into(),
            color: None,
        });
        self
    }

    pub fn item(mut self, text: impl Into<String>) -> Self {
        self.inner.blocks.push(PrintBlock::Bullet {
            text: text.into(),
            color: None,
        });
        self
    }

    pub fn tree_item(mut self, text: impl Into<String>) -> Self {
        self.inner.blocks.push(PrintBlock::Tree {
            text: text.into(),
            color: None,
        });
        self
    }

    pub fn field(mut self, label: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner.blocks.push(PrintBlock::Field {
            label: label.into(),
            value: value.into(),
            color: None,
        });
        self
    }

    pub fn line(mut self) -> Self {
        self.inner.blocks.push(PrintBlock::Line { color: None });
        self
    }
}

impl Widget for Print {
    type State = ();
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
        _state: &Arc<Self::State>,
        max_width: Option<usize>,
        _max_height: Option<usize>,
        _is_final: bool,
    ) -> Vec<String> {
        let mut lines = Vec::new();
        let mut idx = 0;

        while idx < self.blocks.len() {
            if matches!(self.blocks[idx], PrintBlock::Field { .. }) {
                let start_idx = idx;
                let mut max_label_len = 0;

                while idx < self.blocks.len() {
                    if let PrintBlock::Field { label, .. } = &self.blocks[idx] {
                        let label_len = ansi::visible_width(label);
                        if label_len > max_label_len {
                            max_label_len = label_len;
                        }
                        idx += 1;
                    } else {
                        break;
                    }
                }

                for i in start_idx..idx {
                    if let PrintBlock::Field {
                        label,
                        value,
                        color,
                    } = &self.blocks[i]
                    {
                        let label_w = ansi::visible_width(label);
                        let padding = " ".repeat(max_label_len.saturating_sub(label_w));

                        let raw_prefix = format!("  {}{} : ", label, padding);
                        let mut styled_prefix = raw_prefix.as_str().bold();
                        if let Some(c) = color {
                            styled_prefix = styled_prefix.with(*c);
                        }

                        let prefix = styled_prefix.to_string();
                        let prefix_w = ansi::visible_width(&prefix);

                        let target_w = max_width.map(|w| w.saturating_sub(prefix_w));
                        let wrapped = match target_w {
                            Some(w) if w > 0 => ansi::wrap_terminal_text(value, w),
                            _ => value.lines().map(String::from).collect(),
                        };

                        let indent = " ".repeat(prefix_w);
                        for (line_idx, l) in wrapped.into_iter().enumerate() {
                            if line_idx == 0 {
                                lines.push(format!("{prefix}{l}\x1b[0m"));
                            } else {
                                lines.push(format!("{indent}{l}\x1b[0m"));
                            }
                        }
                    }
                }
                continue;
            }

            match &self.blocks[idx] {
                PrintBlock::Header { level, text, color } => {
                    let mut styled = match level {
                        HeaderLevel::H1 => text.as_str().bold().underlined(),
                        HeaderLevel::H2 => text.as_str().bold(),
                        HeaderLevel::H3 => text.as_str().bold().italic(),
                    };

                    if let Some(c) = color {
                        styled = styled.with(*c);
                    }

                    let output = styled.to_string();
                    let wrapped = match max_width {
                        Some(w) if w > 0 => ansi::wrap_terminal_text(&output, w),
                        _ => vec![output],
                    };
                    lines.extend(wrapped);
                }

                PrintBlock::Message { kind, text, color } => {
                    let (icon_str, default_color) = match kind {
                        MessageKind::Info => ("•", Color::Cyan),
                        MessageKind::Warn => ("ℹ", Color::Yellow),
                        MessageKind::Success => ("✓", Color::Green),
                        MessageKind::Error => ("✗", Color::Red),
                    };

                    let active_color = color.unwrap_or(default_color);

                    let icon_prefix = format!("{} ", icon_str.with(active_color));
                    let icon_w = ansi::visible_width(&icon_prefix);
                    let indent_padding = " ".repeat(icon_w);

                    let formatted_body = if matches!(kind, MessageKind::Warn | MessageKind::Error) {
                        if let Some((prefix, tail)) = text.split_once(": ") {
                            format!(
                                "{}{}{tail}",
                                prefix.with(active_color),
                                ":".with(active_color)
                            )
                        } else {
                            text.clone()
                        }
                    } else {
                        text.clone()
                    };

                    let target_w = max_width.map(|w| w.saturating_sub(icon_w));
                    let wrapped_lines = match target_w {
                        Some(w) if w > 0 => ansi::wrap_terminal_text(&formatted_body, w),
                        _ => formatted_body.lines().map(String::from).collect(),
                    };

                    for (line_idx, line) in wrapped_lines.into_iter().enumerate() {
                        if line_idx == 0 {
                            lines.push(format!("{icon_prefix}{line}\x1b[0m"));
                        } else {
                            lines.push(format!("{indent_padding}{line}\x1b[0m"));
                        }
                    }

                    if lines.is_empty() {
                        lines.push(icon_prefix);
                    }
                }

                PrintBlock::Bullet { text, color } => {
                    let mut symbol = "  • ".bold();
                    if let Some(c) = color {
                        symbol = symbol.with(*c);
                    }
                    let prefix = symbol.to_string();
                    let prefix_w = ansi::visible_width(&prefix);
                    let target_w = max_width.map(|w| w.saturating_sub(prefix_w));

                    let wrapped = match target_w {
                        Some(w) if w > 0 => ansi::wrap_terminal_text(text, w),
                        _ => text.lines().map(String::from).collect(),
                    };

                    let indent = " ".repeat(prefix_w);
                    for (i, l) in wrapped.into_iter().enumerate() {
                        if i == 0 {
                            lines.push(format!("{prefix}{l}\x1b[0m"));
                        } else {
                            lines.push(format!("{indent}{l}\x1b[0m"));
                        }
                    }
                }

                PrintBlock::Tree { text, color } => {
                    let mut symbol = "  └─ ".bold();
                    if let Some(c) = color {
                        symbol = symbol.with(*c);
                    }
                    let prefix = symbol.to_string();
                    let prefix_w = ansi::visible_width(&prefix);
                    let target_w = max_width.map(|w| w.saturating_sub(prefix_w));

                    let wrapped = match target_w {
                        Some(w) if w > 0 => ansi::wrap_terminal_text(text, w),
                        _ => text.lines().map(String::from).collect(),
                    };

                    let indent = " ".repeat(prefix_w);
                    for (i, l) in wrapped.into_iter().enumerate() {
                        if i == 0 {
                            lines.push(format!("{prefix}{l}\x1b[0m"));
                        } else {
                            lines.push(format!("{indent}{l}\x1b[0m"));
                        }
                    }
                }

                PrintBlock::Line { color } => {
                    let width = max_width.unwrap_or(80);
                    let raw_line = "─".repeat(width);

                    let styled_line = if let Some(c) = color {
                        raw_line.with(*c).to_string()
                    } else {
                        raw_line.dark_grey().to_string()
                    };

                    lines.push(format!("{styled_line}\x1b[0m"));
                }

                PrintBlock::Field { .. } => unreachable!(),
            }

            idx += 1;
        }

        lines
    }

    fn extract_output(&mut self) -> Self::Output {}
}
