//! Text literals and their escapes.

use super::cursor::Cursor;
use super::{LexError, LexErrorKind};
use crate::Span;

const QUOTE: char = '"';
const ESCAPE: char = '\\';
const MAX_UNICODE_ESCAPE_DIGITS: usize = 6;
const HEXADECIMAL_RADIX: u32 = 16;

/// Reads a string starting at its opening quote and decodes its escapes.
///
/// After a bad escape the rest of the string is still read, so lexing can
/// continue after it; only the first bad escape is reported.
pub(super) fn lex_string(cursor: &mut Cursor<'_>) -> Result<String, LexError> {
    let start = cursor.position();
    cursor.advance();
    let mut value = String::new();
    let mut first_error = None;
    loop {
        let character_start = cursor.position();
        match cursor.peek() {
            None | Some('\n') => {
                let span = Span::new(start, cursor.position());
                return Err(LexError::new(LexErrorKind::UnterminatedString, span));
            }
            Some(QUOTE) => {
                cursor.advance();
                break;
            }
            Some(ESCAPE) => {
                cursor.advance();
                match lex_escape(cursor) {
                    Ok(character) => value.push(character),
                    Err(kind) => {
                        let span = Span::new(character_start, cursor.position());
                        first_error.get_or_insert(LexError::new(kind, span));
                    }
                }
            }
            Some(character) => {
                cursor.advance();
                value.push(character);
            }
        }
    }
    first_error.map_or(Ok(value), Err)
}

fn lex_escape(cursor: &mut Cursor<'_>) -> Result<char, LexErrorKind> {
    let escaped = match cursor.peek() {
        // The string loop reports the missing closing quote.
        None | Some('\n') => return Err(LexErrorKind::UnterminatedString),
        Some(character) => character,
    };
    cursor.advance();
    match escaped {
        'n' => Ok('\n'),
        't' => Ok('\t'),
        'r' => Ok('\r'),
        '0' => Ok('\0'),
        ESCAPE | QUOTE => Ok(escaped),
        'u' => lex_unicode_escape(cursor),
        other => Err(LexErrorKind::InvalidEscape(other)),
    }
}

/// Reads the `{…}` part of `\u{…}`.
fn lex_unicode_escape(cursor: &mut Cursor<'_>) -> Result<char, LexErrorKind> {
    if !cursor.eat('{') {
        return Err(LexErrorKind::InvalidUnicodeEscape);
    }
    let digits_start = cursor.position();
    cursor.eat_while(|character| character.is_ascii_hexdigit());
    let digits = cursor.text_since(digits_start);
    if !cursor.eat('}') || digits.is_empty() || digits.len() > MAX_UNICODE_ESCAPE_DIGITS {
        return Err(LexErrorKind::InvalidUnicodeEscape);
    }
    u32::from_str_radix(digits, HEXADECIMAL_RADIX)
        .ok()
        .and_then(char::from_u32)
        .ok_or(LexErrorKind::InvalidUnicodeEscape)
}
