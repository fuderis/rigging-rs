use crate::render::{block::Block, widget::Widget};

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::{
    fmt::Display,
    sync::atomic::{AtomicUsize, Ordering},
};
use unicode_width::UnicodeWidthStr;

/// A terminal text input component supporting single-line and multi-line modes,
/// horizontal/vertical scrolling, masking for passwords, and custom key bindings.
pub struct Input {
    /// Default fallback value returned when input is empty.
    default: Option<String>,
    /// Placeholder text displayed when no input is provided.
    placeholder: Option<String>,
    /// Mask input characters with asterisks if enabled.
    secret: bool,
    /// Enable multi-line input editing.
    multiline: bool,
    /// Minimum height of the rendering viewport.
    min_height: usize,
    /// Maximum height of the rendering viewport.
    max_height: usize,

    /// Text lines stored as a vector of strings.
    pub(crate) lines: Vec<String>,
    /// Zero-based active cursor line index.
    pub(crate) cursor_line: usize,
    /// Zero-based active cursor column index.
    pub(crate) cursor_col: usize,

    /// Horizontal scroll offset tracking.
    pub(crate) h_scroll: AtomicUsize,
    /// Vertical scroll offset tracking.
    pub(crate) v_scroll: AtomicUsize,

    /// State flag indicating input submission.
    finished: bool,
    /// Flag indicating whether the widget's internal state was mutated.
    is_changed: bool,
}

impl Input {
    /// Creates a new `Input` instance wrapped in a renderable [`Block`].
    ///
    /// # Returns
    /// A configurable [`Block<Input>`] wrapper around the initialized input component.
    pub fn new() -> Block<Self> {
        let input = Self {
            default: None,
            placeholder: None,
            secret: false,
            multiline: false,
            min_height: 1,
            max_height: 5,
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
            h_scroll: AtomicUsize::new(0),
            v_scroll: AtomicUsize::new(0),
            finished: false,
            is_changed: true,
        };

        Block::new(input)
    }

    /// Calculates the relative X position (column offset) of the cursor within the visible content area.
    ///
    /// Takes unicode character widths into account to properly handle multi-byte and wide characters.
    ///
    /// # Returns
    /// The visible column index relative to the left edge of the input.
    pub fn cursor_rel_x(&self) -> usize {
        let h_scroll = self.h_scroll.load(Ordering::Relaxed);
        if self.cursor_col <= h_scroll {
            return 0;
        }

        let current_line = &self.lines[self.cursor_line];
        let visible_chars: String = current_line
            .chars()
            .skip(h_scroll)
            .take(self.cursor_col - h_scroll)
            .collect();

        UnicodeWidthStr::width(visible_chars.as_str())
    }

    /// Calculates the relative Y position (row offset) of the cursor within the visible viewport.
    ///
    /// # Returns
    /// The zero-based visible row index relative to the top of the input area.
    pub fn cursor_rel_y(&self) -> usize {
        let v_scroll = self.v_scroll.load(Ordering::Relaxed);
        self.cursor_line.saturating_sub(v_scroll)
    }

    /// adjusts horizontal and vertical viewport scroll offsets based on the current cursor position.
    fn adjust_scroll(&self, visible_width: usize) {
        let mut v_scroll = self.v_scroll.load(Ordering::Relaxed);
        let h_scroll = self.h_scroll.load(Ordering::Relaxed);

        if self.cursor_line < v_scroll {
            v_scroll = self.cursor_line;
        } else if self.cursor_line >= v_scroll + self.max_height {
            v_scroll = self.cursor_line - self.max_height + 1;
        }

        let total_lines = self.lines.len();
        if total_lines <= self.max_height {
            v_scroll = 0;
        } else if v_scroll + self.max_height > total_lines {
            v_scroll = total_lines.saturating_sub(self.max_height);
        }

        self.v_scroll.store(v_scroll, Ordering::Relaxed);

        let line_len = char_count(&self.lines[self.cursor_line]);
        let mut new_h_scroll = h_scroll;

        if visible_width > 0 {
            if self.cursor_col < new_h_scroll {
                new_h_scroll = self.cursor_col;
            } else if self.cursor_col >= new_h_scroll + visible_width {
                new_h_scroll = self.cursor_col - visible_width + 1;
            }

            if line_len > visible_width {
                let max_possible_scroll = line_len.saturating_sub(visible_width);
                if new_h_scroll > max_possible_scroll {
                    new_h_scroll = max_possible_scroll;
                }
            } else {
                new_h_scroll = 0;
            }
        } else {
            new_h_scroll = 0;
        }

        self.h_scroll.store(new_h_scroll, Ordering::Relaxed);
    }
}

