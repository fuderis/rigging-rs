use crate::render::{
    BulletStyle, ansi,
    block::Block,
    widget::{DynamicWidget, InteractiveWidget, Widget},
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    style::{Color, Stylize},
};
use std::{
    fmt::Display,
    future::Future,
    io,
    pin::Pin,
    sync::{Arc, Mutex},
};
use tokio::sync::{Notify, mpsc};

pub enum ListOp {
    Add(String),
    Clear,
    Set(Vec<String>),
}

pub struct ListHandle {
    sender: mpsc::UnboundedSender<ListOp>,
}

impl ListHandle {
    pub fn push(&self, item: impl Display) {
        let _ = self.sender.send(ListOp::Add(item.to_string()));
    }

    pub fn clear(&self) {
        let _ = self.sender.send(ListOp::Clear);
    }

    pub fn set<I, T>(&self, items: I)
    where
        I: IntoIterator<Item = T>,
        T: Display,
    {
        let items_vec = items.into_iter().map(|i| i.to_string()).collect();
        let _ = self.sender.send(ListOp::Set(items_vec));
    }
}

pub type ListTask =
    Box<dyn FnOnce(ListHandle) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

pub struct List {
    pub(crate) items: Arc<Mutex<Vec<String>>>,
    pub(crate) bullet: BulletStyle,
    pub(crate) bullet_color: Option<Color>,

    pub(crate) active_bullet: BulletStyle,
    pub(crate) active_color: Option<Color>,
    pub(crate) selected_index: Arc<Mutex<Option<usize>>>,

    pub(crate) handler: Option<ListTask>,
    pub(crate) done_signal: Arc<Notify>,
}

impl List {
    pub fn new() -> Block<Self> {
        Block::new(Self {
            items: Arc::new(Mutex::new(Vec::new())),
            bullet: BulletStyle::Dot,
            bullet_color: None,
            active_bullet: BulletStyle::Triangle,
            active_color: Some(Color::Cyan),
            selected_index: Arc::new(Mutex::new(Some(0))),
            handler: None,
            done_signal: Arc::new(Notify::new()),
        })
    }

    pub fn handle_event(&mut self, event: &Event) -> bool {
        if let Event::Key(key) = event {
            if key.kind == KeyEventKind::Press {
                return self.handle_navigation(*key);
            }
        }
        false
    }

    pub fn handle_navigation(&self, key: KeyEvent) -> bool {
        let KeyEvent {
            code, modifiers, ..
        } = key;
        let is_shift = modifiers.contains(KeyModifiers::SHIFT);
        let is_ctrl = modifiers.contains(KeyModifiers::CONTROL);

        let items_len = self.items.lock().map(|l| l.len()).unwrap_or(0);
        if items_len == 0 {
            return false;
        }

        let mut selected_guard = match self.selected_index.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };

        let current = selected_guard.unwrap_or(0);

        match code {
            KeyCode::Enter => return true,
            KeyCode::Char('j') if is_ctrl => return true,

            KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => {
                let next = if current == 0 {
                    items_len - 1
                } else {
                    current - 1
                };
                *selected_guard = Some(next);
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let next = (current + 1) % items_len;
                *selected_guard = Some(next);
            }
            KeyCode::Tab if is_shift => {
                let next = if current == 0 {
                    items_len - 1
                } else {
                    current - 1
                };
                *selected_guard = Some(next);
            }
            KeyCode::Tab => {
                let next = (current + 1) % items_len;
                *selected_guard = Some(next);
            }
            KeyCode::Home => *selected_guard = Some(0),
            KeyCode::End => *selected_guard = Some(items_len - 1),
            _ => {}
        }

        false
    }
}

