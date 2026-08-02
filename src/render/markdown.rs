use crate::{
    render::ansi,
    style::{BulletStyle, StripeStyle},
};

#[cfg(feature = "highlight")]
use super::highlight::SyntaxHighlighter;
#[cfg(feature = "highlight")]
use crate::theme::CodeTheme;

use crossterm::style::{Color, Stylize};
use markdown::{
    ParseOptions,
    mdast::{AlignKind, Node},
    to_mdast,
};

/// Markdown rendering settings and terminal formatter.
#[derive(Clone, Debug)]
pub struct Markdown {
    /// Style of the side bar used for blockquotes and code blocks.
    pub stripe_style: StripeStyle,
    /// Color of the side bar used for blockquotes and code blocks.
    pub stripe_color: Option<Color>,

    /// Style of the list item bullets.
    pub bullet_style: BulletStyle,
    /// Color of the list item bullets.
    pub bullet_color: Option<Color>,

    /// Color applied to inline code spans.
    pub code_color: Option<Color>,

    /// Theme settings for code syntax highlighting.
    #[cfg(feature = "highlight")]
    pub theme: CodeTheme,
}

impl Default for Markdown {
    fn default() -> Self {
        Self {
            stripe_style: StripeStyle::Single,
            stripe_color: Some(Color::DarkGrey),
            bullet_style: BulletStyle::Dot,
            bullet_color: Some(Color::DarkGrey),
            code_color: None,
            #[cfg(feature = "highlight")]
            theme: CodeTheme::default(),
        }
    }
}

impl Markdown {
    /// creates a new [`Markdown`] renderer with default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// sets the stripe style for blockquotes and code blocks.
    pub fn stripe_style(mut self, style: StripeStyle) -> Self {
        self.stripe_style = style;
        self
    }

    /// sets the stripe color for blockquotes and code blocks.
    pub fn stripe_color(mut self, color: Color) -> Self {
        self.stripe_color = Some(color);
        self
    }

    /// sets the bullet style for list items (e.g., Dot, Arrow, Star, Custom).
    pub fn bullet_style(mut self, style: BulletStyle) -> Self {
        self.bullet_style = style;
        self
    }

    /// sets the bullet color for list items.
    pub fn bullet_color(mut self, color: Color) -> Self {
        self.bullet_color = Some(color);
        self
    }

    /// sets the color for inline code spans.
    pub fn code_color(mut self, color: Color) -> Self {
        self.code_color = Some(color);
        self
    }

    /// sets the theme for code syntax highlighting.
    #[cfg(feature = "highlight")]
    pub fn theme(mut self, theme: CodeTheme) -> Self {
        self.theme = theme;
        self
    }

    /// renders raw Markdown content into an ANSI-styled terminal string.
    pub fn render(&self, content: impl AsRef<str>, max_width: usize) -> String {
        let options = ParseOptions::gfm();

        let ast = match to_mdast(content.as_ref(), &options) {
            Ok(node) => node,
            Err(_) => return content.as_ref().to_string(),
        };

        self.render_node(&ast, max_width, "")
    }