impl Widget for Input {
    type Output = String;

    /// checks if the input interaction is finished/submitted.
    fn is_finished(&self) -> bool {
        self.finished
    }

    /// handles terminal window resize events by setting the redraw flag.
    fn on_resize(&mut self, _rows: u16, _cols: u16) {
        self.is_changed = true;
    }

    /// extracts the final string result from the input, falling back to the default value if empty.
    fn extract_output(self) -> Self::Output {
        let res = self.lines.join("\n").trim().to_string();
        if res.is_empty() {
            self.default.unwrap_or_default()
        } else {
            res
        }
    }

    /// processes incoming key events for editing, cursor movement, navigation, and submission.
    fn handle_key(&mut self, key: KeyEvent) {
        let KeyEvent {
            code, modifiers, ..
        } = key;

        let is_ctrl = modifiers.contains(KeyModifiers::CONTROL);
        let is_alt = modifiers.contains(KeyModifiers::ALT);

        let is_ctrl_j = is_ctrl && code == KeyCode::Char('j');
        let is_raw_enter = code == KeyCode::Enter;

        let should_submit = if self.multiline {
            (is_alt && is_raw_enter) || is_ctrl_j
        } else {
            is_raw_enter || is_ctrl_j
        };

        if should_submit {
            self.finished = true;
            self.is_changed = true;
            return;
        }

        let is_alt_combo = |target_char: char, require_shift: bool| -> bool {
            if !is_alt {
                return false;
            }
            let has_shift = modifiers.contains(KeyModifiers::SHIFT);
            if require_shift != has_shift {
                return false;
            }
            match code {
                KeyCode::Char(ch) => ch.to_ascii_lowercase() == target_char,
                _ => false,
            }
        };

        let is_home = code == KeyCode::Home
            || (is_ctrl && code == KeyCode::Char('a'))
            || is_alt_combo('h', true);

        let is_end = code == KeyCode::End
            || (is_ctrl && code == KeyCode::Char('e'))
            || is_alt_combo('l', true);

        let is_top = is_alt_combo('k', true);
        let is_bottom = is_alt_combo('j', true);

        let is_left = code == KeyCode::Left
            || (is_ctrl && code == KeyCode::Char('b'))
            || is_alt_combo('h', false);

        let is_right = code == KeyCode::Right
            || (is_ctrl && code == KeyCode::Char('f'))
            || is_alt_combo('l', false);

        let is_up = code == KeyCode::Up
            || (is_ctrl && code == KeyCode::Char('p'))
            || is_alt_combo('k', false);

        let is_down = code == KeyCode::Down
            || (is_ctrl && code == KeyCode::Char('n'))
            || is_alt_combo('j', false);

        let is_backspace = code == KeyCode::Backspace || (is_ctrl && code == KeyCode::Char('h'));
        let is_delete = code == KeyCode::Delete || (is_ctrl && code == KeyCode::Char('d'));
        let is_delete_word = is_ctrl && code == KeyCode::Char('w');

        // mark component state as changed upon handling any valid input event
        self.is_changed = true;

        if is_top {
            if self.multiline {
                self.cursor_line = 0;
                self.cursor_col = self
                    .cursor_col
                    .min(char_count(&self.lines[self.cursor_line]));
            }
        } else if is_bottom {
            if self.multiline && !self.lines.is_empty() {
                self.cursor_line = self.lines.len() - 1;
                self.cursor_col = self
                    .cursor_col
                    .min(char_count(&self.lines[self.cursor_line]));
            }
        } else if is_home {
            self.cursor_col = 0;
        } else if is_end {
            self.cursor_col = char_count(&self.lines[self.cursor_line]);
        } else if is_left {
            if self.cursor_col > 0 {
                self.cursor_col -= 1;
            } else if self.multiline && self.cursor_line > 0 {
                self.cursor_line -= 1;
                self.cursor_col = char_count(&self.lines[self.cursor_line]);
            }
        } else if is_right {
            let len = char_count(&self.lines[self.cursor_line]);
            if self.cursor_col < len {
                self.cursor_col += 1;
            } else if self.multiline && self.cursor_line + 1 < self.lines.len() {
                self.cursor_line += 1;
                self.cursor_col = 0;
            }
        } else if is_up {
            if self.multiline && self.cursor_line > 0 {
                self.cursor_line -= 1;
                self.cursor_col = self
                    .cursor_col
                    .min(char_count(&self.lines[self.cursor_line]));
            }
        } else if is_down {
            if self.multiline && self.cursor_line + 1 < self.lines.len() {
                self.cursor_line += 1;
                self.cursor_col = self
                    .cursor_col
                    .min(char_count(&self.lines[self.cursor_line]));
            }
        } else if is_backspace {
            if self.cursor_col > 0 {
                let idx = char_to_byte_idx(&self.lines[self.cursor_line], self.cursor_col - 1);
                self.lines[self.cursor_line].remove(idx);
                self.cursor_col -= 1;
            } else if self.multiline && self.cursor_line > 0 {
                let current_line = self.lines.remove(self.cursor_line);
                self.cursor_line -= 1;
                let prev_len = char_count(&self.lines[self.cursor_line]);
                self.lines[self.cursor_line].push_str(&current_line);
                self.cursor_col = prev_len;
            }
        } else if is_delete {
            let len = char_count(&self.lines[self.cursor_line]);
            if self.cursor_col < len {
                let idx = char_to_byte_idx(&self.lines[self.cursor_line], self.cursor_col);
                self.lines[self.cursor_line].remove(idx);
            } else if self.multiline && self.cursor_line + 1 < self.lines.len() {
                let next_line = self.lines.remove(self.cursor_line + 1);
                self.lines[self.cursor_line].push_str(&next_line);
            }
        } else if is_delete_word {
            if self.cursor_col > 0 {
                let line = &self.lines[self.cursor_line];
                let chars: Vec<(usize, char)> = line.char_indices().collect();

                let mut new_col = self.cursor_col;
                while new_col > 0 && chars[new_col - 1].1.is_whitespace() {
                    new_col -= 1;
                }
                while new_col > 0 && !chars[new_col - 1].1.is_whitespace() {
                    new_col -= 1;
                }

                let start_byte = if new_col < chars.len() {
                    chars[new_col].0
                } else {
                    line.len()
                };
                let end_byte = if self.cursor_col < chars.len() {
                    chars[self.cursor_col].0
                } else {
                    line.len()
                };

                self.lines[self.cursor_line].drain(start_byte..end_byte);
                self.cursor_col = new_col;
            }
        } else {
            match code {
                KeyCode::Enter if self.multiline => {
                    let byte_idx = char_to_byte_idx(&self.lines[self.cursor_line], self.cursor_col);
                    let right_part = self.lines[self.cursor_line][byte_idx..].to_string();
                    self.lines[self.cursor_line].truncate(byte_idx);
                    self.lines.insert(self.cursor_line + 1, right_part);
                    self.cursor_line += 1;
                    self.cursor_col = 0;
                }
                KeyCode::Char(c) if !is_ctrl && !is_alt => {
                    let byte_idx = char_to_byte_idx(&self.lines[self.cursor_line], self.cursor_col);
                    self.lines[self.cursor_line].insert(byte_idx, c);
                    self.cursor_col += 1;
                }
                _ => {}
            }
        }
    }

