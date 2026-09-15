use super::{
    context::{Command, Context, SubWidgetPosition},
    guard::TerminalGuard,
    widget::Widget,
};
use crate::{
    style::{Align, BorderStyle, Margin, Padding, Title},
    utils::ansi,
};

use atoman::State;
use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    queue,
    style::{Color, Stylize},
    terminal,
};
use std::{
    borrow::Cow,
    fmt::{Display, Write as FmtWrite},
    io::{self, BufWriter, Write},
    time::Duration,
};
use tokio::sync::mpsc;
use tokio::time::interval;

/// Cache for horizontal borders.
#[derive(Default)]
struct CachedBorder {
    key: (usize, BorderStyle, Option<Color>, Option<Color>),
    line: String,
}

/// Widget wrapper with built-in styling and rendering methods.
pub struct Block<W: Widget> {
    /// Inner widget.
    pub inner: W,
    /// Widget state (controlled by user handler).
    pub state: atoman::State<W::State>,
    /// Set to true if widget has handler.
    pub has_handler: bool,

    /// State of freezing redrawing.
    pub is_frozen: bool,
    /// Forced completion flag.
    pub is_finished_signal: bool,
    /// Terminal cursor display flag.
    pub show_cursor: bool,

    // --- Render Channels ---
    cmd_tx: mpsc::UnboundedSender<Command>,
    cmd_rx: mpsc::UnboundedReceiver<Command>,
    event_tx: mpsc::UnboundedSender<W::Event>,
    event_rx: mpsc::UnboundedReceiver<W::Event>,

    // --- Widget Stylization ---
    pub titles: Vec<Title>,
    pub border: BorderStyle,
    pub border_color: Option<Color>,
    pub bg_color: Option<Color>,
    pub blink_color: Option<Color>,
    pub blink_duration: Duration,
    pub margin: Margin,
    pub padding: Padding,
    pub remove_on_finish: bool,

    pub min_width: Option<usize>,
    pub max_width: Option<usize>,
    pub min_height: Option<usize>,
    pub max_height: Option<usize>,
    pub truncate_lines: bool,

    pub no_top_border: bool,
    pub no_bottom_border: bool,

    cached_top_border: Option<CachedBorder>,
    cached_bottom_border: Option<CachedBorder>,
    last_rendered_frame: Option<Vec<String>>,

    pub(crate) on_key: Option<Box<dyn for<'a> FnMut(&'a mut W, KeyEvent) + Send + Sync + 'static>>,
    pub(crate) on_exit: Option<Box<dyn FnMut() + Send + Sync + 'static>>,
}

impl<W: Widget> Block<W> {
    /// Creates new `Block` with specified widget and initial state.
    pub fn new(inner: W, initial_state: W::State) -> Self {
        let state = State::from(initial_state);
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        Self {
            inner,
            state,
            has_handler: false,

            cmd_tx,
            cmd_rx,
            event_tx,
            event_rx,

            is_frozen: false,
            is_finished_signal: false,
            show_cursor: false,

            titles: Vec::new(),
            border: BorderStyle::None,
            border_color: None,
            bg_color: None,
            blink_color: None,
            blink_duration: Duration::from_millis(600),
            margin: Margin::default(),
            padding: Padding {
                right: 1,
                ..Default::default()
            },
            remove_on_finish: false,

            min_width: None,
            max_width: None,
            min_height: None,
            max_height: None,
            truncate_lines: false,

            no_top_border: false,
            no_bottom_border: false,

            cached_top_border: None,
            cached_bottom_border: None,
            last_rendered_frame: None,

            on_key: None,
            on_exit: None,
        }
    }

    /// Registers asynchronous user handler.
    ///
    /// A `Context` is passed to the task, which owns the `StateGuard` for the lifetime of the context.
    pub fn handler<F, Fut>(mut self, f: F) -> Self
    where
        F: FnOnce(Context<W::State, W::Event>) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        if self.has_handler {
            panic!("There is already a handler: allowed only one rendering handler!");
        }
        self.has_handler = true;