    fn render_node(&self, node: &Node, max_width: usize, indent: &str) -> String {
        match node {
            Node::Root(root) => root
                .children
                .iter()
                .map(|child| self.render_node(child, max_width, indent))
                .collect::<Vec<_>>()
                .join("\n"),

            Node::Paragraph(p) => {
                let inner = p
                    .children
                    .iter()
                    .map(|child| self.render_node(child, max_width, indent))
                    .collect::<String>();

                let wrapped_lines = ansi::wrap_terminal_text(&inner, max_width);
                wrapped_lines
                    .into_iter()
                    .map(|line| format!("{}{}\n", indent, line))
                    .collect::<String>()
            }

            Node::Heading(h) => match h.depth {
                1 | 2 => {
                    let inner = h
                        .children
                        .iter()
                        .map(|child| self.render_node(child, max_width, indent))
                        .collect::<String>();

                    let wrapped = ansi::wrap_terminal_text(&inner, max_width);
                    wrapped
                        .into_iter()
                        .map(|line| {
                            if h.depth == 1 {
                                format!("{}{}\n", indent, line.bold().underlined())
                            } else {
                                format!("{}{}\n", indent, line.bold())
                            }
                        })
                        .collect::<String>()
                }
                _ => {
                    let symbol = self.bullet_style.render_symbol(0);
                    let raw_prefix = format!("{} ", symbol);
                    let prefix_w = ansi::visible_width(&raw_prefix);
                    let inner_width = max_width.saturating_sub(prefix_w);
                    let child_indent = format!("{}{}", indent, " ".repeat(prefix_w));

                    let inner = h
                        .children
                        .iter()
                        .map(|child| self.render_node(child, inner_width, &child_indent))
                        .collect::<String>();

                    let wrapped = ansi::wrap_terminal_text(&inner, inner_width);

                    let styled_symbol = match self.bullet_color {
                        Some(col) => symbol.with(col).to_string(),
                        None => symbol,
                    };

                    wrapped
                        .into_iter()
                        .enumerate()
                        .map(|(idx, line)| {
                            let p = if idx == 0 {
                                format!("{}{}{} ", indent, styled_symbol, "")
                            } else {
                                child_indent.clone()
                            };
                            format!("{}{}\n", p, line.bold())
                        })
                        .collect::<String>()
                }
            },

            Node::Text(t) => t.value.clone(),

            Node::Strong(s) => {
                let inner = s
                    .children
                    .iter()
                    .map(|child| self.render_node(child, max_width, indent))
                    .collect::<String>();
                inner.bold().to_string()
            }

            Node::Emphasis(e) => {
                let inner = e
                    .children
                    .iter()
                    .map(|child| self.render_node(child, max_width, indent))
                    .collect::<String>();
                inner.italic().to_string()
            }

            Node::InlineCode(c) => match self.code_color {
                Some(col) => c.value.as_str().with(col).to_string(),
                None => c.value.as_str().dim().to_string(),
            },

            Node::Code(c) => self.render_code_block(c, max_width),

            Node::Blockquote(b) => {
                let bar_char = format!("{} ", self.stripe_style.char());
                let prefix_w = ansi::visible_width(&bar_char);
                let inner_width = max_width.saturating_sub(prefix_w);

                let prefix = match self.stripe_color {
                    Some(col) => bar_char.with(col).to_string(),
                    None => bar_char,
                };

                let inner = b
                    .children
                    .iter()
                    .map(|child| self.render_node(child, inner_width, ""))
                    .collect::<String>();

                inner
                    .lines()
                    .map(|l| format!("{}{}{}", indent, prefix, l))
                    .collect::<Vec<_>>()
                    .join("\n")
                    + "\n"
            }

            // --- Lists ---
            Node::List(l) => {
                l.children
                    .iter()
                    .enumerate()
                    .map(|(idx, child)| {
                        let symbol = if l.ordered {
                            BulletStyle::Number.render_symbol(idx)
                        } else {
                            self.bullet_style.render_symbol(idx)
                        };

                        let raw_prefix = format!("{} ", symbol);
                        let prefix_w = ansi::visible_width(&raw_prefix);
                        let inner_width = max_width.saturating_sub(prefix_w);

                        // Add exactly 2 spaces to the parent's current indent
                        let child_indent = format!("{}  ", indent);

                        let item_str = match child {
                            Node::ListItem(li) => li
                                .children
                                .iter()
                                .map(|sub_child| {
                                    self.render_node(sub_child, inner_width, &child_indent)
                                })
                                .collect::<String>(),
                            _ => self.render_node(child, inner_width, &child_indent),
                        };

                        let styled_symbol = match self.bullet_color {
                            Some(col) => symbol.with(col).to_string(),
                            None => symbol,
                        };

                        let mut lines = item_str.lines();
                        let first_line = lines.next().unwrap_or("").trim_start();

                        let mut result =
                            format!("{}{}{} {}\n", indent, styled_symbol, "", first_line);
                        for line in lines {
                            result.push_str(&format!("{}\n", line));
                        }
                        result
                    })
                    .collect::<Vec<_>>()
                    .join("")
            }

            Node::ListItem(li) => li
                .children
                .iter()
                .map(|child| self.render_node(child, max_width, indent))
                .collect::<String>(),

            Node::Table(t) => self.render_table(t, max_width),

            _ => String::new(),
        }
    }

