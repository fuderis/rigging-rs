use crossterm::style::Color;

/// Syntax highlight colors for tokens supported by Tree-sitter.
#[derive(Debug, Clone, Copy)]
pub struct CodeTheme {
    /// Color for control keywords and reserved words.
    pub keyword: Color,
    /// Color for types, structs, enums, and constructors.
    pub type_name: Color,
    /// Color for functions and method calls.
    pub function: Color,
    /// Color for macro definitions and invocations.
    pub macro_name: Color,
    /// Color for language built-ins and primitive types.
    pub builtin: Color,
    /// Color for mathematical and logical operators.
    pub operator: Color,
    /// Color for string literals.
    pub string: Color,
    /// Color for numeric and boolean literals.
    pub number: Color,
    /// Color for code comments.
    pub comment: Color,
    /// Color for variable names.
    pub variable: Color,
    /// Color for object properties and struct fields.
    pub property: Color,
    /// Color for constants and static variables.
    pub constant: Color,
}

impl CodeTheme {
    /// List of Tree-sitter highlight query capture names to match against.
    pub const HIGHLIGHT_NAMES: &'static [&'static str] = &[
        "keyword",
        "type",
        "constructor",
        "function",
        "function.method",
        "function.macro",
        "builtin",
        "operator",
        "string",
        "number",
        "boolean",
        "comment",
        "variable",
        "property",
        "constant",
    ];

    /// Maps a Tree-sitter capture index to its corresponding theme color.
    pub fn color_for_index(&self, index: usize) -> Color {
        // match tree-sitter highlight index to the theme color
        match Self::HIGHLIGHT_NAMES.get(index).copied() {
            Some("keyword") => self.keyword,
            Some("type") | Some("constructor") => self.type_name,
            Some("function") | Some("function.method") => self.function,
            Some("function.macro") => self.macro_name,
            Some("builtin") => self.builtin,
            Some("operator") => self.operator,
            Some("string") => self.string,
            Some("number") | Some("boolean") => self.number,
            Some("comment") => self.comment,
            Some("variable") => self.variable,
            Some("property") => self.property,
            Some("constant") => self.constant,
            _ => Color::Reset,
        }
    }
}

impl Default for CodeTheme {
    /// Returns the default theme (Catppuccin Mocha).
    fn default() -> Self {
        super::catppuccin::MOCHA
    }
}
