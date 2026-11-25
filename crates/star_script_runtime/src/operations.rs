//! Operations whose StarScript meaning differs from plain Rust operators.

use crate::RuntimeError;

/// An integer type of StarScript: `short` (`i16`), `int` (`i32`) or
/// `bigint` (`i64`).
pub trait Integer: Copy + PartialEq {
    /// Zero, which no integer can be divided by.
    const ZERO: Self;

    /// `self / divisor`, wrapping around on overflow.
    #[must_use]
    fn wrapping_div(self, divisor: Self) -> Self;

    /// `self % divisor`, wrapping around on overflow.
    #[must_use]
    fn wrapping_rem(self, divisor: Self) -> Self;
}

macro_rules! integer {
    ($($rust:ty),*) => {
        $(
            impl Integer for $rust {
                const ZERO: Self = 0;

                fn wrapping_div(self, divisor: Self) -> Self {
                    <$rust>::wrapping_div(self, divisor)
                }

                fn wrapping_rem(self, divisor: Self) -> Self {
                    <$rust>::wrapping_rem(self, divisor)
                }
            }
        )*
    };
}

integer!(i16, i32, i64);

/// `a / b` between integers. Like the other integer operations, it wraps
/// around on overflow (`int.min / -1`), so debug and release builds of a
/// game always give the same results.
#[must_use]
#[track_caller]
pub fn int_divide<I: Integer>(dividend: I, divisor: I) -> I {
    try_int_divide(dividend, divisor).unwrap_or_else(|error| error.stop())
}

/// `a % b` between integers; the result has the sign of `a`.
#[must_use]
#[track_caller]
pub fn int_remainder<I: Integer>(dividend: I, divisor: I) -> I {
    try_int_remainder(dividend, divisor).unwrap_or_else(|error| error.stop())
}

/// [`int_divide`] that returns its error instead of stopping the game.
///
/// # Errors
///
/// When `divisor` is zero.
pub fn try_int_divide<I: Integer>(dividend: I, divisor: I) -> Result<I, RuntimeError> {
    if divisor == I::ZERO {
        return Err(RuntimeError::new("integer division by zero"));
    }
    Ok(dividend.wrapping_div(divisor))
}

/// [`int_remainder`] that returns its error instead of stopping the game.
///
/// # Errors
///
/// When `divisor` is zero.
pub fn try_int_remainder<I: Integer>(dividend: I, divisor: I) -> Result<I, RuntimeError> {
    if divisor == I::ZERO {
        return Err(RuntimeError::new("integer remainder by zero"));
    }
    Ok(dividend.wrapping_rem(divisor))
}

/// `text.length()`: how many characters it has (at most the largest `int`).
#[must_use]
pub fn text_length(text: &str) -> i32 {
    i32::try_from(text.chars().count()).unwrap_or(i32::MAX)
}

/// `a + b` between texts.
#[must_use]
pub fn concat(mut left: String, right: &str) -> String {
    left.push_str(right);
    left
}

/// `x.to_string()` for a `float` or a `double`: always shows that it is a
/// decimal (`2.0`, not `2`), and the shortest text that reads back the
/// same value.
#[must_use]
pub fn float_to_string(value: impl std::fmt::Debug) -> String {
    format!("{value:?}")
}

/// `Debug.print(message)`: writes a line to the standard output.
pub fn print(message: &str) {
    println!("{message}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integer_division_wraps_around() {
        assert_eq!(int_divide(7, 2), 3);
        assert_eq!(int_divide(-7, 2), -3);
        assert_eq!(int_divide(i64::MIN, -1), i64::MIN);
        assert_eq!(int_divide(i16::MIN, -1_i16), i16::MIN);
        assert_eq!(int_divide(i32::MIN, -1_i32), i32::MIN);
        assert_eq!(int_remainder(-7, 2), -1);
        assert_eq!(int_remainder(i64::MIN, -1), 0);
        assert_eq!(int_remainder(i16::MIN, -1_i16), 0);
    }

    #[test]
    fn checked_division_returns_its_error() {
        assert_eq!(try_int_divide(7, 2), Ok(3));
        let error = try_int_remainder(7_i16, 0).map_err(|error| error.message().to_owned());
        assert_eq!(error, Err("integer remainder by zero".to_owned()));
    }

    #[test]
    #[should_panic(expected = "integer division by zero")]
    fn division_by_zero_is_a_fault() {
        let _ = int_divide(1, 0);
    }

    #[test]
    #[should_panic(expected = "integer remainder by zero")]
    fn remainder_by_zero_is_a_fault() {
        let _ = int_remainder(1, 0);
    }

    #[test]
    fn texts_count_characters() {
        assert_eq!(text_length("año"), 3);
    }

    #[test]
    fn texts_are_joined() {
        assert_eq!(concat("Vidas: ".to_owned(), "3"), "Vidas: 3");
    }

    #[test]
    fn floats_always_look_like_floats() {
        assert_eq!(float_to_string(2.0_f32), "2.0");
        assert_eq!(float_to_string(0.1_f32), "0.1");
        assert_eq!(float_to_string(-1.5_f32), "-1.5");
        assert_eq!(float_to_string(0.1_f64), "0.1");
    }
}
