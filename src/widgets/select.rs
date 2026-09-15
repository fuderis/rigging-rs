use crate::render::{block::Block, widget::Widget};

use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    style::Stylize,
};
use std::sync::Arc;

pub struct SelectMenu {
    prompt: String,
    items: Vec<String>,
    selected_idx: usize,
    output: Option<usize>,
    is_finished: bool,
    dirty: bool,
}

impl SelectMenu {
    pub fn new(prompt: impl Into<String>, items: Vec<impl Into<String>>) -> Block<Self> {
        let items: Vec<String> = items.into_iter().map(Into::into).collect();
        Block::new(
            Self {
                prompt: prompt.into(),
                items,
                selected_idx: 0,
                output: None,
                is_finished: false,
                dirty: true,
            },
            (),
        )
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
            let num_prefix = if idx < 9 {
                format!("{}. ", idx + 1)
            } else {
                "   ".to_string()
            };

            if idx == self.selected_idx {
                let line = format!("  > {}{}", num_prefix, item)
                    .cyan()
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
                if self.selected_idx > 0 {
                    self.selected_idx -= 1;
                } else {
                    self.selected_idx = self.items.len() - 1;
                }
                moved = true;
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_idx + 1 < self.items.len() {
                    self.selected_idx += 1;
                } else {
                    self.selected_idx = 0;
                }
                moved = true;
            }
            KeyCode::Char(ch @ '1'..='9') => {
                let digit_idx = (ch as usize) - ('1' as usize);
                if digit_idx < self.items.len() && self.selected_idx != digit_idx {
                    self.selected_idx = digit_idx;
                    moved = true;
                }
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
