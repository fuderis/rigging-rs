use std::fmt;

/// Represents errors that can occur when converting text to superscript.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SmallTextError {
    /// Encountered a character that does not have a superscript equivalent in Unicode.
    UnsupportedChar { ch: char, position: usize },
}

impl fmt::Display for SmallTextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedChar { ch, position } => {
                // format and write the unsupported character error message
                write!(
                    f,
                    "SmallText error: character '{}' (pos {}) has no Unicode superscript representation",
                    ch, position
                )
            }
        }
    }
}

impl std::error::Error for SmallTextError {}

/// Converts a standard string into superscript text.
///
/// Returns an error if a character is encountered that lacks a Unicode superscript equivalent.
pub fn to_superscript(input: &str) -> Result<String, SmallTextError> {
    // allocate a string with the same capacity as the input
    let mut result = String::with_capacity(input.len());

    // iterate over the characters and their indices
    for (idx, ch) in input.chars().enumerate() {
        let small_ch = match ch {
            // --- lowercase letters ---
            'a' => 'ᵃ',
            'b' => 'ᵇ',
            'c' => 'ᶜ',
            'd' => 'ᵈ',
            'e' => 'ᵉ',
            'f' => 'ᶠ',
            'g' => 'ᵍ',
            'h' => 'ʰ',
            'i' => 'ⁱ',
            'j' => 'ʲ',
            'k' => 'ᵏ',
            'l' => 'ˡ',
            'm' => 'ᵐ',
            'n' => 'ⁿ',
            'o' => 'ᵒ',
            'p' => 'ᵖ', // 'q' is missing from the unicode specification!
            'r' => 'ʳ',
            's' => 'ˢ',
            't' => 'ᵗ',
            'u' => 'ᵘ',
            'v' => 'ᵛ',
            'w' => 'ʷ',
            'x' => 'ˣ',
            'y' => 'ʸ',
            'z' => 'ᶻ',

            // uppercase letters (those available in Unicode)
            'A' => 'ᴬ',
            'B' => 'ᴮ',
            'D' => 'ᴰ',
            'E' => 'ᴱ',
            'G' => 'ᴳ',
            'H' => 'ᴴ',
            'I' => 'ᴵ',
            'J' => 'ᴶ',
            'K' => 'ᴷ',
            'L' => 'ᴸ',
            'M' => 'ᴹ',
            'N' => 'ᴺ',
            'O' => 'ᴼ',
            'P' => 'ᴾ',
            'R' => 'ᴿ',
            'T' => 'ᵀ',
            'U' => 'ᵁ',
            'V' => 'ⱽ',
            'W' => 'ᵂ',
            // 'C', 'F', 'Q', 'X', 'Z' are officially missing in Unicode!

            // numbers
            '0' => '⁰',
            '1' => '¹',
            '2' => '²',
            '3' => '³',
            '4' => '⁴',
            '5' => '⁵',
            '6' => '⁶',
            '7' => '⁷',
            '8' => '⁸',
            '9' => '⁹',

            // pass basic punctuation and spaces as is
            ' ' => ' ',
            '-' => '⁻',
            '+' => '⁺',
            '=' => '⁼',
            '(' => '⁽',
            ')' => '⁾',

            // for everything else (or missing letters like q, C, Q, X...)
            unsupported => {
                // return an error with the unsupported character and its position
                return Err(SmallTextError::UnsupportedChar {
                    ch: unsupported,
                    position: idx,
                });
            }
        };

        // append the superscript character to the result
        result.push(small_ch);
    }

    Ok(result)
}
