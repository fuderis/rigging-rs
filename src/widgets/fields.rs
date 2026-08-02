use crate::render::{
    SeparatorStyle,
    block::Block,
    widget::{DynamicWidget, Widget},
};
use crossterm::style::{Color, Stylize};
use std::{
    fmt::Display,
    future::Future,
    sync::{Arc, Mutex},
};
use tokio::sync::{Notify, mpsc};

#[derive(Clone)]
pub struct FieldItem {
    pub label: String,
    pub value: String,
}

pub enum FieldsOp {
    SetField(String, String),
    RemoveField(String),
    Clear,
    SetAll(Vec<(String, String)>),
}

pub struct FieldsHandle {
    sender: mpsc::UnboundedSender<FieldsOp>,
}

impl FieldsHandle {
    pub fn push(&self, label: impl Display, value: impl Display) {
        let _ = self
            .sender
            .send(FieldsOp::SetField(label.to_string(), value.to_string()));
    }

    pub fn remove(&self, label: impl Display) {
        let _ = self.sender.send(FieldsOp::RemoveField(label.to_string()));
    }

    pub fn clear(&self) {
        let _ = self.sender.send(FieldsOp::Clear);
    }

    pub fn set<I, K, V>(&self, pairs: I)
    where
        I: IntoIterator<Item = (K, V)>,
        K: Display,
        V: Display,
    {
        let items_vec = pairs
            .into_iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();

        let _ = self.sender.send(FieldsOp::SetAll(items_vec));
    }
}

pub struct Fields {
    pub(crate) items: Arc<Mutex<Vec<FieldItem>>>,
    pub(crate) separator: SeparatorStyle,
    pub(crate) separator_color: Option<Color>,
    pub(crate) done_signal: Arc<Notify>,
}

impl Fields {
    pub fn new() -> Block<Self> {
        let done_signal = Arc::new(Notify::new());
        done_signal.notify_one();

        Block::new(Self {
            items: Arc::new(Mutex::new(Vec::new())),
            separator: SeparatorStyle::Colon,
            separator_color: None,
            done_signal,
        })
    }
}

impl Block<Fields> {
    pub fn handler<F, Fut>(self, task: F) -> Self
    where
        F: FnOnce(FieldsHandle) -> Fut + Send + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let done_signal = Arc::new(Notify::new());
        let done_notifier = Arc::clone(&done_signal);

        let (tx, mut rx) = mpsc::unbounded_channel::<FieldsOp>();

        tokio::spawn(async move {
            let handle = FieldsHandle { sender: tx };
            task(handle).await;
        });

        let items_writer = Arc::clone(&self.inner.items);
        tokio::spawn(async move {
            while let Some(op) = rx.recv().await {
                if let Ok(mut lock) = items_writer.lock() {
                    match op {
                        FieldsOp::SetField(label, value) => {
                            if let Some(existing) = lock.iter_mut().find(|f| f.label == label) {
                                existing.value = value;
                            } else {
                                lock.push(FieldItem { label, value });
                            }
                        }
                        FieldsOp::RemoveField(label) => {
                            lock.retain(|f| f.label != label);
                        }
                        FieldsOp::Clear => lock.clear(),
                        FieldsOp::SetAll(pairs) => {
                            *lock = pairs
                                .into_iter()
                                .map(|(l, v)| FieldItem { label: l, value: v })
                                .collect();
                        }
                    }
                }
            }
            done_notifier.notify_one();
        });

        let mut block = self;
        block.inner.done_signal = done_signal;
        block
    }

    pub fn separator(mut self, sep: SeparatorStyle, color: Option<Color>) -> Self {
        self.inner.separator = sep;
        self.inner.separator_color = color;
        self
    }

    pub fn field(self, label: impl Display, value: impl Display) -> Self {
        if let Ok(mut lock) = self.inner.items.lock() {
            lock.push(FieldItem {
                label: label.to_string(),
                value: value.to_string(),
            });
        }
        self
    }

    pub fn fields<I, K, V>(self, pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Display,
        V: Display,
    {
        if let Ok(mut lock) = self.inner.items.lock() {
            for (k, v) in pairs {
                lock.push(FieldItem {
                    label: k.to_string(),
                    value: v.to_string(),
                });
            }
        }
        self
    }
}

impl Widget for Fields {
    type Output = ();

    fn render_content(&self, width: usize) -> Vec<String> {
        let items = self.items.lock().unwrap();
        if items.is_empty() {
            return vec![];
        }

        let max_label_width = items
            .iter()
            .map(|item| strip_ansi_len(&item.label))
            .max()
            .unwrap_or(0);

        let sep_raw = self.separator.as_str();
        let sep_str = self.inner_separator_formatted();
        let sep_len = sep_raw.chars().count();

        items
            .iter()
            .map(|item| {
                let plain_label_len = strip_ansi_len(&item.label);
                let padding_len = max_label_width.saturating_sub(plain_label_len);
                let padding = " ".repeat(padding_len);

                // Суммарная ширина левой части: label + padding + " " + separator + " "
                let prefix_width = max_label_width + 1 + sep_len + 1;
                let available_value_width = width.saturating_sub(prefix_width);

                let val = if strip_ansi_len(&item.value) > available_value_width
                    && available_value_width > 3
                {
                    format!(
                        "{}...",
                        &item.value[..available_value_width.saturating_sub(3)]
                    )
                } else {
                    item.value.clone()
                };

                format!("{}{} {} {}", item.label, padding, sep_str, val)
            })
            .collect()
    }
}

impl DynamicWidget for Fields {
    fn completion_signal(&self) -> Arc<Notify> {
        Arc::clone(&self.done_signal)
    }

    fn extract_output(self) -> Self::Output {
        ()
    }
}

impl Fields {
    fn inner_separator_formatted(&self) -> String {
        let raw = self.separator.as_str();
        if let Some(color) = self.separator_color {
            raw.with(color).to_string()
        } else {
            raw.to_string()
        }
    }
}

/// Вычисляет длины строк без учета ANSI ESC-последовательностей
fn strip_ansi_len(input: &str) -> usize {
    let mut len = 0;
    let mut in_sequence = false;

    for c in input.chars() {
        if c == '\x1b' {
            in_sequence = true;
        } else if in_sequence {
            if c == 'm' || c == 'K' {
                in_sequence = false;
            }
        } else {
            len += 1;
        }
    }

    len
}
