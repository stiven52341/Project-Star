//! Operations whose StarScript meaning differs from plain Rust operators.

use crate::fault;

/// `a / b` between integers. Like the other integer operations, it wraps
/// around on overflow (`int.min / -1`), so debug and release builds of a
/// game always give the same results.
#[must_use]
#[track_caller]
pub fn int_divide(dividend: i64, divisor: i64) -> i64 {
    if divisor == 0 {
        fault("integer division by zero");
    }
    dividend.wrapping_div(divisor)
}

/// `a % b` between integers; the result has the sign of `a`.
#[must_use]
#[track_caller]
pub fn int_remainder(dividend: i64, divisor: i64) -> i64 {
    if divisor == 0 {
        fault("integer remainder by zero");
    }
    dividend.wrapping_rem(divisor)
}

/// `a + b` between texts.
#[must_use]
pub fn concat(mut left: String, right: &str) -> String {
    left.push_str(right);
    left
}

/// `x.to_string()` for a `float`: always shows that it is a `float`
/// (`2.0`, not `2`), and the shortest text that reads back the same value.
#[must_use]
pub fn float_to_string(value: f32) -> String {
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
        assert_eq!(int_remainder(-7, 2), -1);
        assert_eq!(int_remainder(i64::MIN, -1), 0);
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
    fn texts_are_joined() {
        assert_eq!(concat("Vidas: ".to_owned(), "3"), "Vidas: 3");
    }

    #[test]
    fn floats_always_look_like_floats() {
        assert_eq!(float_to_string(2.0), "2.0");
        assert_eq!(float_to_string(0.1), "0.1");
        assert_eq!(float_to_string(-1.5), "-1.5");
    }
}
