/// Defines whether a flag is mandatory or optional with a default fallback value.
#[derive(Debug, Clone)]
pub enum FlagKind {
    /// Flag must be provided by the user.
    Required,
    /// Flag is optional and defaults to the contained value if omitted.
    Optional(String),
}

/// Flag specification containing canonical/alias identifiers and validation requirements.
#[derive(Debug, Clone)]
pub struct FlagSpec {
    /// All defined names for the flag (e.g., `["l", "load", "pull"]`).
    /// The first long name (or first entry) acts as the canonical key in `Context.args`.
    pub names: Vec<String>,
    /// Validation and fallback policy.
    pub kind: FlagKind,
}

impl FlagSpec {
    /// Returns the canonical name used for key lookup in `Context.args`.
    pub fn canonical_name(&self) -> &str {
        self.names
            .iter()
            .find(|n| n.len() > 1)
            .map(|s| s.as_str())
            .unwrap_or_else(|| &self.names[0])
    }

    /// Returns explicitly declared single-character short alias, if present.
    pub fn explicit_short(&self) -> Option<char> {
        self.names
            .iter()
            .find(|n| n.len() == 1)
            .and_then(|s| s.chars().next())
    }
}

/// Token specifications used for parsing routing patterns.
#[derive(Debug, Clone)]
pub enum TokenSpec {
    /// Literal matching string (e.g., subcommand names).
    Literal(String),
    /// Positional argument placeholder.
    Positional(String),
    /// Catch-all positional argument placeholder.
    Rest(String),
}

/// Parsed representation of a raw route pattern.
#[derive(Debug, Clone)]
pub struct RouteSpec {
    /// Original raw pattern text.
    pub pattern_raw: &'static str,
    /// Documentation text assigned to the command/route.
    pub doc: &'static str,
    /// Ordered tokens extracted from the pattern.
    pub tokens: Vec<TokenSpec>,
    /// Flags extracted from the pattern.
    pub flags: Vec<FlagSpec>,
}
