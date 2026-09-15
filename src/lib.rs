#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
mod error;
mod prelude;

pub mod utils;

pub mod parser;
pub use parser::{Commands, Context as CommandContext, PkgMeta};

pub mod render;
pub use render::Context as WidgetContext;

pub mod widgets;

pub mod style;
pub use style::*;

#[cfg(feature = "highlight")]
pub mod theme;

pub use crossterm::{
    self,
    style::{Color, Stylize},
};
