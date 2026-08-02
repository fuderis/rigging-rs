use super::ansi::{get_active_text_color, visible_width};
use crate::theme::CodeTheme;

use crossterm::style::{Color, Stylize};
use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

/// Utility for syntax highlighting source code using Tree-sitter parsers.
pub struct SyntaxHighlighter;

impl SyntaxHighlighter {
    /// Returns a `HighlightConfiguration` for the specified programming language.
    ///
    /// Returns `None` if the provided language identifier is unsupported or invalid.
    fn get_config(lang: &str) -> Option<HighlightConfiguration> {
        let mut config = match lang.to_lowercase().as_str() {
            "yaml" => HighlightConfiguration::new(
                tree_sitter_yaml::LANGUAGE.into(),
                "yaml",
                tree_sitter_yaml::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            "toml" => HighlightConfiguration::new(
                tree_sitter_toml::LANGUAGE.into(),
                "toml",
                tree_sitter_toml::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            "rust" | "rs" => HighlightConfiguration::new(
                tree_sitter_rust::LANGUAGE.into(),
                "rust",
                tree_sitter_rust::HIGHLIGHTS_QUERY,
                tree_sitter_rust::INJECTIONS_QUERY,
                "",
            ),
            "bash" | "sh" | "zsh" => HighlightConfiguration::new(
                tree_sitter_bash::LANGUAGE.into(),
                "bash",
                tree_sitter_bash::HIGHLIGHT_QUERY,
                "",
                "",
            ),
            "python" | "py" => HighlightConfiguration::new(
                tree_sitter_python::LANGUAGE.into(),
                "python",
                tree_sitter_python::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            "javascript" | "js" => HighlightConfiguration::new(
                tree_sitter_javascript::LANGUAGE.into(),
                "javascript",
                tree_sitter_javascript::HIGHLIGHT_QUERY,
                tree_sitter_javascript::INJECTIONS_QUERY,
                "",
            ),
            "typescript" | "ts" => HighlightConfiguration::new(
                tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
                "typescript",
                tree_sitter_typescript::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            "go" => HighlightConfiguration::new(
                tree_sitter_go::LANGUAGE.into(),
                "go",
                tree_sitter_go::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            "json" => HighlightConfiguration::new(
                tree_sitter_json::LANGUAGE.into(),
                "json",
                tree_sitter_json::HIGHLIGHTS_QUERY,
                "",
                "",
            ),
            _ => return None,
        }
        .ok()?;

        config.configure(CodeTheme::HIGHLIGHT_NAMES);
        Some(config)
    }

    /// Highlights the source code string using Tree-sitter and returns it formatted with ANSI escape sequences.
    ///
    /// If `lang` is `None` or unsupported, or if highlighting fails, the code is returned wrapped as plain text.
    pub fn highlight(
        code: &str,
        lang: Option<&str>,
        theme: &CodeTheme,
        max_width: Option<usize>,
    ) -> String {
        let Some(lang_name) = lang else {
            return Self::wrap_plain_code(code, max_width);
        };

        let Some(config) = Self::get_config(lang_name) else {
            return Self::wrap_plain_code(code, max_width);
        };

        let mut highlighter = Highlighter::new();
        let highlights = match highlighter.highlight(&config, code.as_bytes(), None, |_| None) {
            Ok(h) => h,
            Err(_) => return Self::wrap_plain_code(code, max_width),
        };

        let mut raw_highlighted = String::new();
        let mut color_stack: Vec<Color> = Vec::new();

        for event in highlights {
            match event {
                Ok(HighlightEvent::Source { start, end }) => {
                    let fragment = &code[start..end];
                    if let Some(&active_color) = color_stack.last() {
                        if active_color != Color::Reset {
                            let mut first = true;
                            for line_part in fragment.split('\n') {
                                if !first {
                                    raw_highlighted.push('\n');
                                }
                                first = false;
                                if !line_part.is_empty() {
                                    raw_highlighted
                                        .push_str(&line_part.with(active_color).to_string());
                                }
                            }
                        } else {
                            raw_highlighted.push_str(fragment);
                        }
                    } else {
                        raw_highlighted.push_str(fragment);
                    }
                }
                Ok(HighlightEvent::HighlightStart(s)) => {
                    color_stack.push(theme.color_for_index(s.0));
                }
                Ok(HighlightEvent::HighlightEnd) => {
                    color_stack.pop();
                }
                Err(_) => break,
            }
        }

        if let Some(width) = max_width {
            Self::wrap_highlighted_code(&raw_highlighted, width)
        } else {
            raw_highlighted
        }
    }

    /// Normalizes line endings and wraps plain unhighlighted code to the specified maximum width.
    pub fn wrap_plain_code(code: &str, max_width: Option<usize>) -> String {
        let clean = code.replace("\r\n", "\n").replace('\r', "\n");
        let Some(width) = max_width else {
            return clean;
        };
        Self::wrap_highlighted_code(&clean, width)
    }

    /// Performs exact character-by-character line wrapping while detecting and restoring ANSI text colors using `get_active_text_color`.
    fn wrap_highlighted_code(content: &str, max_width: usize) -> String {
        if max_width == 0 {
            return content.to_string();
        }

        let mut final_lines = Vec::new();

        for line in content.split('\n') {
            if visible_width(line) <= max_width {
                final_lines.push(line.to_string());
                continue;
            }

            let mut current_chunk = String::new();
            let mut current_w = 0;

            let mut chars = line.chars().peekable();
            while let Some(ch) = chars.next() {
                // handle ANSI escape sequences
                if ch == '\x1b' {
                    let mut ansi_seq = String::new();
                    ansi_seq.push(ch);
                    while let Some(&next_ch) = chars.peek() {
                        ansi_seq.push(next_ch);
                        chars.next();
                        if next_ch == 'm' {
                            break;
                        }
                    }
                    current_chunk.push_str(&ansi_seq);
                    continue;
                }

                let ch_str = ch.to_string();
                let ch_w = visible_width(&ch_str);

                // wrap line if adding character exceeds maximum width
                if current_w + ch_w > max_width {
                    // retrieve active text color for current chunk using helper function
                    let active_color = get_active_text_color(&current_chunk);

                    // reset current styles before line break
                    current_chunk.push_str("\x1b[0m");
                    final_lines.push(current_chunk);

                    // start new line and restore active color if present
                    current_chunk = String::new();
                    if let Some(color_code) = active_color {
                        current_chunk.push_str(&color_code);
                    }

                    current_w = 0;
                }

                current_chunk.push(ch);
                current_w += ch_w;
            }

            final_lines.push(current_chunk);
        }

        final_lines.join("\n")
    }
}
