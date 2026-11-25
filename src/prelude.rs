#![allow(unused_imports, dead_code)]
pub use crate::{
    error::Error,
    state::{State, StateGuard},
};

pub type DynError = Box<dyn std::error::Error + Send + Sync + 'static>;

pub use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, RwLock},
};

pub use macron::matches_downcast;
