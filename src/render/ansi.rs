use crossterm::style::Color;
use unicode_width::UnicodeWidthStr;

#[cfg(feature = "markdown")]
use crossterm::style::Stylize;

/// parses the active ANSI foreground color sequence by scanning backwards from the end of a string slice.
///
/// # Arguments
///
/// * `text_slice` - The slice of string content to inspect for active ANSI escape codes.
///
/// # Returns
///
/// * `Some(String)` - The raw ANSI escape sequence restoring the last foreground color if found.
/// * `None` - If no active color is found, or if a reset/default color sequence (`0` or `39`) was encountered last.
pub fn get_active_text_color(text_slice: &str) -> Option<String> {
    let bytes = text_slice.as_bytes();
    let mut i = bytes.len();

    while i > 0 {
        i -= 1;

        if bytes[i] == b'm' {
            let mut j = i;
            while j > 0 {
                j -= 1;
                if bytes[j] == 0x1B && j + 1 < bytes.len() && bytes[j + 1] == b'[' {
                    let full_ansi = match std::str::from_utf8(&bytes[j..=i]) {
                        Ok(s) => s,
                        Err(_) => break,
                    };

                    let raw_params = &full_ansi[2..full_ansi.len() - 1];

                    // validation: ANSI SGR parameters can contain ONLY numbers and ';' (or ';', ':')
                    // (if there are spaces or regular letters, it's a false 'm' in the text)
                    if !raw_params
                        .bytes()
                        .all(|b| b.is_ascii_digit() || b == b';' || b == b':')
                    {
                        // this is a regular text with the letter 'm', we continue to search for the real ANSI to the left
                        break;
                    }

                    let param_parts: Vec<&str> = raw_params.split(';').collect();
                    let mut active_in_this_group: Option<String> = None;

                    for &part in &param_parts {
                        if part == "38" {
                            active_in_this_group = Some(full_ansi.to_string());
                            break;
                        } else if part == "0" || part == "39" {
                            active_in_this_group = None;
                        } else if let Ok(code) = part.parse::<u8>() {
                            if (30..=37).contains(&code) || (90..=97).contains(&code) {
                                active_in_this_group = Some(format!("\x1b[{code}m"));
                            }
                        }
                    }

                    if let Some(color) = active_in_this_group {
                        return Some(color);
                    } else if param_parts.contains(&"0") || param_parts.contains(&"39") {
                        return None;
                    }

                    // there were only styles in the group (bold/italic/bg) — we move to the left to j
                    i = j;
                    break;
                }
            }
        }
    }

    None
}

/// calculates the visible display width of a string in terminal grid columns, ignoring ANSI escape sequences.
///
/// # Arguments
///
/// * `text` - The input string slice, potentially containing ANSI codes or tabs.
///
/// # Details
///
/// Automatically expands tab characters (`\t`) into 4 spaces before measuring width via `unicode-width`.
pub fn visible_width(text: &str) -> usize {
    let expanded = expand_tabs(text, 4);
    let clean = strip_ansi_escapes::strip_str(&expanded);
    clean.width()
}

/// replaces tab characters (`\t`) in a string with a specified number of spaces.
///
/// # Arguments
///
/// * `text` - The target string slice.
/// * `tab_width` - The number of spaces to insert per tab character.
pub fn expand_tabs(text: &str, tab_width: usize) -> String {
    let mut result = String::with_capacity(text.len());
    for ch in text.chars() {
        if ch == '\t' {
            result.push_str(&" ".repeat(tab_width));
        } else {
            result.push(ch);
        }
    }
    result
}

/// wraps raw terminal text to fit within a specified maximum display width.
///
/// # Arguments
///
/// * `text` - The string containing paragraph(s) to wrap.
/// * `max_width` - Maximum allowed visible width in terminal columns per line.
///
/// # Details
///
/// Normalizes newline and carriage return variants (`\r\n`, `\r`), converts tabs to 4 spaces,
/// and breaks lines while maintaining ANSI color states across line breaks.
pub fn wrap_terminal_text(text: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return Vec::new();
    }

    let normalized = text
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\t', "    ");

    let mut lines = Vec::new();

    for line_str in normalized.lines() {
        let wrapped = wrap_paragraph(line_str, max_width);

        lines.extend(wrapped);
    }

    if lines.is_empty() {
        lines.push(String::new());
    }

    lines
}