    /// Renders a code block with syntax highlighting and a custom side bar.
    fn render_code_block(&self, c: &markdown::mdast::Code, max_width: usize) -> String {
        let raw_prefix = format!("{} ", self.stripe_style.char());
        let prefix = match self.stripe_color {
            Some(col) => raw_prefix.clone().with(col).to_string(),
            None => raw_prefix.clone(),
        };

        let prefix_w = ansi::visible_width(&raw_prefix);
        let available_width = max_width.saturating_sub(prefix_w);

        if available_width == 0 {
            return String::new();
        }

        let mut lines = Vec::new();

        // Header with language tag (if specified)
        if let Some(ref lang) = c.lang {
            let trimmed_lang = if ansi::visible_width(lang) > available_width {
                lang.chars().take(available_width).collect::<String>()
            } else {
                lang.clone()
            };

            let styled_lang = match self.stripe_color {
                Some(col) => trimmed_lang.with(col).to_string(),
                None => trimmed_lang,
            };

            lines.push(format!("{}{}", prefix, styled_lang));
        }

        let clean_code = c.value.replace("\r\n", "\n").replace('\r', "\n");

        #[cfg(feature = "highlight")]
        let highlighted_code = SyntaxHighlighter::highlight(
            &clean_code,
            c.lang.as_deref(),
            &self.theme,
            Some(available_width),
        );

        #[cfg(not(feature = "highlight"))]
        let highlighted_code = wrap_plain_code(&clean_code, available_width);

        // Prepend side bar prefix to each line
        for highlighted_line in highlighted_code.split('\n') {
            if highlighted_line.is_empty() {
                lines.push(prefix.clone());
                continue;
            }
            lines.push(format!("{}{}", prefix, highlighted_line));
        }

        if lines.is_empty() {
            lines.push(prefix);
        }

        lines.join("\n") + "\n"
    }

