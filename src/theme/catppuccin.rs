use super::CodeTheme;
use crossterm::style::Color;

pub const MOCHA: CodeTheme = CodeTheme {
    keyword: Color::Rgb {
        r: 203,
        g: 166,
        b: 247,
    }, // Mauve (#cba6f7)
    type_name: Color::Rgb {
        r: 249,
        g: 226,
        b: 175,
    }, // Yellow (#f9e2af)
    function: Color::Rgb {
        r: 137,
        g: 180,
        b: 250,
    }, // Blue (#89b4fa)
    macro_name: Color::Rgb {
        r: 245,
        g: 194,
        b: 231,
    }, // Pink (#f5c2e7)
    builtin: Color::Rgb {
        r: 148,
        g: 226,
        b: 213,
    }, // Teal (#94e2d5)
    operator: Color::Rgb {
        r: 137,
        g: 220,
        b: 235,
    }, // Sky (#89dceb)
    string: Color::Rgb {
        r: 166,
        g: 227,
        b: 161,
    }, // Green (#a6e3a1)
    number: Color::Rgb {
        r: 250,
        g: 179,
        b: 135,
    }, // Peach (#fab387)
    comment: Color::Rgb {
        r: 147,
        g: 153,
        b: 178,
    }, // Overlay0 (#9399b2)
    variable: Color::Rgb {
        r: 245,
        g: 224,
        b: 220,
    }, // Rosewater (#f5e0dc)
    property: Color::Rgb {
        r: 242,
        g: 205,
        b: 205,
    }, // Flamingo (#f2cdcd)
    constant: Color::Rgb {
        r: 254,
        g: 100,
        b: 11,
    }, // Red (#fe640b)
};

pub const MACCHIATO: CodeTheme = CodeTheme {
    keyword: Color::Rgb {
        r: 198,
        g: 160,
        b: 246,
    },
    type_name: Color::Rgb {
        r: 238,
        g: 212,
        b: 159,
    },
    function: Color::Rgb {
        r: 138,
        g: 173,
        b: 244,
    },
    macro_name: Color::Rgb {
        r: 245,
        g: 189,
        b: 230,
    },
    builtin: Color::Rgb {
        r: 139,
        g: 213,
        b: 202,
    },
    operator: Color::Rgb {
        r: 145,
        g: 215,
        b: 227,
    },
    string: Color::Rgb {
        r: 166,
        g: 218,
        b: 149,
    },
    number: Color::Rgb {
        r: 245,
        g: 169,
        b: 127,
    },
    comment: Color::Rgb {
        r: 147,
        g: 154,
        b: 183,
    },
    variable: Color::Rgb {
        r: 244,
        g: 219,
        b: 214,
    },
    property: Color::Rgb {
        r: 240,
        g: 198,
        b: 198,
    },
    constant: Color::Rgb {
        r: 237,
        g: 135,
        b: 150,
    },
};

pub const FRAPPE: CodeTheme = CodeTheme {
    keyword: Color::Rgb {
        r: 202,
        g: 158,
        b: 230,
    },
    type_name: Color::Rgb {
        r: 229,
        g: 200,
        b: 144,
    },
    function: Color::Rgb {
        r: 140,
        g: 170,
        b: 238,
    },
    macro_name: Color::Rgb {
        r: 244,
        g: 184,
        b: 228,
    },
    builtin: Color::Rgb {
        r: 129,
        g: 200,
        b: 190,
    },
    operator: Color::Rgb {
        r: 153,
        g: 209,
        b: 219,
    },
    string: Color::Rgb {
        r: 166,
        g: 209,
        b: 137,
    },
    number: Color::Rgb {
        r: 239,
        g: 159,
        b: 118,
    },
    comment: Color::Rgb {
        r: 147,
        g: 153,
        b: 178,
    },
    variable: Color::Rgb {
        r: 242,
        g: 213,
        b: 207,
    },
    property: Color::Rgb {
        r: 238,
        g: 190,
        b: 190,
    },
    constant: Color::Rgb {
        r: 231,
        g: 130,
        b: 132,
    },
};

pub const LATTE: CodeTheme = CodeTheme {
    keyword: Color::Rgb {
        r: 136,
        g: 57,
        b: 239,
    },
    type_name: Color::Rgb {
        r: 223,
        g: 142,
        b: 29,
    },
    function: Color::Rgb {
        r: 30,
        g: 102,
        b: 245,
    },
    macro_name: Color::Rgb {
        r: 234,
        g: 118,
        b: 203,
    },
    builtin: Color::Rgb {
        r: 23,
        g: 146,
        b: 153,
    },
    operator: Color::Rgb {
        r: 4,
        g: 165,
        b: 229,
    },
    string: Color::Rgb {
        r: 64,
        g: 160,
        b: 43,
    },
    number: Color::Rgb {
        r: 254,
        g: 100,
        b: 11,
    },
    comment: Color::Rgb {
        r: 156,
        g: 160,
        b: 176,
    },
    variable: Color::Rgb {
        r: 220,
        g: 138,
        b: 120,
    },
    property: Color::Rgb {
        r: 221,
        g: 120,
        b: 120,
    },
    constant: Color::Rgb {
        r: 210,
        g: 15,
        b: 57,
    },
};
