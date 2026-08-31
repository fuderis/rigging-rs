use super::{FlagSpec, HandlerFn, TokenSpec};

use std::collections::BTreeMap;

/// Node representation within the command prefix tree (trie).
#[derive(Default)]
pub struct CommandNode {
    /// Documentation summary string.
    pub doc: &'static str,
    /// Async execution closure if the node represents an executable terminal command.
    pub handler: Option<HandlerFn>,
    /// Command-specific flags.
    pub flags: Vec<FlagSpec>,
    /// Positional token requirements.
    pub positional_tokens: Vec<TokenSpec>,
    /// Subcommand nodes indexed by literal command name.
    pub subcommands: BTreeMap<String, CommandNode>,
    /// Indicates whether this command is hidden from auto-generated help output.
    pub hidden: bool,
}

impl CommandNode {
    /// Creates a new `CommandNode` initialized with documentation text.
    pub fn new(doc: &'static str) -> Self {
        Self {
            doc,
            ..Default::default()
        }
    }

    /// Recursively checks if the node or any of its children contain executable targets.
    pub fn has_visible_targets(&self) -> bool {
        if self.hidden {
            return false;
        }
        if self.handler.is_some() {
            return true;
        }
        self.subcommands
            .values()
            .any(|child| child.has_visible_targets())
    }
}
