#![doc = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md"))]
pub mod render;
pub mod widgets;

pub mod style;
pub use style::*;

#[cfg(feature = "highlight")]
pub mod theme;

pub use clap::{self, Args, Command, Parser, Subcommand};
pub use crossterm::{
    self,
    style::{Color, Stylize},
};
