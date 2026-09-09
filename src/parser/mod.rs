//! Command-line argument parsing and command routing engine.
//!
//! Provides a prefix-tree (trie) based command dispatcher supporting nested subcommands,
//! positional arguments, optional/required flags with explicit aliases, auto-generated help messages, and hidden commands.

pub mod context;
pub use context::Context;

pub mod node;
pub use node::CommandNode;

pub mod spec;
pub use spec::{FlagKind, FlagSpec, RouteSpec, TokenSpec};

pub mod meta;
pub use meta::PkgMeta;

use crate::{prelude::*, render::ansi};

use crossterm::style::Stylize;
use heck::ToTitleCase;
use std::{future::Future, pin::Pin};
use strsim::levenshtein;

/// Type alias for heap-allocated, thread-safe dynamic futures.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Type alias for standard asynchronous command handlers.
pub type HandlerFn = Box<dyn Fn(Context) -> BoxFuture<'static, Result<()>> + Send + Sync>;

/// Represents the final internal resolution of command execution.
enum DispatchOutput {
    /// Handler finished execution returning a standard result.
    Executed(Result<()>),
    /// Help message string generated for display.
    Help(String),
    /// Version message string generated for display.
    Version(String),
}

/// Root builder and execution router for CLI applications.
pub struct Commands {
    name: String,
    description: String,
    version: String,
    global_flags: Vec<FlagSpec>,
    root: CommandNode,
}

