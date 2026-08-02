/// Represents a title attached to a block's border.
#[derive(Debug, Clone)]
pub struct Title {
    /// The text content of the title.
    pub text: String,
    /// The alignment of the title along the block border.
    pub align: Align,
}

/// Defines internal spacing between the block's border and its content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Padding {
    /// Top padding in character units.
    pub top: usize,
    /// Right padding in character units.
    pub right: usize,
    /// Bottom padding in character units.
    pub bottom: usize,
    /// Left padding in character units.
    pub left: usize,
}

impl Padding {
    /// Creates a new `Padding` instance with explicit values for each side.
    pub fn new(top: usize, right: usize, bottom: usize, left: usize) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Creates vertical padding with equal `top` and `bottom` values, setting horizontal padding to zero.
    pub fn ver(indent: usize) -> Self {
        Self::new(indent, 0, indent, 0)
    }

    /// Creates horizontal padding with equal `left` and `right` values, setting vertical padding to zero.
    pub fn hor(indent: usize) -> Self {
        Self::new(0, indent, 0, indent)
    }

    /// Sets the top padding value.
    pub fn top(mut self, indent: usize) -> Self {
        self.top = indent;
        self
    }

    /// Sets the bottom padding value.
    pub fn bottom(mut self, indent: usize) -> Self {
        self.bottom = indent;
        self
    }

    /// Sets the left padding value.
    pub fn left(mut self, indent: usize) -> Self {
        self.left = indent;
        self
    }

    /// Sets the right padding value.
    pub fn right(mut self, indent: usize) -> Self {
        self.right = indent;
        self
    }
}

/// Defines external spacing outside the block's border.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Margin {
    /// Top margin in character units.
    pub top: usize,
    /// Right margin in character units.
    pub right: usize,
    /// Bottom margin in character units.
    pub bottom: usize,
    /// Left margin in character units.
    pub left: usize,
}

impl Margin {
    /// Creates a new `Margin` instance with explicit values for each side.
    pub fn new(top: usize, right: usize, bottom: usize, left: usize) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    /// Creates vertical margin with equal `top` and `bottom` values, setting horizontal margin to zero.
    pub fn ver(indent: usize) -> Self {
        Self::new(indent, 0, indent, 0)
    }

    /// Creates horizontal margin with equal `left` and `right` values, setting vertical margin to zero.
    pub fn hor(indent: usize) -> Self {
        Self::new(0, indent, 0, indent)
    }

    /// Sets the top margin value.
    pub fn top(mut self, indent: usize) -> Self {
        self.top = indent;
        self
    }

    /// Sets the bottom margin value.
    pub fn bottom(mut self, indent: usize) -> Self {
        self.bottom = indent;
        self
    }

    /// Sets the left margin value.
    pub fn left(mut self, indent: usize) -> Self {
        self.left = indent;
        self
    }

    /// Sets the right margin value.
    pub fn right(mut self, indent: usize) -> Self {
        self.right = indent;
        self
    }
}

/// Defines the visual style of the block's border.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BorderStyle {
    /// No border is drawn.
    #[default]
    None,
    /// Single thin line border (`┌ ┐ └ ┘ ─ │`).
    Solid,
    /// Single thin line border with rounded corners (`╭ ╮ ╰ ╯ ─ │`).
    Rounded,
    /// Double line border (`╔ ╗ ╚ ╝ ═ ║`).
    Double,
}

impl BorderStyle {
    /// Returns the character set forming the border corners and sides.
    ///
    /// The returned tuple follows the order: `(top_left, top_right, bottom_left, bottom_right, horizontal, vertical)`.
    pub fn as_chars(&self) -> (char, char, char, char, char, char) {
        match self {
            BorderStyle::None => (' ', ' ', ' ', ' ', ' ', ' '),
            BorderStyle::Solid => ('┌', '┐', '└', '┘', '─', '│'),
            BorderStyle::Rounded => ('╭', '╮', '╰', '╯', '─', '│'),
            BorderStyle::Double => ('╔', '╗', '╚', '╝', '═', '║'),
        }
    }
}

/// Defines the alignment of titles on the border.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Align {
    /// Aligned to the left of the top border.
    #[default]
    TopLeft,
    /// Centered along the top border.
    TopCenter,
    /// Aligned to the right of the top border.
    TopRight,
    /// Aligned to the left of the bottom border.
    BottomLeft,
    /// Centered along the bottom border.
    BottomCenter,
    /// Aligned to the right of the bottom border.
    BottomRight,
}