impl Block<List> {
    pub fn handler<F, Fut>(mut self, task: F) -> Self
    where
        F: FnOnce(ListHandle) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.inner.handler = Some(Box::new(move |handle| Box::pin(task(handle))));
        self
    }

    pub fn bullet(mut self, bullet: BulletStyle, color: Option<Color>) -> Self {
        self.inner.bullet = bullet;
        self.inner.bullet_color = color;
        self
    }

    pub fn active_bullet(mut self, bullet: BulletStyle, color: Option<Color>) -> Self {
        self.inner.active_bullet = bullet;
        self.inner.active_color = color;
        self
    }

    pub fn item(self, item: impl Display) -> Self {
        if let Ok(mut lock) = self.inner.items.lock() {
            lock.push(item.to_string());
        }
        self
    }

    pub fn items<I, T>(self, items: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Display,
    {
        if let Ok(mut lock) = self.inner.items.lock() {
            for item in items {
                lock.push(item.to_string());
            }
        }
        self
    }

    pub async fn render(mut self) -> io::Result<(usize, String)> {
        if let Some(handler) = self.inner.handler.take() {
            let (tx, mut rx) = mpsc::unbounded_channel::<ListOp>();
            let items_writer = Arc::clone(&self.inner.items);
            let done_notify = Arc::clone(&self.inner.done_signal);

            tokio::spawn(async move {
                let handle = ListHandle { sender: tx };
                handler(handle).await;
            });

            tokio::spawn(async move {
                while let Some(op) = rx.recv().await {
                    if let Ok(mut lock) = items_writer.lock() {
                        match op {
                            ListOp::Add(item) => lock.push(item),
                            ListOp::Clear => lock.clear(),
                            ListOp::Set(new_items) => *lock = new_items,
                        }
                    }
                }
                done_notify.notify_one();
            });
        }

        InteractiveWidget::render(self).await
    }
}

impl Widget for List {
    type Output = (usize, String);

    fn render_content(&self, max_width: usize) -> Vec<String> {
        if max_width == 0 {
            return Vec::new();
        }

        let mut lines = Vec::new();
        let items = self.items.lock().unwrap();
        let selected_opt = *self.selected_index.lock().unwrap();

        for (idx, item) in items.iter().enumerate() {
            let is_selected = selected_opt == Some(idx);

            let (bullet, color) = if is_selected {
                (self.active_bullet, self.active_color)
            } else {
                (self.bullet, self.bullet_color)
            };

            let symbol_raw = bullet.render_symbol(idx);
            let symbol_len = ansi::visible_width(&symbol_raw);

            let symbol = if let Some(c) = color {
                symbol_raw.with(c).to_string()
            } else {
                symbol_raw
            };

            let indent_len = symbol_len + 1;
            let available_text_width = max_width.saturating_sub(indent_len);

            if available_text_width == 0 {
                let formatted_symbol = if is_selected {
                    symbol.bold().to_string()
                } else {
                    symbol
                };
                lines.push(formatted_symbol);
                continue;
            }

            let wrapped_item_lines = wrap_terminal_text(item, available_text_width);
            let indent = " ".repeat(indent_len);

            for (line_idx, line) in wrapped_item_lines.into_iter().enumerate() {
                let formatted_line = if is_selected {
                    line.bold().to_string()
                } else {
                    line
                };

                if line_idx == 0 {
                    lines.push(format!("{} {}\x1b[0m", symbol, formatted_line));
                } else {
                    lines.push(format!("{}{}\x1b[0m", indent, formatted_line));
                }
            }
        }

        lines
    }
}

impl DynamicWidget for List {
    fn completion_signal(&self) -> Arc<Notify> {
        Arc::clone(&self.done_signal)
    }

    fn extract_output(self) -> Self::Output {
        let items = self.items.lock().unwrap();
        let selected_idx = self.selected_index.lock().unwrap().unwrap_or(0);

        if selected_idx < items.len() {
            (selected_idx, items[selected_idx].clone())
        } else {
            (0, String::new())
        }
    }
}

pub(crate) fn wrap_terminal_text(text: &str, max_width: usize) -> Vec<String> {
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

fn wrap_paragraph(paragraph: &str, max_width: usize) -> Vec<String> {
    if paragraph.is_empty() {
        return vec![String::new()];
    }

    let mut out = Vec::new();
    let mut current = String::new();

    for token in paragraph.split(' ') {
        let token_w = ansi::visible_width(token);

        if current.is_empty() {
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
            let current_w = ansi::visible_width(&current);
            let needed = 1 + token_w;

            if current_w + needed <= max_width {
                current.push(' ');
                current.push_str(token);
            } else {
                out.push(current);
                current = String::new();

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

fn split_token_by_width(token: &str, max_width: usize) -> Vec<String> {
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
        let ch_w = ansi::visible_width(&ch_s);

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