impl Commands {
    /// Initializes a new `Commands` router with defaults extracted from Cargo environment variables.
    pub fn new() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            version: String::new(),
            global_flags: Vec::new(),
            root: CommandNode::default(),
        }
    }

    pub fn meta(mut self, meta: PkgMeta) -> Self {
        self.name = meta.name.into();
        self.description = meta.description.into();
        self.version = meta.version.into();
        self
    }

    /// Sets the binary or application name displayed in help outputs.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Sets the main description of the application.
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Sets the application version string.
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Registers global command-line options available across all subcommands.
    pub fn args(mut self, flags: &[&str]) -> Self {
        for flag in flags {
            if let Some(spec) = parse_single_flag(flag) {
                self.global_flags.push(spec);
            }
        }
        self
    }

    /// Registers a command group without an execution handler (used for nesting subcommands).
    pub fn group(mut self, pattern: &'static str, doc: &'static str) -> Self {
        let spec = parse_pattern(pattern, doc);
        self.insert_group(spec);
        self
    }

    /// Registers a publicly visible subcommand with an asynchronous execution handler.
    pub fn cmd<F, Fut>(mut self, pattern: &'static str, doc: &'static str, handler: F) -> Self
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let spec = parse_pattern(pattern, doc);
        self.insert_route(spec, Box::new(move |ctx| Box::pin(handler(ctx))), false);
        self
    }

    /// Registers a hidden subcommand with an asynchronous execution handler.
    ///
    /// Hidden commands remain executable but are excluded from auto-generated help listings.
    pub fn hide_cmd<F, Fut>(mut self, pattern: &'static str, doc: &'static str, handler: F) -> Self
    where
        F: Fn(Context) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        let spec = parse_pattern(pattern, doc);
        self.insert_route(spec, Box::new(move |ctx| Box::pin(handler(ctx))), true);
        self
    }

    fn insert_group(&mut self, spec: RouteSpec) {
        let mut current = &mut self.root;
        for token in spec.tokens {
            if let TokenSpec::Literal(name) = token {
                current = current
                    .subcommands
                    .entry(name)
                    .or_insert_with(|| CommandNode::new(""));
            }
        }
        current.doc = spec.doc;
    }

    fn insert_route(&mut self, spec: RouteSpec, handler: HandlerFn, hidden: bool) {
        let mut current = &mut self.root;

        for token in spec.tokens {
            match token {
                TokenSpec::Literal(name) => {
                    current = current
                        .subcommands
                        .entry(name)
                        .or_insert_with(|| CommandNode::new(""));
                }
                TokenSpec::Positional(_) | TokenSpec::Rest(_) => {
                    current.positional_tokens.push(token);
                }
            }
        }

        current.doc = spec.doc;
        current.flags = spec.flags;
        current.handler = Some(handler);
        current.hidden = hidden;
    }

    /// Main entrance point for executing CLI commands from process environment arguments.
    ///
    /// Returns `Ok(())` on successful execution or help/version display, and exits process on error.
    pub async fn run(self) -> Result<()> {
        let args: Vec<String> = std::env::args().skip(1).collect();

        match self.dispatch(&args).await {
            Ok(DispatchOutput::Executed(res)) => res,
            Ok(DispatchOutput::Help(help_str)) => {
                println!("{}", help_str);
                Ok(())
            }
            Ok(DispatchOutput::Version(ver_str)) => {
                println!("{}", ver_str);
                Ok(())
            }
            Err(err) => {
                eprintln!("{} {}", "Error:".red().bold(), err);
                std::process::exit(1);
            }
        }
    }

    /// Progressively dispatches CLI arguments against the command tree.
    async fn dispatch(&self, args: &[String]) -> StdResult<DispatchOutput, ParseError> {
        if args.first().map(|s| s.as_str()) == Some("-V")
            || args.first().map(|s| s.as_str()) == Some("--version")
        {
            return Ok(DispatchOutput::Version(format!(
                "{} {}",
                self.name, self.version
            )));
        }

        if args.is_empty() {
            return Ok(DispatchOutput::Help(
                self.generate_node_help(&self.root, &self.name),
            ));
        }

        let mut current = &self.root;
        let mut path = vec![self.name.clone()];
        let mut idx = 0;
        let mut failed_token: Option<&str> = None;

        // 1. Traverse literal subcommands
        while idx < args.len() {
            let token = &args[idx];

            if token.starts_with('-') {
                break;
            }

            if let Some(next_node) = current.subcommands.get(token) {
                current = next_node;
                path.push(token.clone());
                idx += 1;
            } else {
                failed_token = Some(token);
                break;
            }
        }

        let remaining = &args[idx..];
        let full_path = path.join(" ");

        // 2. Check for explicit help flags
        let is_help_requested = remaining
            .iter()
            .any(|arg| arg == "--help" || arg == "-h" || arg == "help");

        if is_help_requested && (current.handler.is_none() || remaining.is_empty()) {
            return Ok(DispatchOutput::Help(
                self.generate_node_help(current, &full_path),
            ));
        }

        // 3. Execute matched terminal handler or generate suggestions / help
        if let Some(ref handler) = current.handler {
            let (ctx, help_flag) = parse_and_validate_node(current, &self.global_flags, remaining)?;

            if help_flag {
                Ok(DispatchOutput::Help(
                    self.generate_node_help(current, &full_path),
                ))
            } else {
                let res = handler(ctx).await;
                Ok(DispatchOutput::Executed(res))
            }
        } else if failed_token.is_some() || !current.subcommands.is_empty() {
            if let Some(typo) = failed_token {
                let best_match = self.find_similar_command(current, typo);

                let help_hint = if let Some(suggestion) = best_match {
                    format!(
                        "  {} {}{}",
                        "Did you mean".bold(),
                        format!("{} {}", full_path, suggestion).green().bold(),
                        "?".bold()
                    )
                } else {
                    format!(
                        "  {} '{}' {}",
                        "Run".bold(),
                        format!("{} --help", full_path).cyan().bold(),
                        "to see available commands.".bold()
                    )
                };

                Err(ParseError::ErrorWithHelp(
                    Box::new(ParseError::UnknownCommand(typo.to_string())),
                    help_hint,
                ))
            } else {
                Ok(DispatchOutput::Help(
                    self.generate_node_help(current, &full_path),
                ))
            }
        } else {
            Err(ParseError::UnknownCommand(args.join(" ")))
        }
    }

    /// Finds the most suitable team based on the Levenshtein distance.
    fn find_similar_command<'a>(&self, node: &'a CommandNode, input: &str) -> Option<&'a str> {
        let mut best_candidate = None;
        let mut min_distance = usize::MAX;

        for (name, child) in &node.subcommands {
            if child.hidden || !child.has_visible_targets() {
                continue;
            }

            let dist = levenshtein(input, name);

            // allow a typo threshold depending on the word length:
            // (for short words (<= 3 characters) — a maximum of 1 typo,
            // for long words — up to 3 typos).
            let max_allowed_dist = match input.len() {
                0..=3 => 1,
                4..=6 => 2,
                _ => 3,
            };

            if dist <= max_allowed_dist && dist < min_distance {
                min_distance = dist;
                best_candidate = Some(name.as_str());
            }
        }

        best_candidate
    }

    /// Generates formatted help string for a target command node.
    fn generate_node_help(&self, node: &CommandNode, path: &str) -> String {
        let mut out = String::new();
        let is_root = path == self.name;

        if is_root {
            let formatted_name = self.name.to_title_case();
            let mut header = formatted_name.as_str().bold().to_string();

            if !self.version.is_empty() {
                header.push_str(&format!(" ({})", format!("v{}", self.version).grey()));
            }
            if !self.description.is_empty() {
                header.push_str(&format!(" — {}", self.description));
            }
            out.push_str(&format!("{}\n\n", header));
        } else if !node.doc.is_empty() {
            out.push_str(&format!("{}\n\n", node.doc));
        }

        let has_visible_subcommands = node
            .subcommands
            .values()
            .any(|sub| sub.has_visible_targets());

        let mut usage = format!("{} {}", "Usage:".bold().underlined(), path.bold());
        if has_visible_subcommands {
            usage.push_str(&" [COMMAND]".dim().to_string());
        }
        if !node.positional_tokens.is_empty() {
            for pos in &node.positional_tokens {
                match pos {
                    TokenSpec::Positional(p) => usage.push_str(&format!(" {{{}}}", p)),
                    TokenSpec::Rest(r) => usage.push_str(&format!(" {{{}..}}", r)),
                    _ => {}
                }
            }
        }
        usage.push_str(&" [OPTIONS]\n\n".dim().to_string());
        out.push_str(&usage);

        if has_visible_subcommands {
            out.push_str(&"Commands:\n".bold().underlined().to_string());

            let mut max_width = "help".len();
            for (sub_name, sub_node) in &node.subcommands {
                if sub_node.has_visible_targets() {
                    max_width = max_width.max(sub_name.len());
                }
            }
            let target_width = max_width + 4;

            for (sub_name, sub_node) in &node.subcommands {
                if !sub_node.has_visible_targets() {
                    continue;
                }

                let formatted = sub_name.as_str().bold().to_string();
                let pad = " ".repeat(target_width.saturating_sub(sub_name.len()));

                let doc_summary = if !sub_node.doc.is_empty() {
                    sub_node.doc.to_string()
                } else {
                    let mut leafs = Vec::new();
                    Self::collect_subcommand_leafs(sub_node, "", &mut leafs);
                    if leafs.is_empty() {
                        "Command group".to_string()
                    } else {
                        format!("[{}]", leafs.join(", "))
                    }
                };

                out.push_str(&format!("  {}{}{}\n", formatted, pad, doc_summary));
            }

            if is_root {
                let help_cmd = "help".bold().to_string();
                let pad = " ".repeat(target_width.saturating_sub("help".len()));
                out.push_str(&format!(
                    "  {}{}{}\n",
                    help_cmd, pad, "Print this message or the help of the given subcommand(s)."
                ));
            }
            out.push('\n');
        }

        out.push_str(&"Options:\n".bold().underlined().to_string());

        let all_flags: Vec<FlagSpec> = node
            .flags
            .iter()
            .chain(self.global_flags.iter())
            .cloned()
            .collect();

        let help_flag_str = format_option_flags("h", "help");
        let ver_flag_str = if is_root {
            format_option_flags("V", "version")
        } else {
            String::new()
        };

        let mut max_flag_width =
            ansi::visible_width(&help_flag_str).max(ansi::visible_width(&ver_flag_str));

        for flag in &all_flags {
            let flag_fmt = format_flag_name(flag);
            max_flag_width = max_flag_width.max(ansi::visible_width(&flag_fmt));
        }

        let target_option_width = max_flag_width + 4;

        for flag in &all_flags {
            out.push_str(&format_flag_help(flag, target_option_width, "  "));
        }

        let pad_h =
            " ".repeat(target_option_width.saturating_sub(ansi::visible_width(&help_flag_str)));
        out.push_str(&format!(
            "  {}{}{}\n",
            help_flag_str, pad_h, "Print help message."
        ));

        if is_root {
            let pad_v =
                " ".repeat(target_option_width.saturating_sub(ansi::visible_width(&ver_flag_str)));
            out.push_str(&format!(
                "  {}{}{}\n",
                ver_flag_str, pad_v, "Print program version."
            ));
        }

        out
    }

    fn collect_subcommand_leafs(node: &CommandNode, prefix: &str, acc: &mut Vec<String>) {
        if node.hidden || !node.has_visible_targets() {
            return;
        }

        if node.handler.is_some() || node.subcommands.is_empty() {
            if !prefix.is_empty() {
                acc.push(prefix.to_string());
            }
            return;
        }

        for (name, child) in &node.subcommands {
            if child.hidden || !child.has_visible_targets() {
                continue;
            }
            let next_prefix = if prefix.is_empty() {
                name.clone()
            } else {
                format!("{} {}", prefix, name)
            };
            Self::collect_subcommand_leafs(child, &next_prefix, acc);
        }
    }
}

