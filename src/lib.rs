#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
mod error;
mod prelude;

pub mod utils;

#[cfg(feature = "parser")]
pub mod parser;
#[cfg(feature = "parser")]
pub use parser::{Commands, Context as CommandContext, PkgMeta};

#[cfg(feature = "render")]
pub mod render;
#[cfg(feature = "render")]
pub use render::Context as WidgetContext;

#[cfg(feature = "render")]
pub mod style;
#[cfg(feature = "render")]
pub use style::*;

#[cfg(feature = "widgets")]
pub mod widgets;

#[cfg(feature = "highlight")]
pub mod theme;

pub use crossterm::{
    self,
    style::{Color, Stylize},
};
