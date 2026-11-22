//! Integer and decimal literals.

use super::cursor::Cursor;
use super::{LexErrorKind, is_identifier_continue};
use crate::TokenKind;

const DIGIT_SEPARATOR: char = '_';
const HEXADECIMAL_RADIX: u32 = 16;
const BINARY_RADIX: u32 = 2;
const DECIMAL_RADIX: u32 = 10;

/// Reads a number starting at an ASCII digit.
pub(super) fn lex_number(cursor: &mut Cursor<'_>) -> Result<TokenKind, LexErrorKind> {
    let result = match radix_prefix(cursor) {
        Some(radix) => lex_prefixed_integer(cursor, radix),
        None => lex_decimal(cursor),
    }
    .and_then(|kind| reject_suffix(cursor).map(|()| kind));
    if result.is_err() {
        // Skip the rest of the malformed number so it does not produce more errors.
        cursor.eat_while(is_identifier_continue);
    }
    result
}

fn radix_prefix(cursor: &Cursor<'_>) -> Option<u32> {
    if cursor.peek() != Some('0') {
        return None;
    }
    match cursor.peek_second() {
        Some('x' | 'X') => Some(HEXADECIMAL_RADIX),
        Some('b' | 'B') => Some(BINARY_RADIX),
        _ => None,
    }
}

fn lex_prefixed_integer(cursor: &mut Cursor<'_>, radix: u32) -> Result<TokenKind, LexErrorKind> {
    cursor.advance();
    cursor.advance();
    let digits = read_digits(cursor, radix);
    if digits.is_empty() {
        return Err(LexErrorKind::MissingDigits);
    }
    i64::from_str_radix(&digits, radix)
        .map(TokenKind::Integer)
        .map_err(|_| LexErrorKind::IntegerTooLarge)
}

fn lex_decimal(cursor: &mut Cursor<'_>) -> Result<TokenKind, LexErrorKind> {
    let mut text = read_digits(cursor, DECIMAL_RADIX);
    let has_fraction = starts_fraction(cursor);
    if has_fraction {
        cursor.advance();
        text.push('.');
        text.push_str(&read_digits(cursor, DECIMAL_RADIX));
    }
    let has_exponent = matches!(cursor.peek(), Some('e' | 'E'));
    if has_exponent {
        text.push_str(&read_exponent(cursor)?);
    }
    if has_fraction || has_exponent {
        parse_float(&text)
    } else {
        text.parse()
            .map(TokenKind::Integer)
            .map_err(|_| LexErrorKind::IntegerTooLarge)
    }
}

/// A `.` followed by a digit; otherwise the dot belongs to `..` or `a.b`.
fn starts_fraction(cursor: &Cursor<'_>) -> bool {
    cursor.peek() == Some('.') && cursor.peek_second().is_some_and(|c| c.is_ascii_digit())
}

fn read_exponent(cursor: &mut Cursor<'_>) -> Result<String, LexErrorKind> {
    cursor.advance();
    let mut exponent = String::from("e");
    if let Some(sign @ ('+' | '-')) = cursor.peek() {
        cursor.advance();
        exponent.push(sign);
    }
    let digits = read_digits(cursor, DECIMAL_RADIX);
    if digits.is_empty() {
        return Err(LexErrorKind::MissingDigits);
    }
    exponent.push_str(&digits);
    Ok(exponent)
}

fn parse_float(text: &str) -> Result<TokenKind, LexErrorKind> {
    match text.parse::<f64>() {
        Ok(value) if value.is_finite() => Ok(TokenKind::Float(value)),
        _ => Err(LexErrorKind::FloatTooLarge),
    }
}

/// Reads digits of `radix` and separators, returning the digits only.
fn read_digits(cursor: &mut Cursor<'_>, radix: u32) -> String {
    let mut digits = String::new();
    while let Some(character) = cursor.peek() {
        if character.is_digit(radix) {
            digits.push(character);
        } else if character != DIGIT_SEPARATOR {
            break;
        }
        cursor.advance();
    }
    digits
}

fn reject_suffix(cursor: &Cursor<'_>) -> Result<(), LexErrorKind> {
    match cursor.peek() {
        Some(character) if is_identifier_continue(character) => {
            Err(LexErrorKind::InvalidCharacterInNumber(character))
        }
        _ => Ok(()),
    }
}
