use serde::{Deserialize, Serialize};

/// Cargo package metadata
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PkgMeta {
    pub name: &'static str,
    pub description: &'static str,
    pub version: &'static str,
}

#[macro_export]
macro_rules! pkg_meta {
    () => {
        $crate::parser::PkgMeta {
            name: env!("CARGO_PKG_NAME"),
            description: env!("CARGO_PKG_DESCRIPTION"),
            version: env!("CARGO_PKG_VERSION"),
        }
    };
}