    /// Renders tables with dynamically calculated column widths.
    fn render_table(&self, t: &markdown::mdast::Table, max_width: usize) -> String {
        let alignments = &t.align;

        let raw_rows: Vec<Vec<String>> = t
            .children
            .iter()
            .filter_map(|row_node| {
                let Node::TableRow(row) = row_node else {
                    return None;
                };

                let cells: Vec<String> = row
                    .children
                    .iter()
                    .map(|cell_node| {
                        let Node::TableCell(cell) = cell_node else {
                            return String::new();
                        };

                        cell.children
                            .iter()
                            .map(|child| self.render_node(child, max_width, ""))
                            .collect::<String>()
                            .replace(['\n', '\r'], " ")
                            .split_whitespace()
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .collect();

                if cells.is_empty() { None } else { Some(cells) }
            })
            .collect();

        if raw_rows.is_empty() {
            return String::new();
        }

        let num_cols = raw_rows.iter().map(|r| r.len()).max().unwrap_or(0);
        if num_cols == 0 {
            return String::new();
        }

        let rows: Vec<Vec<String>> = raw_rows
            .into_iter()
            .map(|mut r| {
                r.resize(num_cols, String::new());
                r
            })
            .collect();

        let mut min_col_widths = vec![3usize; num_cols];
        let mut natural_col_widths = vec![0usize; num_cols];

        for row in &rows {
            for (col_idx, cell) in row.iter().enumerate() {
                natural_col_widths[col_idx] =
                    std::cmp::max(natural_col_widths[col_idx], ansi::visible_width(cell));

                for word in cell.split_whitespace() {
                    min_col_widths[col_idx] =
                        std::cmp::max(min_col_widths[col_idx], ansi::visible_width(word));
                }
            }
        }

        let overhead = num_cols * 3 + 1;
        let available_for_cols = max_width.saturating_sub(overhead);

        let mut col_widths = vec![0usize; num_cols];
        let total_natural: usize = natural_col_widths.iter().sum();

        if total_natural <= available_for_cols {
            col_widths = natural_col_widths.clone();
        } else {
            let total_min: usize = min_col_widths.iter().sum();

            if available_for_cols <= total_min {
                col_widths = min_col_widths.clone();
            } else {
                let extra_space = available_for_cols - total_min;
                let extra_natural_pool: usize = natural_col_widths
                    .iter()
                    .zip(&min_col_widths)
                    .map(|(&nat, &min)| nat.saturating_sub(min))
                    .sum();

                let mut used = 0usize;

                for i in 0..num_cols {
                    let span = natural_col_widths[i].saturating_sub(min_col_widths[i]);
                    let extra = if extra_natural_pool > 0 {
                        (extra_space * span) / extra_natural_pool
                    } else {
                        0
                    };
                    col_widths[i] = min_col_widths[i] + extra;
                    used += extra;
                }

                let mut rem = extra_space.saturating_sub(used);
                let mut i = 0;
                while rem > 0 && num_cols > 0 {
                    if col_widths[i] < natural_col_widths[i] {
                        col_widths[i] += 1;
                        rem -= 1;
                    }
                    i = (i + 1) % num_cols;
                }
            }
        }

        let make_border = |left: &str, mid: &str, right: &str| -> String {
            let parts: Vec<String> = col_widths.iter().map(|&w| "─".repeat(w + 2)).collect();
            format!("{}{}{}\n", left, parts.join(mid), right)
        };

        let top_border = make_border("┌", "┬", "┐");
        let header_sep = make_border("├", "┼", "┤");
        let bottom_border = make_border("└", "┴", "┘");

        let mut formatted = String::new();
        formatted.push_str(&top_border);

        for (row_idx, row) in rows.iter().enumerate() {
            let mut cell_lines: Vec<Vec<String>> = Vec::with_capacity(num_cols);
            let mut max_lines = 1usize;

            for (col_idx, &target_width) in col_widths.iter().enumerate() {
                let raw_cell = row.get(col_idx).map(|s| s.as_str()).unwrap_or("");
                let align = alignments.get(col_idx).cloned().unwrap_or(AlignKind::None);

                let lines = pad_and_wrap_cell(raw_cell, target_width, align, row_idx == 0);
                max_lines = max_lines.max(lines.len());
                cell_lines.push(lines);
            }

            for line_idx in 0..max_lines {
                let mut row_str = String::from("│");

                for (col_idx, &target_width) in col_widths.iter().enumerate() {
                    let align = alignments.get(col_idx).cloned().unwrap_or(AlignKind::None);

                    let empty_padding = apply_alignment("", target_width, align);
                    let default_line = if row_idx == 0 {
                        empty_padding.bold().to_string()
                    } else {
                        empty_padding
                    };

                    let line_content = cell_lines
                        .get(col_idx)
                        .and_then(|lines| lines.get(line_idx))
                        .cloned()
                        .unwrap_or(default_line);

                    row_str.push_str(&format!(" {} │", line_content));
                }

                formatted.push_str(&row_str);
                formatted.push('\n');
            }

            if row_idx == 0 && rows.len() > 1 {
                formatted.push_str(&header_sep);
            } else if row_idx > 0 && row_idx + 1 < rows.len() {
                let mut empty_row_str = String::from("│");
                for &target_width in &col_widths {
                    let empty_cell = " ".repeat(target_width + 2);
                    empty_row_str.push_str(&format!("{}│", empty_cell));
                }
                formatted.push_str(&empty_row_str);
                formatted.push('\n');
            }
        }

        formatted.push_str(&bottom_border);
        formatted
    }
}

/// Wraps plain code text without active `highlight` feature.
#[cfg(not(feature = "highlight"))]
fn wrap_plain_code(code: &str, available_width: usize) -> String {
    if available_width == 0 {
        return code.to_string();
    }
    let mut result_lines = Vec::new();
    for line in code.lines() {
        if ansi::visible_width(line) <= available_width {
            result_lines.push(line.to_string());
        } else {
            let wrapped = ansi::wrap_terminal_text(line, available_width);
            result_lines.extend(wrapped);
        }
    }
    result_lines.join("\n")
}

/// renders Markdown text using default configurations for quick one-off rendering.
pub fn markdown(content: impl AsRef<str>, max_width: usize) -> String {
    Markdown::default().render(content, max_width)
}

fn apply_alignment(text: &str, target_width: usize, align: AlignKind) -> String {
    let current_w = ansi::visible_width(text);
    let total_pad = target_width.saturating_sub(current_w);

    match align {
        AlignKind::Right => {
            format!("{}{}", " ".repeat(total_pad), text)
        }
        AlignKind::Center => {
            let left_pad = total_pad / 2;
            let right_pad = total_pad - left_pad;
            format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
        }
        AlignKind::Left | AlignKind::None => {
            format!("{}{}", text, " ".repeat(total_pad))
        }
    }
}

fn pad_and_wrap_cell(
    text: &str,
    target_width: usize,
    align: AlignKind,
    is_header: bool,
) -> Vec<String> {
    let current_w = ansi::visible_width(text);

    if current_w <= target_width {
        let full_text = apply_alignment(text, target_width, align);
        let styled = if is_header {
            full_text.bold().to_string()
        } else {
            full_text
        };
        return vec![styled];
    }

    let mut result = Vec::new();
    let mut current_line = String::new();
    let mut current_len = 0;

    for word in text.split_whitespace() {
        let word_w = ansi::visible_width(word);

        if current_len + word_w + (if current_len > 0 { 1 } else { 0 }) > target_width {
            if !current_line.is_empty() {
                let aligned = apply_alignment(&current_line, target_width, align);
                result.push(if is_header {
                    aligned.bold().to_string()
                } else {
                    aligned
                });
                current_line.clear();
                current_len = 0;
            }
        }

        if !current_line.is_empty() {
            current_line.push(' ');
            current_len += 1;
        }

        current_line.push_str(word);
        current_len += word_w;
    }

    if !current_line.is_empty() {
        let aligned = apply_alignment(&current_line, target_width, align);
        result.push(if is_header {
            aligned.bold().to_string()
        } else {
            aligned
        });
    }

    result
}
