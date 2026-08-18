use crate::render::{block::Block, widget::Widget};
#[cfg(feature = "buffer")]
use atoman::prelude::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
#[cfg(feature = "buffer")]
use std::{collections::HashMap, sync::Arc};
use std::{
    fmt::Display,
    sync::atomic::{AtomicUsize, Ordering},
};
use unicode_width::UnicodeWidthStr;

/// Global storage for command histories separated by buffer ID (`u64`).
#[cfg(feature = "buffer")]
static COMMAND_HISTORIES: State<HashMap<u64, Vec<Arc<String>>>> = State::new(HashMap::new);

/// A text input widget supporting single-line and multi-line modes, secret masking,
/// scrolling, and command history buffers.
pub struct Input {
    /// Default fallback value returned if the input is empty on submit.
    default: Option<String>,
    /// Placeholder text rendered when the input buffer is empty.
    placeholder: Option<String>,
    /// Indicates whether character masking (asterisks) is active.
    secret: bool,
    /// Indicates whether multi-line editing is enabled.
    multiline: bool,
    /// Minimum allowed height (in rows) for multi-line display.
    min_height: usize,
    /// Maximum allowed height (in rows) for multi-line display.
    max_height: usize,

    /// Text lines contained in the input field.
    pub(crate) lines: Vec<String>,
    /// Active line index of the cursor.
    pub(crate) cursor_line: usize,
    /// Active column position (in character count) of the cursor.
    pub(crate) cursor_col: usize,

    /// Horizontal scrolling offset (in characters).
    pub(crate) h_scroll: AtomicUsize,
    /// Vertical scrolling offset (in lines).
    pub(crate) v_scroll: AtomicUsize,

    /// Indicates whether the input interaction has ended.
    finished: bool,
    /// Tracks if the widget state changed and requires re-rendering.
    is_changed: bool,

    /// ID of the history buffer being used (`None` disables history).
    #[cfg(feature = "buffer")]
    buffer_id: Option<u64>,
    /// Maximum limit of saved commands in history.
    #[cfg(feature = "buffer")]
    buffer_limit: Option<usize>,
    /// Current index within the navigated command history.
    #[cfg(feature = "buffer")]
    history_idx: Option<usize>,
    /// Draft of user input stored before navigating through history.
    #[cfg(feature = "buffer")]
    saved_draft: String,
}

