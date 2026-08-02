use super::CodeTheme;
use crossterm::style::Color;

/// Atom One Dark (Легендарная классика)
pub const ONE_DARK: CodeTheme = CodeTheme {
    keyword: Color::Rgb {
        r: 198,
        g: 120,
        b: 221,
    }, // Purple (#c678dd)
    type_name: Color::Rgb {
        r: 229,
        g: 192,
        b: 123,
    }, // Yellow (#e5c07b)
    function: Color::Rgb {
        r: 97,
        g: 175,
        b: 239,
    }, // Blue (#61afef)
    macro_name: Color::Rgb {
        r: 224,
        g: 108,
        b: 117,
    }, // Red (#e06c75)
    builtin: Color::Rgb {
        r: 86,
        g: 182,
        b: 194,
    }, // Cyan (#56b6c2)
    operator: Color::Rgb {
        r: 86,
        g: 182,
        b: 194,
    }, // Cyan
    string: Color::Rgb {
        r: 152,
        g: 195,
        b: 121,
    }, // Green (#98c379)
    number: Color::Rgb {
        r: 209,
        g: 154,
        b: 102,
    }, // Dark Yellow/Orange (#d19a66)
    comment: Color::Rgb {
        r: 92,
        g: 99,
        b: 112,
    }, // Chalk Gray (#5c6370)
    variable: Color::Rgb {
        r: 171,
        g: 178,
        b: 191,
    }, // Foreground (#abb2bf)
    property: Color::Rgb {
        r: 224,
        g: 108,
        b: 117,
    }, // Red
    constant: Color::Rgb {
        r: 209,
        g: 154,
        b: 102,
    }, // Orange
};

/// Atom One Light / Rust Docs Theme (Официальная тема Rustdoc "Light/Source")
pub const RUST_DOCS: CodeTheme = CodeTheme {
    keyword: Color::Rgb {
        r: 137,
        g: 89,
        b: 168,
    }, // Deep Purple
    type_name: Color::Rgb {
        r: 40,
        g: 116,
        b: 166,
    }, // Steel Blue
    function: Color::Rgb {
        r: 66,
        g: 113,
        b: 174,
    }, // Blue
    macro_name: Color::Rgb {
        r: 62,
        g: 153,
        b: 159,
    }, // Cyan Macro
    builtin: Color::Rgb {
        r: 199,
        g: 37,
        b: 78,
    }, // Dark Pink
    operator: Color::Rgb {
        r: 62,
        g: 153,
        b: 159,
    }, // Cyan
    string: Color::Rgb {
        r: 113,
        g: 140,
        b: 0,
    }, // Olive Green (Классический rustdoc)
    number: Color::Rgb {
        r: 249,
        g: 145,
        b: 87,
    }, // Orange
    comment: Color::Rgb {
        r: 142,
        g: 144,
        b: 140,
    }, // Gray
    variable: Color::Rgb {
        r: 77,
        g: 78,
        b: 76,
    }, // Dark Charcoal
    property: Color::Rgb {
        r: 200,
        g: 40,
        b: 40,
    }, // Reddish
    constant: Color::Rgb {
        r: 249,
        g: 145,
        b: 87,
    }, // Orange
};
