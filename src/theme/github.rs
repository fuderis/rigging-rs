use super::CodeTheme;
use crossterm::style::Color;

/// GitHub Dark Default
pub const DARK: CodeTheme = CodeTheme {
    keyword: Color::Rgb {
        r: 255,
        g: 123,
        b: 114,
    }, // Coral / Red
    type_name: Color::Rgb {
        r: 121,
        g: 192,
        b: 255,
    }, // Light Blue
    function: Color::Rgb {
        r: 210,
        g: 168,
        b: 255,
    }, // Purple
    macro_name: Color::Rgb {
        r: 255,
        g: 166,
        b: 87,
    }, // Orange
    builtin: Color::Rgb {
        r: 121,
        g: 192,
        b: 255,
    }, // Light Blue
    operator: Color::Rgb {
        r: 255,
        g: 123,
        b: 114,
    }, // Red
    string: Color::Rgb {
        r: 165,
        g: 214,
        b: 255,
    }, // Soft Cyan/Blue
    number: Color::Rgb {
        r: 121,
        g: 192,
        b: 255,
    }, // Cyan
    comment: Color::Rgb {
        r: 139,
        g: 148,
        b: 158,
    }, // Muted Gray
    variable: Color::Rgb {
        r: 201,
        g: 209,
        b: 217,
    }, // Whiteish Text
    property: Color::Rgb {
        r: 121,
        g: 192,
        b: 255,
    }, // Cyan
    constant: Color::Rgb {
        r: 121,
        g: 192,
        b: 255,
    }, // Cyan
};

/// GitHub Light Default
pub const LIGHT: CodeTheme = CodeTheme {
    keyword: Color::Rgb {
        r: 207,
        g: 34,
        b: 46,
    }, // Deep Red
    type_name: Color::Rgb {
        r: 14,
        g: 116,
        b: 144,
    }, // Teal/Blue
    function: Color::Rgb {
        r: 130,
        g: 80,
        b: 223,
    }, // Purple
    macro_name: Color::Rgb {
        r: 149,
        g: 56,
        b: 0,
    }, // Brown/Orange
    builtin: Color::Rgb {
        r: 5,
        g: 80,
        b: 174,
    }, // Dark Blue
    operator: Color::Rgb {
        r: 207,
        g: 34,
        b: 46,
    }, // Red
    string: Color::Rgb {
        r: 10,
        g: 48,
        b: 105,
    }, // Dark Navy String
    number: Color::Rgb {
        r: 5,
        g: 80,
        b: 174,
    }, // Blue
    comment: Color::Rgb {
        r: 108,
        g: 117,
        b: 125,
    }, // Gray
    variable: Color::Rgb {
        r: 36,
        g: 41,
        b: 47,
    }, // Off Black
    property: Color::Rgb {
        r: 5,
        g: 80,
        b: 174,
    }, // Blue
    constant: Color::Rgb {
        r: 5,
        g: 80,
        b: 174,
    }, // Blue
};
