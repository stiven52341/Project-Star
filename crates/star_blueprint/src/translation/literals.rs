//! Constant values written as StarScript literals.

use crate::model::Literal;

/// `i64::MIN` cannot be written as `-` and a number: the number alone would
/// be too large. This expression has its value.
const SMALLEST_INTEGER: &str = "(-9223372036854775807 - 1)";

/// `value` as StarScript code, or `None` if StarScript cannot write it (a
/// number that is not finite).
pub(super) fn literal_code(value: &Literal) -> Option<String> {
    match value {
        Literal::Bool(value) => Some(value.to_string()),
        Literal::Int(i64::MIN) => Some(SMALLEST_INTEGER.to_owned()),
        Literal::Int(value) => Some(value.to_string()),
        Literal::Float(value) => value.is_finite().then(|| float_code(*value)),
        Literal::String(text) => Some(string_code(text)),
    }
}

/// Rust's shortest exact form (`2.0`, `0.1`, `1e20`), which StarScript reads
/// as a decimal: it always has a `.` or an exponent.
fn float_code(value: f64) -> String {
    format!("{value:?}")
}

/// `text` between quotes, with the characters StarScript cannot hold
/// written as escapes.
fn string_code(text: &str) -> String {
    let mut code = String::with_capacity(text.len() + 2);
    code.push('"');
    for character in text.chars() {
        match character {
            '"' => code.push_str("\\\""),
            '\\' => code.push_str("\\\\"),
            '\n' => code.push_str("\\n"),
            '\t' => code.push_str("\\t"),
            '\r' => code.push_str("\\r"),
            '\0' => code.push_str("\\0"),
            // `\u{7}`, the form StarScript reads.
            other if other.is_control() => code.extend(other.escape_unicode()),
            other => code.push(other),
        }
    }
    code.push('"');
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    fn code(value: &Literal) -> String {
        literal_code(value).expect("writable")
    }

    #[test]
    fn numbers_keep_their_kind() {
        assert_eq!(code(&Literal::Int(3)), "3");
        assert_eq!(code(&Literal::Int(-3)), "-3");
        assert_eq!(code(&Literal::Float(3.0)), "3.0");
        assert_eq!(code(&Literal::Float(0.1)), "0.1");
        assert_eq!(code(&Literal::Float(1e20)), "1e20");
    }

    #[test]
    fn the_smallest_integer_is_written_as_an_expression() {
        assert_eq!(code(&Literal::Int(i64::MIN)), SMALLEST_INTEGER);
    }

    #[test]
    fn numbers_that_are_not_finite_cannot_be_written() {
        assert_eq!(literal_code(&Literal::Float(f64::NAN)), None);
        assert_eq!(literal_code(&Literal::Float(f64::INFINITY)), None);
    }

    #[test]
    fn texts_are_escaped() {
        assert_eq!(
            code(&Literal::String("say \"hi\"\\\n\u{7}ñ".to_owned())),
            "\"say \\\"hi\\\"\\\\\\n\\u{7}ñ\""
        );
    }

    #[test]
    fn star_script_reads_every_literal_written() {
        let values = [
            Literal::Bool(true),
            Literal::Int(-3),
            Literal::Int(i64::MIN),
            Literal::Float(1e20),
            Literal::Float(-0.1),
            Literal::String("say \"hi\"\\\n\t\r\0\u{7}ñ".to_owned()),
        ];
        for value in &values {
            let source = format!("class A {{\n    var value = {}\n}}\n", code(value));

            let checked = star_script::check_source(&source);

            assert!(checked.is_ok(), "{source}\n{:?}", checked.err());
        }
    }
}