impl Input {
    /// Creates a new `Input` widget wrapped inside a [`Block`].
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
            #[cfg(feature = "buffer")]
            buffer_id: None,
            #[cfg(feature = "buffer")]
            buffer_limit: None,
            #[cfg(feature = "buffer")]
            history_idx: None,
            #[cfg(feature = "buffer")]
            saved_draft: String::new(),
        };

        Block::new(input)
    }

    /// Clears the history buffer associated with the specified `id`.
    #[cfg(feature = "buffer")]
    pub fn remove_buffer(id: u64) {
        let mut histories = COMMAND_HISTORIES.dirty_lock();
        histories.remove(&id);
    }

    /// Saves current input to history if `buffer_id` is specified and string starts with `/`.
    #[cfg(feature = "buffer")]
    fn save_to_history(&self) {
        let buffer_id = match self.buffer_id {
            Some(id) => id,
            None => return,
        };

        let full_text = self.lines.join("\n");
        let trimmed = full_text.trim();

        if trimmed.starts_with('/') {
            let mut histories = COMMAND_HISTORIES.dirty_lock();
            let history = histories.entry(buffer_id).or_default();

            // avoid consecutive duplicate entries
            if history.last().map_or(true, |last| last.as_str() != trimmed) {
                history.push(Arc::new(trimmed.to_string()));

                // enforce capacity limits on history buffer
                if let Some(limit) = self.buffer_limit {
                    if limit > 0 && history.len() > limit {
                        let drain_count = history.len() - limit;
                        history.drain(0..drain_count);
                    }
                }
            }
        }
    }

    /// Returns the relative visual X position of the cursor accounting for horizontal scroll and double-width characters.
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

    /// Returns the relative visual Y position of the cursor relative to the vertical viewport scroll.
    pub fn cursor_rel_y(&self) -> usize {
        let v_scroll = self.v_scroll.load(Ordering::Relaxed);
        self.cursor_line.saturating_sub(v_scroll)
    }

    /// Adjusts horizontal and vertical scroll offsets based on cursor position and viewport dimensions.
    fn adjust_scroll(&self, visible_width: usize) {
        let mut v_scroll = self.v_scroll.load(Ordering::Relaxed);
        let h_scroll = self.h_scroll.load(Ordering::Relaxed);

        if self.cursor_line < v_scroll {
            v_scroll = self.cursor_line;
        } else if self.cursor_line >= self.max_height {
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

    fn is_finished(&self) -> bool {
        self.finished
    }

    fn on_resize(&mut self, _rows: u16, _cols: u16) {
        self.is_changed = true;
    }

    fn extract_output(self) -> Self::Output {
        let res = self.lines.join("\n").trim().to_string();
        if res.is_empty() {
            self.default.unwrap_or_default()
        } else {
            res
        }
    }

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
            #[cfg(feature = "buffer")]
            self.save_to_history();
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

        self.is_changed = true;

        // --- process history navigation ---
        #[cfg(feature = "buffer")]
        if let Some(buffer_id) = self.buffer_id {
            if is_up || is_down {
                let can_navigate = if self.multiline {
                    (is_up && self.cursor_line == 0)
                        || (is_down && self.cursor_line == self.lines.len().saturating_sub(1))
                } else {
                    true
                };

                if can_navigate {
                    let histories = COMMAND_HISTORIES.dirty_get();
                    if let Some(history) = histories.get(&buffer_id) {
                        if !history.is_empty() {
                            if is_up {
                                match self.history_idx {
                                    None => {
                                        self.saved_draft = self.lines.join("\n");
                                        let new_idx = history.len() - 1;
                                        self.history_idx = Some(new_idx);
                                        self.lines = vec![history[new_idx].to_string()];
                                    }
                                    Some(idx) if idx > 0 => {
                                        let new_idx = idx - 1;
                                        self.history_idx = Some(new_idx);
                                        self.lines = vec![history[new_idx].to_string()];
                                    }
                                    _ => {}
                                }
                            } else if is_down {
                                if let Some(idx) = self.history_idx {
                                    if idx + 1 < history.len() {
                                        let new_idx = idx + 1;
                                        self.history_idx = Some(new_idx);
                                        self.lines = vec![history[new_idx].to_string()];
                                    } else {
                                        self.history_idx = None;
                                        self.lines = self
                                            .saved_draft
                                            .split('\n')
                                            .map(String::from)
                                            .collect();
                                    }
                                }
                            }

                            self.cursor_line = self.lines.len() - 1;
                            self.cursor_col = char_count(&self.lines[self.cursor_line]);
                            return;
                        }
                    }
                }
            }
        }

        // manual editing resets history navigation mode
        #[cfg(feature = "buffer")]
        if is_backspace || is_delete || is_delete_word || matches!(code, KeyCode::Char(_)) {
            self.history_idx = None;
        }

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

    fn is_changed(&self) -> bool {
        self.is_changed
    }

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

    fn cursor_position(&self) -> Option<(usize, usize)> {
        Some((self.cursor_rel_x(), self.cursor_rel_y()))
    }

    fn show_cursor(&self) -> bool {
        true
    }
}

impl Block<Input> {
    /// Sets the default fallback value returned when the input field is left empty upon submission.
    pub fn default_val(mut self, val: impl Display) -> Self {
        self.inner.default = Some(val.to_string());
        self
    }

    /// Sets the placeholder text displayed when the input field is empty.
    pub fn placeholder(mut self, text: impl Display) -> Self {
        self.inner.placeholder = Some(text.to_string());
        self
    }

    /// Enables or disables secret masking mode (replaces characters with asterisks).
    pub fn secret(mut self, enabled: bool) -> Self {
        self.inner.secret = enabled;
        self
    }

    /// Enables or disables multi-line input mode.
    pub fn multiline(mut self, enabled: bool) -> Self {
        self.inner.multiline = enabled;
        self
    }

    /// Enables history buffer for the input using the provided `id` and optional entry limit.
    ///
    /// # Example
    /// ```rust
    /// // Without entry limit:
    /// Input::new().use_buffer(1, None);
    ///
    /// // Limited to the 50 most recent commands:
    /// Input::new().use_buffer(1, Some(50));
    /// ```
    #[cfg(feature = "buffer")]
    pub fn use_buffer(mut self, id: u64, limit: Option<usize>) -> Self {
        self.inner.buffer_id = Some(id);
        self.inner.buffer_limit = limit;
        self
    }
}

/// Clips a string slice to fit within a specified display width using unicode character widths.
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

/// Returns the total character count (Unicode scalar values) of a string slice.
#[inline]
fn char_count(s: &str) -> usize {
    s.chars().count()
}

/// Converts a character index to its corresponding byte index within a string slice.
#[inline]
fn char_to_byte_idx(s: &str, char_idx: usize) -> usize {
    s.char_indices()
        .nth(char_idx)
        .map(|(idx, _)| idx)
        .unwrap_or(s.len())
}
