use super::CodeTheme;
use crossterm::style::Color;

/// VS Code Dark Plus (Стандартная тема Visual Studio Code)
pub const DARK_PLUS: CodeTheme = CodeTheme {
    keyword: Color::Rgb {
        r: 197,
        g: 134,
        b: 192,
    }, // Pinkish Purple (#c586c0)
    type_name: Color::Rgb {
        r: 78,
        g: 201,
        b: 176,
    }, // Turquoise (#4ec9b0)
    function: Color::Rgb {
        r: 220,
        g: 220,
        b: 170,
    }, // Light Yellow (#dcdcaa)
    macro_name: Color::Rgb {
        r: 78,
        g: 201,
        b: 176,
    }, // Teal/Green
    builtin: Color::Rgb {
        r: 86,
        g: 156,
        b: 214,
    }, // Soft Blue (#569cd6)
    operator: Color::Rgb {
        r: 214,
        g: 214,
        b: 214,
    }, // Light Gray
    string: Color::Rgb {
        r: 206,
        g: 145,
        b: 120,
    }, // Brown/Orange (#ce9178)
    number: Color::Rgb {
        r: 181,
        g: 206,
        b: 168,
    }, // Light Green (#b5cea8)
    comment: Color::Rgb {
        r: 106,
        g: 153,
        b: 85,
    }, // Green Comment (#6a9955)
    variable: Color::Rgb {
        r: 156,
        g: 220,
        b: 254,
    }, // Light Blue (#9cdcfe)
    property: Color::Rgb {
        r: 156,
        g: 220,
        b: 254,
    }, // Light Blue
    constant: Color::Rgb {
        r: 79,
        g: 193,
        b: 255,
    }, // Bright Sky Blue
};