    /// returns true if the input's state has mutated since the last render pass.
    fn is_changed(&self) -> bool {
        self.is_changed
    }

    /// renders the visible line slices according to current viewport height, width, and scroll positions.
    fn render_content(&mut self, width: usize) -> Vec<String> {
        self.adjust_scroll(width);

        let visible_height = if self.multiline {
            self.lines.len().clamp(self.min_height, self.max_height)
        } else {
            1
        };

        let mut result = Vec::with_capacity(visible_height);
        let v_scroll = self.v_scroll.load(Ordering::Relaxed);
        let h_scroll = self.h_scroll.load(Ordering::Relaxed);

        let is_entirely_empty = self.lines.len() == 1 && self.lines[0].is_empty();

        for row in 0..visible_height {
            let line_idx = v_scroll + row;
            if line_idx < self.lines.len() {
                let line = &self.lines[line_idx];

                if is_entirely_empty && line_idx == 0 {
                    let placeholder = self.placeholder.as_deref().unwrap_or("");
                    let char_start = h_scroll.min(char_count(placeholder));
                    let byte_start = char_to_byte_idx(placeholder, char_start);
                    result.push(clip_to_width(&placeholder[byte_start..], width));
                } else if self.secret {
                    let stars = "*".repeat(char_count(line));
                    let char_start = h_scroll.min(stars.len());
                    result.push(clip_to_width(&stars[char_start..], width));
                } else {
                    let char_start = h_scroll.min(char_count(line));
                    let byte_start = char_to_byte_idx(line, char_start);
                    result.push(clip_to_width(&line[byte_start..], width));
                }
            }
        }

        self.is_changed = false;
        result
    }

