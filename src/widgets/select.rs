use crate::render::{block::Block, widget::Widget};

use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    style::{Color, Stylize},
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

const DIGIT_TIMEOUT: Duration = Duration::from_millis(500);

/// Interactive selection menu.
pub struct SelectMenu {
    prompt: String,
    items: Vec<String>,
    selected_idx: usize,
    selected_color: Color,
    output: Option<usize>,
    is_finished: bool,
    dirty: bool,
    digit_buffer: String,
    last_digit_time: Option<Instant>,
}

impl SelectMenu {
    pub fn new(prompt: impl Into<String>, items: Vec<impl Into<String>>) -> Block<Self> {
        let items: Vec<String> = items.into_iter().map(Into::into).collect();
        Block::new(
            Self {
                prompt: prompt.into(),
                items,
                selected_idx: 0,
                selected_color: Color::Cyan,
                output: None,
                is_finished: false,
                dirty: true,
                digit_buffer: String::new(),
                last_digit_time: None,
            },
            (),
        )
    }

    /// Вспомогательный метод для обработки ввода цифр
    fn handle_digit_input(&mut self, digit: char) -> bool {
        let now = Instant::now();

        // Если прошло больше 500 мс — сбрасываем накопленный буфер
        if let Some(last_time) = self.last_digit_time {
            if now.duration_since(last_time) > DIGIT_TIMEOUT {
                self.digit_buffer.clear();
            }
        } else {
            self.digit_buffer.clear();
        }

        self.digit_buffer.push(digit);
        self.last_digit_time = Some(now);

        if let Ok(num) = self.digit_buffer.parse::<usize>() {
            if num > 0 {
                let target_idx = num - 1;
                if target_idx < self.items.len() && self.selected_idx != target_idx {
                    self.selected_idx = target_idx;
                    return true;
                }
            }
        }

        false
    }
}

impl Block<SelectMenu> {
    pub fn select_color(mut self, color: Color) -> Self {
        self.inner.selected_color = color;
        self
    }
}

impl Widget for SelectMenu {
    type State = ();
    type Output = Option<usize>;
    type Event = ();

    fn is_changed(&self) -> bool {
        self.dirty
    }

    fn is_finished(&self) -> bool {
        self.is_finished
    }

    fn render_frame(
        &mut self,
        _state: &Arc<Self::State>,
        _max_width: Option<usize>,
        _max_height: Option<usize>,
        _is_final: bool,
    ) -> Vec<String> {
        self.dirty = false;

        let mut lines = Vec::with_capacity(self.items.len() + 1);
        lines.push(self.prompt.clone());

        for (idx, item) in self.items.iter().enumerate() {
            let num_prefix = format!("{}. ", idx + 1);

            if idx == self.selected_idx {
                let line = format!("  > {}{}", num_prefix, item)
                    .with(self.selected_color)
                    .bold()
                    .to_string();
                lines.push(line);
            } else {
                let line = format!("    {}{}", num_prefix, item);
                lines.push(line);
            }
        }

        lines
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.is_finished || self.items.is_empty() {
            return;
        }

        let mut moved = false;

        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('j') {
            self.output = Some(self.selected_idx);
            self.is_finished = true;
            self.dirty = true;
            return;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.digit_buffer.clear();
                if self.selected_idx > 0 {
                    self.selected_idx -= 1;
                } else {
                    self.selected_idx = self.items.len() - 1;
                }
                moved = true;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.digit_buffer.clear();
                if self.selected_idx + 1 < self.items.len() {
                    self.selected_idx += 1;
                } else {
                    self.selected_idx = 0;
                }
                moved = true;
            }
            KeyCode::Char(ch @ '0'..='9') => {
                moved = self.handle_digit_input(ch);
            }
            KeyCode::Enter => {
                self.output = Some(self.selected_idx);
                self.is_finished = true;
                moved = true;
            }
            KeyCode::Esc => {
                self.output = None;
                self.is_finished = true;
                moved = true;
            }
            _ => {}
        }

        if moved {
            self.dirty = true;
        }
    }

    fn extract_output(&mut self) -> Self::Output {
        self.output.take()
    }
}
