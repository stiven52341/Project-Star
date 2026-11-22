//! Mistakes the lexer can find in the source text.

use std::error::Error;
use std::fmt;

use crate::Span;

/// A lexical mistake and where it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    /// What is wrong.
    pub kind: LexErrorKind,
    /// The code that is wrong.
    pub span: Span,
}

impl LexError {
    pub(super) const fn new(kind: LexErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl fmt::Display for LexError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(formatter)
    }
}

impl Error for LexError {}

/// Every kind of lexical mistake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexErrorKind {
    /// A character that cannot start any token, such as `@`.
    UnexpectedCharacter(char),
    /// A letter outside ASCII where a name was expected, such as `ñ`.
    NonAsciiIdentifier(char),
    /// A `"` without its closing `"` on the same line.
    UnterminatedString,
    /// A `/*` without its closing `*/`.
    UnterminatedBlockComment,
    /// A `\` followed by a character that is not a known escape.
    InvalidEscape(char),
    /// A `\u{…}` that is malformed or not a valid character.
    InvalidUnicodeEscape,
    /// A number without digits where they are required: `0x`, `1e`.
    MissingDigits,
    /// A number followed by a letter or a digit of the wrong base: `10px`, `0b12`.
    InvalidCharacterInNumber(char),
    /// An integer that does not fit in 64 bits.
    IntegerTooLarge,
    /// A decimal number too large to represent.
    FloatTooLarge,
}

impl fmt::Display for LexErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedCharacter(character) => {
                write!(formatter, "unexpected character `{character}`")
            }
            Self::NonAsciiIdentifier(character) => write!(
                formatter,
                "names can only use ASCII letters, digits and `_`, found `{character}`"
            ),
            Self::UnterminatedString => {
                formatter.write_str("string is missing its closing `\"` on this line")
            }
            Self::UnterminatedBlockComment => {
                formatter.write_str("comment is missing its closing `*/`")
            }
            Self::InvalidEscape(character) => write!(
                formatter,
                "unknown escape `\\{character}`; valid escapes are \\n \\t \\r \\\\ \\\" \\0 \\u{{…}}"
            ),
            Self::InvalidUnicodeEscape => formatter.write_str(
                "invalid unicode escape; write it as `\\u{…}` with 1 to 6 hexadecimal digits",
            ),
            Self::MissingDigits => formatter.write_str("number is missing its digits"),
            Self::InvalidCharacterInNumber(character) => {
                write!(formatter, "invalid character `{character}` in number")
            }
            Self::IntegerTooLarge => {
                formatter.write_str("integer is too large; the maximum is 9223372036854775807")
            }
            Self::FloatTooLarge => formatter.write_str("number is too large"),
        }
    }
}
