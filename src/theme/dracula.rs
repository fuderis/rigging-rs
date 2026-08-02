use super::CodeTheme;
use crossterm::style::Color;

/// Dracula Theme (Контрастная культовая темная тема)
pub const OFFICIAL: CodeTheme = CodeTheme {
    keyword: Color::Rgb {
        r: 255,
        g: 121,
        b: 198,
    }, // Pink (#ff79c6)
    type_name: Color::Rgb {
        r: 139,
        g: 233,
        b: 253,
    }, // Cyan (#8be9fd)
    function: Color::Rgb {
        r: 80,
        g: 250,
        b: 123,
    }, // Green (#50fa7b)
    macro_name: Color::Rgb {
        r: 255,
        g: 184,
        b: 108,
    }, // Orange (#ffb86c)
    builtin: Color::Rgb {
        r: 139,
        g: 233,
        b: 253,
    }, // Cyan
    operator: Color::Rgb {
        r: 255,
        g: 121,
        b: 198,
    }, // Pink
    string: Color::Rgb {
        r: 241,
        g: 250,
        b: 140,
    }, // Yellow (#f1fa8c)
    number: Color::Rgb {
        r: 189,
        g: 147,
        b: 249,
    }, // Purple (#bd93f9)
    comment: Color::Rgb {
        r: 98,
        g: 114,
        b: 164,
    }, // Comment Blue/Gray (#6272a4)
    variable: Color::Rgb {
        r: 248,
        g: 248,
        b: 242,
    }, // Foreground (#f8f8f2)
    property: Color::Rgb {
        r: 255,
        g: 121,
        b: 198,
    }, // Pink
    constant: Color::Rgb {
        r: 189,
        g: 147,
        b: 249,
    }, // Purple
};