/// Parses a raw flag token (including combined aliases like `-l|--load|--pull`) into a `FlagSpec`.
fn parse_single_flag(raw: &str) -> Option<FlagSpec> {
    let (names_part, default_val) = if let Some((left, right)) = raw.split_once('=') {
        (left, Some(right.to_string()))
    } else {
        (raw, None)
    };

    let names: Vec<String> = names_part
        .split('|')
        .map(|s| s.trim().trim_start_matches('-').to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if names.is_empty() {
        return None;
    }

    let kind = match default_val {
        Some(val) => FlagKind::Optional(val),
        None => FlagKind::Required,
    };

    Some(FlagSpec { names, kind })
}

/// Parses a pattern string containing command name, positionals, and flags into a `RouteSpec`.
fn parse_pattern(pattern: &'static str, doc: &'static str) -> RouteSpec {
    let mut tokens = Vec::new();
    let mut flags = Vec::new();

    for part in pattern.split_whitespace() {
        if part.starts_with('-') {
            if let Some(spec) = parse_single_flag(part) {
                flags.push(spec);
            }
        } else if part.starts_with('{') && part.ends_with('}') {
            let inner = &part[1..part.len() - 1];
            if inner.starts_with("..") {
                tokens.push(TokenSpec::Rest(inner.trim_start_matches("..").to_string()));
            } else if inner.ends_with("..") {
                tokens.push(TokenSpec::Rest(inner.trim_end_matches("..").to_string()));
            } else {
                tokens.push(TokenSpec::Positional(inner.to_string()));
            }
        } else {
            tokens.push(TokenSpec::Literal(part.to_string()));
        }
    }

    RouteSpec {
        pattern_raw: pattern,
        doc,
        tokens,
        flags,
    }
}

/// Validates arguments and builds execution `Context` for a resolved node.
fn parse_and_validate_node(
    node: &CommandNode,
    global_flags: &[FlagSpec],
    remaining_tokens: &[String],
) -> StdResult<(Context, bool), ParseError> {
    let mut is_help_requested = false;
    let mut input_words = Vec::new();

    for token in remaining_tokens {
        if token == "--help" || token == "-h" {
            is_help_requested = true;
        } else {
            input_words.push(token.as_str());
        }
    }

    let all_flags: Vec<_> = node
        .flags
        .iter()
        .chain(global_flags.iter())
        .cloned()
        .collect();

    let mut flag_lookup: HashMap<String, (&FlagSpec, bool)> = HashMap::new();

    for flag in &all_flags {
        for name in &flag.names {
            let prefix = if name.len() == 1 { "-" } else { "--" };
            flag_lookup.insert(format!("{}{}", prefix, name), (flag, false));

            if let FlagKind::Optional(ref default_val) = flag.kind {
                if default_val == "true" && name.len() > 1 {
                    flag_lookup.insert(format!("--no-{}", name), (flag, true));
                }
            }
        }
    }

    let mut input_flags = HashMap::new();
    let mut clean_words = Vec::new();

    let mut i = 0;
    while i < input_words.len() {
        let word = input_words[i];

        if word.starts_with('-') {
            let (flag_key, inline_val) = word.split_once('=').unwrap_or((word, ""));

            if let Some((flag_spec, is_negated)) = flag_lookup.get(flag_key) {
                let canonical = flag_spec.canonical_name().to_string();

                // determine whether the flag is Boolean based on its default value.
                let is_boolean_flag = match &flag_spec.kind {
                    FlagKind::Optional(val) => val == "true" || val == "false",
                    FlagKind::Required => false,
                };

                if !inline_val.is_empty() {
                    // the value is passed via '='
                    let final_val = if *is_negated {
                        if inline_val.parse::<bool>().unwrap_or(true) {
                            "false"
                        } else {
                            "true"
                        }
                    } else {
                        inline_val
                    };
                    input_flags.insert(canonical, final_val.to_string());
                } else if *is_negated {
                    input_flags.insert(canonical, "false".to_string());
                } else if is_boolean_flag {
                    // for boolean flags, check whether “true” / “false” was explicitly passed further.
                    if i + 1 < input_words.len()
                        && (input_words[i + 1] == "true" || input_words[i + 1] == "false")
                    {
                        input_flags.insert(canonical, input_words[i + 1].to_string());
                        i += 1;
                    } else {
                        input_flags.insert(canonical, "true".to_string());
                    }
                } else {
                    // the flag value is passed as a space‑separated string: --uid 1
                    if i + 1 < input_words.len() && !input_words[i + 1].starts_with('-') {
                        input_flags.insert(canonical, input_words[i + 1].to_string());
                        i += 1;
                    } else if matches!(flag_spec.kind, FlagKind::Required) {
                        return Err(ParseError::MissingRequiredFlag(canonical));
                    }
                }
            } else {
                return Err(ParseError::UnknownFlag(word.to_string()));
            }
        } else {
            clean_words.push(word);
        }
        i += 1;
    }

    let mut ctx = Context::default();
    let mut word_idx = 0;

    for token in &node.positional_tokens {
        match token {
            TokenSpec::Positional(name) => {
                if word_idx >= clean_words.len() {
                    if !is_help_requested {
                        return Err(ParseError::MissingPositional(name.clone()));
                    }
                } else {
                    ctx.args
                        .insert(name.clone(), clean_words[word_idx].to_string());
                    word_idx += 1;
                }
            }
            TokenSpec::Rest(name) => {
                let rest_text = clean_words[word_idx..].join(" ");
                let key = if name.is_empty() {
                    "rest".to_string()
                } else {
                    name.clone()
                };
                ctx.args.insert(key, rest_text);
                word_idx = clean_words.len();
                break;
            }
            _ => {}
        }
    }

    if word_idx < clean_words.len() && !is_help_requested {
        return Err(ParseError::UnexpectedArgument(
            clean_words[word_idx].to_string(),
        ));
    }

    for flag_spec in &all_flags {
        let canonical = flag_spec.canonical_name();
        if let Some(val) = input_flags.get(canonical) {
            ctx.args.insert(canonical.to_string(), val.clone());
        } else {
            match &flag_spec.kind {
                FlagKind::Optional(default_val) => {
                    ctx.args.insert(canonical.to_string(), default_val.clone());
                }
                FlagKind::Required => {
                    if !is_help_requested {
                        return Err(ParseError::MissingRequiredFlag(canonical.to_string()));
                    }
                }
            }
        }
    }

    Ok((ctx, is_help_requested))
}

fn format_option_flags(short: &str, long: &str) -> String {
    format!("-{}, --{}", short.bold(), long.bold())
}

fn format_flag_name(flag: &FlagSpec) -> String {
    let explicit_short = flag.explicit_short();

    let long_names: Vec<&str> = flag
        .names
        .iter()
        .filter(|n| n.len() > 1)
        .map(|s| s.as_str())
        .collect();

    let long_str = long_names
        .iter()
        .map(|name| format!("--{}", name.bold()))
        .collect::<Vec<_>>()
        .join(", ");

    if let Some(s) = explicit_short {
        if long_str.is_empty() {
            format!("-{}", s.to_string().bold())
        } else {
            format!("-{}, {}", s.to_string().bold(), long_str)
        }
    } else {
        format!("    {}", long_str)
    }
}

fn format_flag_help(flag: &FlagSpec, target_width: usize, indent: &str) -> String {
    let flag_name = format_flag_name(flag);
    let pad = " ".repeat(target_width.saturating_sub(ansi::visible_width(&flag_name)));

    match &flag.kind {
        FlagKind::Required => {
            format!("{}{}{}{}\n", indent, flag_name, pad, "(required)".grey())
        }
        FlagKind::Optional(d) => {
            format!(
                "{}{}{}{}\n",
                indent,
                flag_name,
                pad,
                format!("[default: {}]", d).grey()
            )
        }
    }
}