        let state_clone = self.state.clone();
        let cmd_tx = self.cmd_tx.clone();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            let guard = state_clone.lock().await;
            let ctx = Context {
                state: guard,
                tx: cmd_tx,
                event_tx,
            };
            f(ctx).await;
        });

        self
    }

    // --- Fluent Builders ---

    pub fn show_cursor(mut self, show: bool) -> Self {
        self.show_cursor = show;
        self
    }

    pub fn on_key<F>(mut self, f: F) -> Self
    where
        F: FnMut(&mut W, KeyEvent) + Send + Sync + 'static,
    {
        self.on_key = Some(Box::new(f));
        self
    }

    pub fn on_exit<F>(mut self, f: F) -> Self
    where
        F: FnMut() + Send + Sync + 'static,
    {
        self.on_exit = Some(Box::new(f));
        self
    }

    pub fn width(mut self, width: usize) -> Self {
        self.min_width = Some(width);
        self.max_width = Some(width);
        self
    }

    pub fn min_width(mut self, width: usize) -> Self {
        self.min_width = Some(width);
        self
    }

    pub fn max_width(mut self, width: usize) -> Self {
        self.max_width = Some(width);
        self
    }

    pub fn height(mut self, height: usize) -> Self {
        self.min_height = Some(height);
        self.max_height = Some(height);
        self
    }

    pub fn min_height(mut self, height: usize) -> Self {
        self.min_height = Some(height);
        self
    }

    pub fn max_height(mut self, height: usize) -> Self {
        self.max_height = Some(height);
        self
    }

    pub fn truncate_lines(mut self, enable: bool) -> Self {
        self.truncate_lines = enable;
        self
    }

    pub fn clear_after(mut self, clear: bool) -> Self {
        self.remove_on_finish = clear;
        self
    }

    pub fn no_top_border(mut self, hide: bool) -> Self {
        self.no_top_border = hide;
        self
    }

    pub fn no_bottom_border(mut self, hide: bool) -> Self {
        self.no_bottom_border = hide;
        self
    }

    pub fn title(mut self, title: impl Display, align: Align) -> Self {
        self.titles.push(Title {
            text: title.to_string(),
            align,
        });
        self
    }

    pub fn background_color(mut self, color: Color) -> Self {
        self.bg_color = Some(color);
        self.invalidate_border_cache();
        self
    }

    pub fn blink_color(mut self, color: Color) -> Self {
        self.blink_color = Some(color);
        self
    }

    pub fn blink_duration(mut self, duration: Duration) -> Self {
        self.blink_duration = duration;
        self
    }

    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }

    pub fn padding_ver(mut self, padding: usize) -> Self {
        self.padding.top = padding;
        self.padding.bottom = padding;
        self
    }

    pub fn padding_hor(mut self, padding: usize) -> Self {
        self.padding.left = padding;
        self.padding.right = padding;
        self
    }

    pub fn padding_top(mut self, padding: usize) -> Self {
        self.padding.top = padding;
        self
    }

    pub fn padding_right(mut self, padding: usize) -> Self {
        self.padding.right = padding;
        self
    }

    pub fn padding_bottom(mut self, padding: usize) -> Self {
        self.padding.bottom = padding;
        self
    }

    pub fn padding_left(mut self, padding: usize) -> Self {
        self.padding.left = padding;
        self
    }

    pub fn margin(mut self, margin: Margin) -> Self {
        self.margin = margin;
        self
    }

    pub fn margin_ver(mut self, margin: usize) -> Self {
        self.margin.top = margin;
        self.margin.bottom = margin;
        self
    }

    pub fn margin_hor(mut self, margin: usize) -> Self {
        self.margin.left = margin;
        self.margin.right = margin;
        self
    }

    pub fn margin_top(mut self, margin: usize) -> Self {
        self.margin.top = margin;
        self
    }

    pub fn margin_right(mut self, margin: usize) -> Self {
        self.margin.right = margin;
        self
    }

    pub fn margin_bottom(mut self, margin: usize) -> Self {
        self.margin.bottom = margin;
        self
    }

    pub fn margin_left(mut self, margin: usize) -> Self {
        self.margin.left = margin;
        self
    }

    pub fn border_style(mut self, border: BorderStyle) -> Self {
        self.border = border;
        self.invalidate_border_cache();
        self
    }

    pub fn border_color(mut self, color: Color) -> Self {
        self.border_color = Some(color);
        self.invalidate_border_cache();
        self
    }

    #[inline]
    fn invalidate_border_cache(&mut self) {
        self.cached_top_border = None;
        self.cached_bottom_border = None;
    }

    // --- Size Calculations & Formatting ---

    #[inline]
    pub fn vertical_overhead(&self) -> usize {
        let has_border = !matches!(self.border, BorderStyle::None);
        let border_y = if has_border { 2 } else { 0 };
        border_y + self.padding.top + self.padding.bottom
    }

    #[inline]
    pub fn horizontal_overhead(&self) -> usize {
        let has_border = !matches!(self.border, BorderStyle::None);
        let border_x = if has_border { 2 } else { 0 };
        border_x + self.margin.left + self.margin.right + self.padding.left + self.padding.right
    }

    pub fn calculate_cursor_offset(&self) -> Option<(u16, u16)> {
        if !self.show_cursor {
            return None;
        }

        let (rel_x, rel_y) = self.inner.cursor_position()?;

        let has_top_titles = self
            .titles
            .iter()
            .any(|t| matches!(t.align, Align::TopLeft | Align::TopCenter | Align::TopRight));
        let has_top_line =
            !self.no_top_border && (self.border != BorderStyle::None || has_top_titles);
        let has_left_border = self.border != BorderStyle::None;

        let cursor_x =
            self.margin.left + if has_left_border { 1 } else { 0 } + self.padding.left + rel_x;
        let cursor_y =
            self.margin.top + if has_top_line { 1 } else { 0 } + self.padding.top + rel_y;

        Some((cursor_x as u16, cursor_y as u16))
    }

    #[inline]
    fn write_repeat_char(buf: &mut String, ch: char, count: usize) {
        for _ in 0..count {
            buf.push(ch);
        }
    }

    fn apply_bg_to_buf(&self, text: &str, buf: &mut String) {
        let Some(bg) = self.bg_color else {
            buf.push_str(text);
            return;
        };

        let bg_code = match bg {
            Color::Reset => "\x1b[49m".to_string(),
            Color::Black => "\x1b[40m".to_string(),
            Color::Red => "\x1b[41m".to_string(),
            Color::Green => "\x1b[42m".to_string(),
            Color::Yellow => "\x1b[43m".to_string(),
            Color::Blue => "\x1b[44m".to_string(),
            Color::Magenta => "\x1b[45m".to_string(),
            Color::Cyan => "\x1b[46m".to_string(),
            Color::White => "\x1b[47m".to_string(),
            Color::DarkGrey => "\x1b[100m".to_string(),
            Color::Rgb { r, g, b } => format!("\x1b[48;2;{};{};{}m", r, g, b),
            Color::AnsiValue(val) => format!("\x1b[48;5;{}m", val),
            _ => String::new(),
        };

        if bg_code.is_empty() {
            buf.push_str(text);
            return;
        }

        buf.push_str(&bg_code);

        let reset_with_bg = format!("\x1b[0m{}", bg_code);
        let mut start = 0;
        while let Some(pos) = text[start..].find("\x1b[0m") {
            let idx = start + pos;
            buf.push_str(&text[start..idx]);
            buf.push_str(&reset_with_bg);
            start = idx + 4;
        }
        buf.push_str(&text[start..]);

        buf.push_str("\x1b[49m");
    }

    fn render_border_line(
        &self,
        left_corner: char,
        right_corner: char,
        h_symbol: char,
        target_aligns: (Align, Align, Align),
        inner_w: usize,
        border_col: Color,
    ) -> String {
        let mut out = String::with_capacity(inner_w + self.margin.left + 32);
        Self::write_repeat_char(&mut out, ' ', self.margin.left);

        let filtered_titles: Vec<_> = self
            .titles
            .iter()
            .filter(|t| {
                t.align == target_aligns.0
                    || t.align == target_aligns.1
                    || t.align == target_aligns.2
            })
            .collect();

        let mut line_raw = String::with_capacity(inner_w + 32);

        if filtered_titles.is_empty() {
            let _ = write!(line_raw, "{}", left_corner.with(border_col));
            let mut h_str = String::with_capacity(inner_w);
            Self::write_repeat_char(&mut h_str, h_symbol, inner_w);
            let _ = write!(line_raw, "{}", h_str.with(border_col));
            let _ = write!(line_raw, "{}", right_corner.with(border_col));

            self.apply_bg_to_buf(&line_raw, &mut out);
            return out;
        }

        let lefts: Vec<_> = filtered_titles
            .iter()
            .filter(|t| t.align == target_aligns.0)
            .copied()
            .collect();
        let centers: Vec<_> = filtered_titles
            .iter()
            .filter(|t| t.align == target_aligns.1)
            .copied()
            .collect();
        let rights: Vec<_> = filtered_titles
            .iter()
            .filter(|t| t.align == target_aligns.2)
            .copied()
            .collect();

        let format_group = |group: &[&Title]| -> String {
            let mut s = String::new();
            for t in group {
                s.push_str(&t.text);
            }
            s
        };

        let left_str = format_group(&lefts);
        let center_str = format_group(&centers);
        let right_str = format_group(&rights);

        let left_w = ansi::visible_width(&left_str);
        let center_w = ansi::visible_width(&center_str);
        let right_w = ansi::visible_width(&right_str);

        let left_gap = if center_w > 0 {
            (inner_w.saturating_sub(center_w) / 2).saturating_sub(left_w)
        } else {
            inner_w.saturating_sub(left_w + right_w)
        };

        let right_gap = inner_w.saturating_sub(left_w + left_gap + center_w + right_w);

        let mut left_gap_str = String::with_capacity(left_gap);
        Self::write_repeat_char(&mut left_gap_str, h_symbol, left_gap);

        let mut right_gap_str = String::with_capacity(right_gap);
        Self::write_repeat_char(&mut right_gap_str, h_symbol, right_gap);

        let _ = write!(
            line_raw,
            "{}{}{}{}{}{}{}",
            left_corner.with(border_col),
            left_str,
            left_gap_str.with(border_col),
            center_str,
            right_gap_str.with(border_col),
            right_str,
            right_corner.with(border_col)
        );

        self.apply_bg_to_buf(&line_raw, &mut out);
        out
    }

    fn get_cached_border(&mut self, is_top: bool, inner_w: usize, border_col: Color) -> String {
        let key = (inner_w, self.border, self.border_color, self.bg_color);

        let cache_ref = if is_top {
            &self.cached_top_border
        } else {
            &self.cached_bottom_border
        };

        if let Some(c) = cache_ref {
            if c.key == key {
                return c.line.clone();
            }
        }

        let (tl, tr, bl, br, h, _) = self.border.as_chars();
        let line = if is_top {
            self.render_border_line(
                tl,
                tr,
                h,
                (Align::TopLeft, Align::TopCenter, Align::TopRight),
                inner_w,
                border_col,
            )
        } else {
            self.render_border_line(
                bl,
                br,
                h,
                (Align::BottomLeft, Align::BottomCenter, Align::BottomRight),
                inner_w,
                border_col,
            )
        };

        let cache_mut = if is_top {
            &mut self.cached_top_border
        } else {
            &mut self.cached_bottom_border
        };

        *cache_mut = Some(CachedBorder {
            key,
            line: line.clone(),
        });

        line
    }

    pub fn render_frame_with_viewport(
        &mut self,
        available_width: usize,
        viewport_height: Option<usize>,
        is_final: bool,
    ) -> Vec<String> {
        let state_arc = self.state.get();

        let pad = self.padding;
        let mar = self.margin;
        let border_col = self.border_color.unwrap_or(Color::DarkGrey);

        let has_top_titles = self
            .titles
            .iter()
            .any(|t| matches!(t.align, Align::TopLeft | Align::TopCenter | Align::TopRight));
        let has_bot_titles = self.titles.iter().any(|t| {
            matches!(
                t.align,
                Align::BottomLeft | Align::BottomCenter | Align::BottomRight
            )
        });

        let has_top_line =
            !self.no_top_border && (self.border != BorderStyle::None || has_top_titles);
        let has_bot_line =
            !self.no_bottom_border && (self.border != BorderStyle::None || has_bot_titles);
        let has_border = has_top_line || has_bot_line;

        let border_overhead = if has_border { 2 } else { 0 };
        let total_horiz_overhead = border_overhead + pad.left + pad.right + mar.left + mar.right;

        let safe_term_cols = available_width.saturating_sub(1);
        let mut max_allowed_inner = safe_term_cols.saturating_sub(total_horiz_overhead);

        if let Some(max_w) = self.max_width {
            let user_max_content = max_w.saturating_sub(border_overhead + pad.left + pad.right);
            max_allowed_inner = max_allowed_inner.min(user_max_content);
        }

        let vert_overhead = mar.top
            + mar.bottom
            + pad.top
            + pad.bottom
            + if has_top_line { 1 } else { 0 }
            + if has_bot_line { 1 } else { 0 };

        let max_text_rows = self
            .max_height
            .or(viewport_height)
            .map(|h| h.saturating_sub(vert_overhead));

        let raw_lines = self.inner.render_frame(
            &state_arc,
            if self.truncate_lines {
                None
            } else {
                Some(max_allowed_inner)
            },
            max_text_rows,
            is_final,
        );

        let mut content_lines = Vec::with_capacity(raw_lines.len());
        for line in raw_lines {
            content_lines.extend(line.split('\n').map(str::to_owned));
        }

        let effective_max_h = self.max_height.or(viewport_height);
        if let Some(max_h) = effective_max_h {
            let max_text_rows = max_h.saturating_sub(vert_overhead);
            if content_lines.len() > max_text_rows {
                let skip_count = content_lines.len().saturating_sub(max_text_rows);
                content_lines = content_lines.into_iter().skip(skip_count).collect();
            }
        }

        let actual_content_w = content_lines
            .iter()
            .map(|l| ansi::visible_width(l))
            .max()
            .unwrap_or(0);

        let calc_titles_w = |aligns: (Align, Align, Align)| -> usize {
            self.titles
                .iter()
                .filter(|t| t.align == aligns.0 || t.align == aligns.1 || t.align == aligns.2)
                .map(|t| ansi::visible_width(&t.text))
                .sum()
        };

        let top_w = calc_titles_w((Align::TopLeft, Align::TopCenter, Align::TopRight));
        let bot_w = calc_titles_w((Align::BottomLeft, Align::BottomCenter, Align::BottomRight));

        let mut inner_w = (actual_content_w + pad.left + pad.right).max(top_w.max(bot_w));

        if let Some(min_w) = self.min_width {
            let min_inner = min_w.saturating_sub(border_overhead);
            inner_w = inner_w.max(min_inner);
        }

        inner_w = inner_w.min(max_allowed_inner + pad.left + pad.right);

        let (_, _, _, _, _, v) = self.border.as_chars();
        let mut lines = Vec::with_capacity(content_lines.len() + vert_overhead);

        for _ in 0..mar.top {
            lines.push(String::new());
        }

        if has_top_line {
            lines.push(self.get_cached_border(true, inner_w, border_col));
        }

        let mut empty_inside = String::with_capacity(inner_w);
        Self::write_repeat_char(&mut empty_inside, ' ', inner_w);

        for _ in 0..pad.top {
            let mut line_buf = String::with_capacity(inner_w + mar.left + 16);
            Self::write_repeat_char(&mut line_buf, ' ', mar.left);

            if has_border {
                let mut row = String::with_capacity(inner_w + 16);
                let _ = write!(
                    row,
                    "{}{}{}",
                    v.with(border_col),
                    empty_inside,
                    v.with(border_col)
                );
                self.apply_bg_to_buf(&row, &mut line_buf);
            } else {
                self.apply_bg_to_buf(&empty_inside, &mut line_buf);
            }
            lines.push(line_buf);
        }

        let actual_text_area_w = inner_w.saturating_sub(pad.left + pad.right);

        for line in content_lines {
            let mut safe_line: Cow<str> = if line.contains('\t') {
                Cow::Owned(line.replace('\t', "    "))
            } else {
                Cow::Borrowed(&line)
            };

            if self.truncate_lines {
                safe_line = Cow::Owned(ansi::truncate_str(&safe_line, actual_text_area_w));
            };

            let line_w = ansi::visible_width(&safe_line);
            let right_space = actual_text_area_w.saturating_sub(line_w);

            let mut inner_line = String::with_capacity(inner_w + 32);
            Self::write_repeat_char(&mut inner_line, ' ', pad.left);
            inner_line.push_str(&safe_line);
            Self::write_repeat_char(&mut inner_line, ' ', right_space);
            Self::write_repeat_char(&mut inner_line, ' ', pad.right);

            let mut line_buf = String::with_capacity(inner_w + mar.left + 32);
            Self::write_repeat_char(&mut line_buf, ' ', mar.left);

            if has_border {
                let mut row = String::with_capacity(inner_w + 32);
                let _ = write!(
                    row,
                    "{}{}{}",
                    v.with(border_col),
                    inner_line,
                    v.with(border_col)
                );
                self.apply_bg_to_buf(&row, &mut line_buf);
            } else {
                self.apply_bg_to_buf(&inner_line, &mut line_buf);
            }
            lines.push(line_buf);
        }

        for _ in 0..pad.bottom {
            let mut line_buf = String::with_capacity(inner_w + mar.left + 16);
            Self::write_repeat_char(&mut line_buf, ' ', mar.left);

            if has_border {
                let mut row = String::with_capacity(inner_w + 16);
                let _ = write!(
                    row,
                    "{}{}{}",
                    v.with(border_col),
                    empty_inside,
                    v.with(border_col)
                );
                self.apply_bg_to_buf(&row, &mut line_buf);
            } else {
                self.apply_bg_to_buf(&empty_inside, &mut line_buf);
            }
            lines.push(line_buf);
        }

        if has_bot_line {
            lines.push(self.get_cached_border(false, inner_w, border_col));
        }

        for _ in 0..mar.bottom {
            lines.push(String::new());
        }

        lines
    }

    // --- Dynamic Terminal Control ---

    pub(crate) fn prepare_viewport<Writer: Write>(
        writer: &mut Writer,
        prev_height: usize,
        target_height: usize,
    ) -> io::Result<()> {
        if target_height > prev_height {
            if prev_height == 0 {
                if target_height > 1 {
                    let lines_to_add = target_height - 1;
                    for _ in 0..lines_to_add {
                        queue!(writer, crossterm::style::Print("\r\n"))?;
                    }
                    queue!(
                        writer,
                        cursor::MoveUp(lines_to_add as u16),
                        cursor::MoveToColumn(0)
                    )?;
                }
            } else {
                let needed = target_height - prev_height;
                if prev_height > 1 {
                    queue!(writer, cursor::MoveDown((prev_height - 1) as u16))?;
                }
                for _ in 0..needed {
                    queue!(writer, crossterm::style::Print("\r\n"))?;
                }
                queue!(
                    writer,
                    cursor::MoveUp((target_height - 1) as u16),
                    cursor::MoveToColumn(0)
                )?;
            }
            writer.flush()?;
        }
        Ok(())
    }

    pub(crate) fn clear_previous_frame<Writer: Write>(
        writer: &mut Writer,
        prev_height: usize,
    ) -> io::Result<()> {
        if prev_height == 0 {
            return Ok(());
        }

        queue!(writer, cursor::MoveToColumn(0))?;
        for i in 0..prev_height {
            queue!(writer, terminal::Clear(terminal::ClearType::CurrentLine))?;
            if i < prev_height - 1 {
                queue!(writer, cursor::MoveDown(1), cursor::MoveToColumn(0))?;
            }
        }

        if prev_height > 1 {
            queue!(writer, cursor::MoveUp((prev_height - 1) as u16))?;
        }
        queue!(writer, cursor::MoveToColumn(0))?;
        writer.flush()
    }

    pub(crate) fn print_lines_dynamic<Writer: Write>(
        writer: &mut Writer,
        lines: &[String],
    ) -> io::Result<()> {
        if lines.is_empty() {
            return Ok(());
        }

        for (i, line) in lines.iter().enumerate() {
            queue!(
                writer,
                cursor::MoveToColumn(0),
                terminal::Clear(terminal::ClearType::CurrentLine),
                crossterm::style::Print(line)
            )?;

            if i < lines.len() - 1 {
                queue!(writer, cursor::MoveDown(1), cursor::MoveToColumn(0))?;
            }
        }

        if lines.len() > 1 {
            queue!(writer, cursor::MoveUp((lines.len() - 1) as u16))?;
        }
        queue!(writer, cursor::MoveToColumn(0))?;

        writer.flush()
    }

    pub(crate) fn print_lines_final<Writer: Write>(
        writer: &mut Writer,
        lines: &[String],
    ) -> io::Result<()> {
        if lines.is_empty() {
            return Ok(());
        }

        for (i, line) in lines.iter().enumerate() {
            queue!(
                writer,
                cursor::MoveToColumn(0),
                terminal::Clear(terminal::ClearType::CurrentLine),
                crossterm::style::Print(line)
            )?;

            if i < lines.len() - 1 {
                queue!(writer, crossterm::style::Print("\r\n"))?;
            }
        }

        queue!(writer, crossterm::style::Print("\r\n"))?;
        writer.flush()
    }

    // --- Main Rendering Cycle ---

    /// Renders block to the specified Writer (compatible method for `push_sub_widget').
    pub async fn render_to(
        self,
        stdout: &mut io::BufWriter<io::Stdout>,
        _prev_height: usize,
    ) -> io::Result<(W::Output, Vec<String>)> {
        self.render_with_writer(stdout).await
    }

    /// Entry point for launching widget rendering.
    pub async fn render(self) -> io::Result<W::Output> {
        let mut stdout = BufWriter::with_capacity(1024, io::stdout());
        let (output, _) = self.render_with_writer(&mut stdout).await?;
        Ok(output)
    }

    pub async fn render_with_writer(
        mut self,
        mut stdout: &mut io::BufWriter<io::Stdout>,
    ) -> io::Result<(W::Output, Vec<String>)> {
        let _terminal_guard = TerminalGuard::new(true);

        let mut prev_height = 0;
        let mut first_render = true;

        let mut fps_ticker = interval(Duration::from_millis(16));
        let mut needs_redraw = false;

        loop {
            fps_ticker.tick().await;

            // control commands
            while let Ok(cmd) = self.cmd_rx.try_recv() {
                match cmd {
                    Command::Invalidate => needs_redraw = true,
                    Command::Freeze => self.is_frozen = true,
                    Command::Resume => self.is_frozen = false,
                    Command::Finish => self.is_finished_signal = true,
                    Command::PushSubWidget {
                        runner,
                        position,
                        keep_rendered,
                        reply,
                    } => {
                        match position {
                            SubWidgetPosition::Below => {
                                if let Some(ref lines) = self.last_rendered_frame {
                                    let _ = Self::clear_previous_frame(stdout, prev_height);
                                    let _ = Self::print_lines_final(stdout, lines);
                                    prev_height = 0;
                                }
                            }
                            SubWidgetPosition::Above | SubWidgetPosition::Replace => {
                                // clearing parent's frame to free up space
                                let _ = Self::clear_previous_frame(stdout, prev_height);
                                prev_height = 0;
                            }
                        }

                        // executing a subwidget
                        let (res, sub_lines) = runner(stdout, prev_height).await?;
                        let _ = reply.send(res);

                        // erase the subwidget
                        if !keep_rendered || position == SubWidgetPosition::Replace {
                            let _ = Self::clear_previous_frame(stdout, sub_lines.len());
                        }

                        first_render = true;
                    }
                }
            }

            // handling user events for a widget
            while let Ok(ev) = self.event_rx.try_recv() {
                self.inner.handle_event(ev);
            }

            // keyboard and terminal events
            while event::poll(Duration::from_secs(0))? {
                match event::read()? {
                    Event::Key(key) => {
                        if !self.is_frozen {
                            self.inner.handle_key(key);

                            if let Some(ref mut handler) = self.on_key {
                                handler(&mut self.inner, key);
                            }
                        }

                        if key.code == KeyCode::Char('c')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            let _ = Self::clear_previous_frame(&mut stdout, prev_height);
                            let _ = queue!(stdout, cursor::Show);
                            let _ = stdout.flush();
                            let _ = terminal::disable_raw_mode();

                            if let Some(ref mut handler) = self.on_exit {
                                handler();
                            } else {
                                std::process::exit(0);
                            }
                        }
                    }

                    Event::Resize(cols, rows) => {
                        self.invalidate_border_cache();
                        self.last_rendered_frame = None;
                        self.inner.on_resize(cols, rows);

                        let _ = Self::clear_previous_frame(&mut stdout, prev_height);
                        let _ = queue!(
                            stdout,
                            terminal::Clear(terminal::ClearType::All),
                            cursor::MoveTo(0, 0)
                        );
                        let _ = stdout.flush();

                        prev_height = 0;
                    }

                    _ => {}
                }
            }

            let (term_cols, term_rows) = terminal::size().unwrap_or((80, 24));

            // check shutdown signal
            if self.is_finished_signal || (!self.has_handler && self.inner.is_finished()) {
                if !self.is_frozen && (self.inner.is_changed() || needs_redraw) {
                    let _lines = self.render_frame_with_viewport(
                        term_cols as usize,
                        Some(term_rows as usize),
                        true,
                    );
                }

                break;
            }

            // redrawing frame
            if !self.is_frozen && (self.inner.is_changed() || needs_redraw || first_render) {
                first_render = false;

                let lines = self.render_frame_with_viewport(
                    term_cols as usize,
                    Some(term_rows as usize),
                    false,
                );

                self.last_rendered_frame = Some(lines.clone());

                Self::prepare_viewport(&mut stdout, prev_height, lines.len())?;
                Self::clear_previous_frame(&mut stdout, prev_height)?;
                Self::print_lines_dynamic(&mut stdout, &lines)?;
                prev_height = lines.len();

                // positioning cursor
                if self.show_cursor {
                    if let Some((col, row)) = self.calculate_cursor_offset() {
                        queue!(stdout, cursor::MoveToColumn(col))?;

                        if row > 0 {
                            queue!(stdout, cursor::MoveDown(row))?;
                        }
                        queue!(stdout, cursor::Show)?;
                    } else {
                        queue!(stdout, cursor::Hide)?;
                    }
                } else {
                    queue!(stdout, cursor::Hide)?;
                }
                stdout.flush()?;

                if self.show_cursor {
                    if let Some((_, row)) = self.calculate_cursor_offset() {
                        if row > 0 {
                            queue!(stdout, cursor::MoveUp(row))?;
                        }
                    }
                }
            }
        }

        // final render
        let final_lines = if !self.remove_on_finish {
            let (term_cols, term_rows) = terminal::size().unwrap_or((80, 24));

            // run blink animation
            if let Some(blink_col) = self.blink_color {
                let original_bg = self.bg_color.unwrap_or(Color::Rgb { r: 0, g: 0, b: 0 });
                let mid_color = ansi::lerp_color(original_bg, blink_col, 0.5);

                let total_ms = self.blink_duration.as_millis() as f32;
                let p1 = Duration::from_millis((total_ms * 0.25) as u64);
                let p2 = Duration::from_millis((total_ms * 0.45) as u64);
                let p3 = Duration::from_millis((total_ms * 0.30) as u64);

                let mut render_blink_phase = |this: &mut Self, color: Color| {
                    this.bg_color = Some(color);
                    this.invalidate_border_cache();

                    let lines = this.render_frame_with_viewport(
                        term_cols as usize,
                        Some(term_rows as usize),
                        false,
                    );

                    let _ = Self::clear_previous_frame(&mut stdout, prev_height);
                    let _ = Self::print_lines_dynamic(&mut stdout, &lines);
                    prev_height = lines.len();
                };

                render_blink_phase(&mut self, mid_color);
                tokio::time::sleep(p1).await;

                render_blink_phase(&mut self, blink_col);
                tokio::time::sleep(p2).await;

                render_blink_phase(&mut self, mid_color);
                tokio::time::sleep(p3).await;

                self.bg_color = Some(original_bg);
                self.invalidate_border_cache();
            }

            Self::clear_previous_frame(stdout, prev_height)?;
            self.max_height = None;

            let lines = self.render_frame_with_viewport(term_cols as usize, None, true);
            Self::print_lines_final(stdout, &lines)?;
            lines
        } else {
            Self::clear_previous_frame(stdout, prev_height)?;
            Vec::new()
        };

        Ok((self.inner.extract_output(), final_lines))
    }
}
