use crate::render::{block::Block, widget::Widget};
use serde::{Deserialize, Serialize};

#[cfg(feature = "markdown")]
use crossterm::style::Color;
use crossterm::{
    event::{KeyCode, KeyEvent, KeyModifiers},
    style::Stylize,
};
use std::sync::Arc;

#[cfg(feature = "highlight")]
use crate::theme::CodeTheme;
#[cfg(feature = "markdown")]
use crate::{
    render::markdown::Markdown,
    style::{BulletStyle, StripeStyle},
};

/// Confirmatoin status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Confirmation {
    Yes,
    No,
}

/// Confirmation prompt widget.
pub struct ConfirmPrompt {
    prompt: String,
    default: Option<Confirmation>,
    output: Option<Confirmation>,
    is_finished: bool,
    dirty: bool,

    #[cfg(feature = "markdown")]
    pub(crate) markdown: Option<Markdown>,
}

impl ConfirmPrompt {
    pub fn new(prompt: impl Into<String>) -> Block<Self> {
        Block::new(
            Self {
                prompt: prompt.into(),
                default: None,
                output: None,
                is_finished: false,
                dirty: true,

                #[cfg(feature = "markdown")]
                markdown: Some(Markdown::default()),
            },
            (),
        )
    }
}

impl Block<ConfirmPrompt> {
    pub fn default(mut self, default: Confirmation) -> Self {
        self.inner.default = Some(default);
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
        if let Some(md) = self.inner.markdown {
            self.inner.markdown =
                Some(md.stripe_color(color).bullet_color(color).code_color(color));
        }
        self
    }

    #[cfg(feature = "markdown")]
    pub fn stripe_style(mut self, style: StripeStyle) -> Self {
        let md = self.inner.markdown.unwrap_or_default();
        self.inner.markdown = Some(md.stripe_style(style));
        self
    }

    #[cfg(feature = "markdown")]
    pub fn stripe_color(mut self, color: Color) -> Self {
        let md = self.inner.markdown.unwrap_or_default();
        self.inner.markdown = Some(md.stripe_color(color));
        self
    }

    #[cfg(feature = "markdown")]
    pub fn bullet_style(mut self, style: BulletStyle) -> Self {
        let md = self.inner.markdown.unwrap_or_default();
        self.inner.markdown = Some(md.bullet_style(style));
        self
    }

    #[cfg(feature = "markdown")]
    pub fn bullet_color(mut self, color: Color) -> Self {
        let md = self.inner.markdown.unwrap_or_default();
        self.inner.markdown = Some(md.bullet_color(color));
        self
    }

    #[cfg(feature = "markdown")]
    pub fn code_color(mut self, color: Color) -> Self {
        let md = self.inner.markdown.unwrap_or_default();
        self.inner.markdown = Some(md.code_color(color));
        self
    }

    #[cfg(all(feature = "markdown", feature = "highlight"))]
    pub fn code_theme(mut self, theme: CodeTheme) -> Self {
        let md = self.inner.markdown.unwrap_or_default();
        self.inner.markdown = Some(md.theme(theme));
        self
    }
}

impl Widget for ConfirmPrompt {
    type State = ();
    type Output = Option<Confirmation>;
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

        let hint_str = match self.default {
            Some(Confirmation::Yes) => "[Y/n]",
            Some(Confirmation::No) => "[y/N]",
            None => "[y/n]",
        };

        let hint = hint_str.dim().to_string();
        #[cfg(feature = "markdown")]
        let hint_len = hint_str.len() + 1; // +1 for a space before hint

        // preparing prompt with markdown parsing
        let rendered_prompt = match max_width {
            Some(_w) => {
                #[cfg(feature = "markdown")]
                {
                    let available_width = _w.saturating_sub(hint_len);
                    if let Some(md) = &self.markdown {
                        md.render(&self.prompt, available_width)
                            .trim_end_matches(|c| c == '\r' || c == '\n')
                            .to_string()
                    } else {
                        self.prompt.clone()
                    }
                }
                #[cfg(not(feature = "markdown"))]
                {
                    self.prompt.clone()
                }
            }
            None => {
                #[cfg(feature = "markdown")]
                {
                    if let Some(md) = &self.markdown {
                        md.render(&self.prompt, usize::MAX)
                            .trim_end_matches(|c| c == '\r' || c == '\n')
                            .to_string()
                    } else {
                        self.prompt.clone()
                    }
                }
                #[cfg(not(feature = "markdown"))]
                {
                    self.prompt.clone()
                }
            }
        };

        let mut lines: Vec<String> = rendered_prompt.lines().map(|s| s.to_string()).collect();

        if let Some(last_line) = lines.last_mut() {
            last_line.push(' ');
            last_line.push_str(&hint);
        } else {
            lines.push(hint);
        }

        lines
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if self.is_finished {
            return;
        }

        let is_ctrl_j =
            key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('j');

        let mut handled = true;

        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.output = Some(Confirmation::Yes);
                self.is_finished = true;
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                self.output = Some(Confirmation::No);
                self.is_finished = true;
            }
            KeyCode::Enter if self.default.is_some() => {
                self.output = self.default;
                self.is_finished = true;
            }
            _ if is_ctrl_j && self.default.is_some() => {
                self.output = self.default;
                self.is_finished = true;
            }
            KeyCode::Esc => {
                self.output = None;
                self.is_finished = true;
            }
            _ => handled = false,
        }

        if handled {
            self.dirty = true;
        }
    }

    fn extract_output(&mut self) -> Self::Output {
        self.output.take()
    }
}