    /// returns the relative (x, y) coordinates of the cursor in the visible content box.
    fn cursor_position(&self) -> Option<(usize, usize)> {
        Some((self.cursor_rel_x(), self.cursor_rel_y()))
    }

    /// indicates whether the terminal cursor should be displayed.
    fn show_cursor(&self) -> bool {
        true
    }
}

impl Block<Input> {
    /// Sets a default fallback value to be used when the user submits an empty input.
    ///
    /// # Arguments
    /// * `val` - The fallback text value convertibles into a [`String`].
    pub fn default_val(mut self, val: impl Display) -> Self {
        self.inner.default = Some(val.to_string());
        self
    }

    /// Sets placeholder text to be displayed when the input is empty.
    ///
    /// # Arguments
    /// * `text` - The placeholder text convertible into a [`String`].
    pub fn placeholder(mut self, text: impl Display) -> Self {
        self.inner.placeholder = Some(text.to_string());
        self
    }

    /// Enables or disables input masking (e.g., replacing text with asterisks for password fields).
    ///
    /// # Arguments
    /// * `enabled` - `true` to mask characters, `false` to display raw input.
    pub fn secret(mut self, enabled: bool) -> Self {
        self.inner.secret = enabled;
        self
    }

    /// Enables or disables multi-line input mode.
    ///
    /// # Arguments
    /// * `enabled` - `true` to allow multi-line editing and scrolling, `false` for single-line mode.
    pub fn multiline(mut self, enabled: bool) -> Self {
        self.inner.multiline = enabled;
        self
    }
}

/// clips a string slice so that its visual width does not exceed `max_width`.
fn clip_to_width(slice: &str, max_width: usize) -> String {
    let mut current_width = 0;
    let mut end_byte = 0;

    for (idx, ch) in slice.char_indices() {
        let ch_width = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if current_width + ch_width > max_width {
            break;
        }
        current_width += ch_width;
        end_byte = idx + ch.len_utf8();
    }

    slice[..end_byte].to_string()
}

/// returns the total character count (code points) of a string slice.
#[inline]
fn char_count(s: &str) -> usize {
    s.chars().count()
}

/// converts a zero-based character index to a byte offset within the given string slice.
#[inline]
fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(s.len())
}