/// wraps a single paragraph of text while preserving ANSI color state across line wraps.
///
/// # Arguments
///
/// * `paragraph` - A single line of text without newline characters.
/// * `max_width` - Maximum target visible display width.
pub fn wrap_paragraph(paragraph: &str, max_width: usize) -> Vec<String> {
    if paragraph.is_empty() {
        return vec![String::new()];
    }

    let mut out = Vec::new();
    let mut current = String::new();
    let mut active_ansi = String::new();

    for token in paragraph.split(' ') {
        let token_w = visible_width(token);

        if current.is_empty() {
            if !active_ansi.is_empty() && !token.starts_with('\x1b') {
                current.push_str(&active_ansi);
            }

            if token_w <= max_width {
                current.push_str(token);
            } else {
                let parts = split_token_by_width(token, max_width);

                if let Some((last, rest)) = parts.split_last() {
                    out.extend(rest.iter().cloned());

                    current = last.clone();
                }
            }
        } else {
            let current_w = visible_width(&current);

            let needed = 1 + token_w;

            if current_w + needed <= max_width {
                current.push(' ');

                current.push_str(token);
            } else {
                if let Some(color) = get_active_text_color(&current) {
                    active_ansi = color;
                } else {
                    active_ansi.clear();
                }

                out.push(current);

                current = active_ansi.clone();

                if token_w <= max_width {
                    current.push_str(token);
                } else {
                    let parts = split_token_by_width(token, max_width);

                    if let Some((last, rest)) = parts.split_last() {
                        out.extend(rest.iter().cloned());

                        current = last.clone();
                    }
                }
            }
        }
    }

    if !current.is_empty() {
        out.push(current);
    }

    out
}

/// splits a single word or continuous token into character chunks based on terminal display width.
///
/// # Arguments
///
/// * `token` - The string token/word to split (may include ANSI escape sequences).
/// * `max_width` - Maximum display width allowed for each resulting chunk.
///
/// # Details
///
/// Ensures ANSI sequences are preserved and not split mid-escape sequence.
pub fn split_token_by_width(token: &str, max_width: usize) -> Vec<String> {
    if max_width == 0 {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut current = String::new();
    let mut current_visible_width = 0;
    let mut chars = token.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1B' {
            current.push(ch);

            if let Some(&'[') = chars.peek() {
                current.push(chars.next().unwrap());

                while let Some(&c) = chars.peek() {
                    current.push(c);

                    chars.next();

                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            }

            continue;
        }

        let ch_s = ch.to_string();
        let ch_w = visible_width(&ch_s);

        if current_visible_width + ch_w > max_width && !current.is_empty() {
            out.push(current);

            current = String::new();

            current_visible_width = 0;
        }

        current.push(ch);

        current_visible_width += ch_w;
    }

    if !current.is_empty() {
        out.push(current);
    }

    out
}

