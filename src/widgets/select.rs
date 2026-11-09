use crate::render::{block::Block, widget::Widget};

use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    style::{Color, Stylize},
};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[cfg(feature = "highlight")]
use crate::theme::CodeTheme;
#[cfg(feature = "markdown")]
use crate::{
    render::markdown::Markdown,
    style::{BulletStyle, StripeStyle},
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

    /// Configuration for Markdown parsing and rendering.
    #[cfg(feature = "markdown")]
    pub(crate) markdown: Option<Markdown>,
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

                #[cfg(feature = "markdown")]
                markdown: Some(Markdown::default()),
            },
            (),
        )
    }

    fn handle_digit_input(&mut self, digit: char) -> bool {
        let now = Instant::now();

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

    #[cfg(feature = "markdown")]
    pub fn markdown(mut self, enable: bool) -> Self {
        if enable && self.inner.markdown.is_none() {
            self.inner.markdown = Some(Markdown::default());
        } else if !enable {
            self.inner.markdown = None;
        }
        self
    }

    #[cfg(feature = "markdown")]
    pub fn accent_color(mut self, color: Color) -> Self {
        if let Some(md) = self.inner.markdown.take() {
            self.inner.markdown =
                Some(md.stripe_color(color).bullet_color(color).code_color(color));
        }
        self.inner.selected_color = color;
        self
    }

    #[cfg(feature = "markdown")]
    pub fn stripe_style(mut self, style: StripeStyle) -> Self {
        let md = self.inner.markdown.take().unwrap_or_default();
        self.inner.markdown = Some(md.stripe_style(style));
        self
    }

    #[cfg(feature = "markdown")]
    pub fn stripe_color(mut self, color: Color) -> Self {
        let md = self.inner.markdown.take().unwrap_or_default();
        self.inner.markdown = Some(md.stripe_color(color));
        self
    }

    #[cfg(feature = "markdown")]
    pub fn bullet_style(mut self, style: BulletStyle) -> Self {
        let md = self.inner.markdown.take().unwrap_or_default();
        self.inner.markdown = Some(md.bullet_style(style));
        self
    }

    #[cfg(feature = "markdown")]
    pub fn bullet_color(mut self, color: Color) -> Self {
        let md = self.inner.markdown.take().unwrap_or_default();
        self.inner.markdown = Some(md.bullet_color(color));
        self
    }

    #[cfg(feature = "markdown")]
    pub fn code_color(mut self, color: Color) -> Self {
        let md = self.inner.markdown.take().unwrap_or_default();
        self.inner.markdown = Some(md.code_color(color));
        self
    }

    #[cfg(all(feature = "markdown", feature = "highlight"))]
    pub fn code_theme(mut self, theme: CodeTheme) -> Self {
        let md = self.inner.markdown.take().unwrap_or_default();
        self.inner.markdown = Some(md.theme(theme));
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
        max_width: Option<usize>,
        _max_height: Option<usize>,
        _is_final: bool,
    ) -> Vec<String> {
        self.dirty = false;

        let mut lines = Vec::new();

        // Рендеринг промпта с поддержкой Markdown
        let prompt_lines: Vec<String> = match max_width {
            Some(w) if w > 0 => {
                #[cfg(feature = "markdown")]
                {
                    if let Some(md) = &self.markdown {
                        let rendered = md.render(&self.prompt, w);
                        let trimmed = rendered.trim_end_matches(|c| c == '\r' || c == '\n');
                        if trimmed.is_empty() {
                            Vec::new()
                        } else {
                            trimmed.lines().map(|s| s.to_string()).collect()
                        }
                    } else {
                        crate::utils::ansi::wrap_terminal_text(&self.prompt, w)
                    }
                }
                #[cfg(not(feature = "markdown"))]
                {
                    crate::utils::ansi::wrap_terminal_text(&self.prompt, w)
                }
            }
            _ => self.prompt.lines().map(|s| s.to_string()).collect(),
        };

        lines.extend(prompt_lines);

        // Рендеринг элементов меню с trim_start для элемента (без trim_end) и без префикса выбора
        for (idx, item) in self.items.iter().enumerate() {
            let num_prefix = format!("{}. ", idx + 1);
            #[cfg(feature = "markdown")]
            let item_max_width = max_width.map(|w| w.saturating_sub(num_prefix.len()));

            #[cfg(feature = "markdown")]
            let rendered_item_lines = if let Some(md) = &self.markdown {
                let available_w = item_max_width.unwrap_or(80);
                let rendered = md.render(item, available_w);
                let trimmed = rendered.trim_start_matches(|c| c == '\r' || c == '\n');
                if trimmed.is_empty() {
                    vec![item.clone()]
                } else {
                    trimmed.lines().map(|s| s.to_string()).collect()
                }
            } else {
                item.lines().map(|s| s.to_string()).collect()
            };

            #[cfg(not(feature = "markdown"))]
            let rendered_item_lines = item.lines().map(|s| s.to_string()).collect::<Vec<String>>();

            for (line_idx, item_line) in rendered_item_lines.into_iter().enumerate() {
                let is_selected = idx == self.selected_idx;

                let line_str = if line_idx == 0 {
                    format!("{}{}{}", num_prefix, item_line, "\x1b[0m")
                } else {
                    format!("{}{}", item_line, "\x1b[0m")
                };

                if is_selected {
                    lines.push(line_str.with(self.selected_color).bold().to_string());
                } else {
                    lines.push(line_str);
                }
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
