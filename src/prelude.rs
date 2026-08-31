#![allow(unused_imports)]
pub use crate::error::*;

pub use std::result::Result as StdResult;
pub type DynError = Box<dyn std::error::Error + Send + Sync + 'static>;
pub type Result<T> = StdResult<T, DynError>;

pub use std::collections::{HashMap, HashSet};