/// Defines the visual style of a horizontal divider line under a prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LineStyle {
    /// No divider line.
    #[default]
    None,
    /// Solid line (`──────`).
    Solid,
    /// Dashed line (`╌╌╌╌╌╌`).
    Dashed,
    /// Dotted line (`┈┈┈┈┈┈`).
    Dotted,
    /// Double line (`══════`).
    Double,
    /// Heavy line (`━━━━━━`).
    Heavy,
}

impl LineStyle {
    /// Returns the character representing the line pattern.
    pub fn as_char(&self) -> char {
        match self {
            Self::None => ' ',
            Self::Solid => '─',
            Self::Dashed => '╌',
            Self::Dotted => '┈',
            Self::Double => '═',
            Self::Heavy => '━',
        }
    }
}

/// Animation presets for the spinner widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpinnerStyle {
    /// Classic Braille dot animation: `⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏`.
    #[default]
    Dots,
    /// Mini Braille dot animation: `⠄ ⠆ ⠇ ⠋ ⠙ ⠸ ⠰ ⠠`.
    MiniDots,
    /// Traditional ASCII pipe animation: `| / - \`.
    Line,
    /// No spinner icon (dynamically updated text only).
    None,
}

impl SpinnerStyle {
    /// Returns the character slice representing the frames for the selected spinner style.
    pub fn frames(&self) -> &'static [char] {
        match self {
            SpinnerStyle::Dots => &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'],
            SpinnerStyle::MiniDots => &['⠄', '⠆', '⠇', '⠋', '⠙', '⠸', '⠰', '⠠'],
            SpinnerStyle::Line => &['|', '/', '-', '\\'],
            SpinnerStyle::None => &[],
        }
    }
}

/// Visual styles for the vertical left border stripe of a quote block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StripeStyle {
    /// Without line.
    #[default]
    None,
    /// Single thin line (`│`).
    Single,
    /// Double line (`║`).
    Double,
    /// Thick solid block (`▌`).
    Thick,
    /// Dotted line (`┊`).
    Dotted,
    /// Custom character supplied by the caller.
    Custom(char),
}

impl StripeStyle {
    /// Returns the character representation of the current stripe style.
    pub fn char(&self) -> char {
        match self {
            StripeStyle::None => ' ',
            StripeStyle::Single => '│',
            StripeStyle::Double => '║',
            StripeStyle::Thick => '▌',
            StripeStyle::Dotted => '┊',
            StripeStyle::Custom(ch) => *ch,
        }
    }
}

/// Defines the style of the bullet point used in a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BulletStyle {
    /// Filled bullet dot (`•`).
    #[default]
    Dot,
    /// Hollow circle (`◦`).
    Circle,
    /// Small square (`▪`).
    Square,
    /// Dash symbol (`-`).
    Dash,
    /// Right-pointing arrow (`➔`).
    Arrow,
    /// Solid triangle (`▶`).
    Triangle,
    /// Star symbol (`★`).
    Star,
    /// Checkmark (`✔`).
    Check,
    /// Cross mark (`✖`).
    Cross,
    /// Indexed number string (`1.`, `2.`, `3.`).
    Number,
    /// Custom static string bullet symbol.
    Custom(&'static str),
}

impl BulletStyle {
    /// Renders the bullet point symbol as a string, taking the item index into account for numbered lists.
    pub fn render_symbol(&self, index: usize) -> String {
        match self {
            Self::Dot => "•".to_string(),
            Self::Circle => "◦".to_string(),
            Self::Square => "▪".to_string(),
            Self::Dash => "-".to_string(),
            Self::Arrow => "➔".to_string(),
            Self::Triangle => "▶".to_string(),
            Self::Star => "★".to_string(),
            Self::Check => "✔".to_string(),
            Self::Cross => "✖".to_string(),
            Self::Number => format!("{}.", index + 1),
            Self::Custom(s) => s.to_string(),
        }
    }
}

/// Represents various visual separators between key-value pairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SeparatorStyle {
    /// Colon symbol (`:`).
    #[default]
    Colon,
    /// Arrow symbol (`➔`).
    Arrow,
    /// Fat arrow symbol (`⇒`).
    FatArrow,
    /// Pipe symbol (`|`).
    Pipe,
    /// Bullet dot symbol (`•`).
    Dot,
    /// Tilde symbol (`~`).
    Tilde,
    /// Custom static string separator.
    Custom(&'static str),
}

impl SeparatorStyle {
    /// Returns the string representation of the separator.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Colon => ":",
            Self::Arrow => "➔",
            Self::FatArrow => "⇒",
            Self::Pipe => "|",
            Self::Dot => "•",
            Self::Tilde => "~",
            Self::Custom(s) => s,
        }
    }
}