/// pads and wraps Markdown table cell text according to column constraints and alignment settings.
///
/// # Arguments
///
/// * `text` - Raw cell content string.
/// * `target_width` - Exact column width to pad/wrap against.
/// * `align` - The alignment rule (left, right, center) applied per wrapped line.
/// * `is_header` - If `true`, applies bold formatting to the output string lines.
#[cfg(feature = "markdown")]
pub fn pad_and_wrap_cell(
    text: &str,
    target_width: usize,
    align: markdown::mdast::AlignKind,
    is_header: bool,
) -> Vec<String> {
    let current_w = visible_width(text);

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

    for word in text.split(' ') {
        let word_w = visible_width(word);
        let needed = word_w + if current_len > 0 { 1 } else { 0 };

        if current_len + needed > target_width {
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

            if word_w > target_width {
                let parts = split_token_by_width(word, target_width);

                if let Some((last, rest)) = parts.split_last() {
                    for part in rest {
                        let aligned = apply_alignment(part, target_width, align);

                        result.push(if is_header {
                            aligned.bold().to_string()
                        } else {
                            aligned
                        });
                    }

                    current_line = last.clone();

                    current_len = visible_width(&current_line);
                }
            } else {
                current_line.push_str(word);

                current_len = word_w;
            }
        } else {
            if !current_line.is_empty() {
                current_line.push(' ');

                current_len += 1;
            }

            current_line.push_str(word);
            current_len += word_w;
        }
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

/// applies whitespace padding to a text string based on the requested alignment kind.
///
/// # Arguments
///
/// * `text` - Target text string slice.
/// * `target_width` - Total expected column length after padding.
/// * `align` - Alignment mode (`Right`, `Center`, or `Left`/`None`).
#[cfg(feature = "markdown")]
pub fn apply_alignment(
    text: &str,
    target_width: usize,
    align: markdown::mdast::AlignKind,
) -> String {
    let current_w = visible_width(text);

    let total_pad = target_width.saturating_sub(current_w);

    match align {
        markdown::mdast::AlignKind::Right => {
            format!("{}{}", " ".repeat(total_pad), text)
        }

        markdown::mdast::AlignKind::Center => {
            let left_pad = total_pad / 2;
            let right_pad = total_pad - left_pad;

            format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
        }

        markdown::mdast::AlignKind::Left | markdown::mdast::AlignKind::None => {
            format!("{}{}", text, " ".repeat(total_pad))
        }
    }
}

/// linearly interpolates between two colors using a factor `t`.
///
/// # Arguments
///
/// * `start` - Initial `Color` variant (must be `Color::Rgb`).
/// * `end` - Target `Color` variant (must be `Color::Rgb`).
/// * `t` - Interpolation step factor, typically clamped between `0.0` and `1.0`.
///
/// # Details
///
/// Returns `end` directly if either color uses non-RGB variants (e.g., standard ANSI 16-color palette).
pub fn lerp_color(start: Color, end: Color, t: f32) -> Color {
    match (start, end) {
        (
            Color::Rgb {
                r: r1,
                g: g1,
                b: b1,
            },
            Color::Rgb {
                r: r2,
                g: g2,
                b: b2,
            },
        ) => Color::Rgb {
            r: (r1 as f32 + (r2 as f32 - r1 as f32) * t) as u8,
            g: (g1 as f32 + (g2 as f32 - g1 as f32) * t) as u8,
            b: (b1 as f32 + (b2 as f32 - b1 as f32) * t) as u8,
        },
        _ => end,
    }
}

/// Truncates the string `s` to the specified visible width `max_width` (in terminal columns).
///
/// Preserves all ANSI escape sequences encountered before the width limit is reached.
/// If the truncation occurs within a colored/styled section, a style reset (`\x1b[0m`) is added to the end to reset the formatting for the subsequent text.
pub fn truncate_str(s: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }

    if visible_width(s) <= max_width {
        return s.to_string();
    }

    let mut result = String::with_capacity(s.len());
    let mut current_width = 0;
    let mut chars = s.chars().peekable();
    let mut is_styled = false;

    while let Some(ch) = chars.next() {
        // parsing ANSI escape sequences
        if ch == '\x1B' {
            result.push(ch);

            if let Some(&'[') = chars.peek() {
                result.push(chars.next().unwrap());

                let mut sequence_body = String::new();
                while let Some(&c) = chars.peek() {
                    result.push(c);
                    sequence_body.push(c);
                    chars.next();

                    // ANSI CSI sequences end with the letter
                    if c.is_ascii_alphabetic() {
                        // if encounter a reset (\x1b[0m or \x1b[m)
                        if c == 'm' {
                            let params = &sequence_body[..sequence_body.len() - 1];
                            if params.is_empty() || params == "0" || params.ends_with(";0") {
                                is_styled = false;
                            } else {
                                is_styled = true;
                            }
                        }
                        break;
                    }
                }
            }
            continue;
        }

        // calculating the visible width of the current character
        let ch_s = ch.to_string();
        let ch_w = visible_width(&ch_s);

        // if adding a character exceeds the allowed width, stop adding characters.
        if current_width + ch_w > max_width {
            break;
        }

        result.push(ch);
        current_width += ch_w;
    }

    // if the text was cut off inside a styled block, we add an ANSI reset.
    if is_styled {
        result.push_str("\x1b[0m");
    }

    result
}
