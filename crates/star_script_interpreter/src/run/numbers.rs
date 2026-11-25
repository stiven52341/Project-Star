//! Operations on numbers, with the meaning the generated Rust gives them:
//! integers wrap around, and dividing one by zero is a runtime error.

use std::cmp::Ordering;
use std::sync::Arc;

use star_script_runtime::{try_int_divide, try_int_remainder};

use crate::error::RunError;
use crate::tree::{Arithmetic, Comparison, NumberType};
use crate::value::Value;

/// Operands of different types: lowering converts them to one first.
const MIXED_OPERANDS: RunError = RunError::Internal("operands of different types");

/// `left op right` between two integers of one Rust type: the operations
/// of the generated Rust, which wrap around.
macro_rules! integer_arithmetic {
    ($operator:expr, $left:expr, $right:expr) => {
        match $operator {
            Arithmetic::Add => $left.wrapping_add($right),
            Arithmetic::Subtract => $left.wrapping_sub($right),
            Arithmetic::Multiply => $left.wrapping_mul($right),
            Arithmetic::Divide => try_int_divide($left, $right)?,
            Arithmetic::Remainder => try_int_remainder($left, $right)?,
        }
    };
}

/// `left op right` between two decimals of one Rust type.
macro_rules! float_arithmetic {
    ($operator:expr, $left:expr, $right:expr) => {
        match $operator {
            Arithmetic::Add => $left + $right,
            Arithmetic::Subtract => $left - $right,
            Arithmetic::Multiply => $left * $right,
            Arithmetic::Divide => $left / $right,
            Arithmetic::Remainder => $left % $right,
        }
    };
}

/// `$value as` the Rust type of `$to`.
macro_rules! convert_number {
    ($value:expr, $to:expr) => {{
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_precision_loss,
            clippy::cast_lossless,
            clippy::cast_possible_wrap,
            clippy::unnecessary_cast
        )]
        let converted = match $to {
            NumberType::Short => Value::Short($value as i16),
            NumberType::Int => Value::Int($value as i32),
            NumberType::BigInt => Value::BigInt($value as i64),
            NumberType::Float => Value::Float($value as f32),
            NumberType::Double => Value::Double($value as f64),
        };
        converted
    }};
}

/// `left operator right`, both of one number type, or `+` of two texts.
pub(super) fn arithmetic(
    operator: Arithmetic,
    left: &Value,
    right: &Value,
) -> Result<Value, RunError> {
    Ok(match (left, right) {
        (Value::Short(left), Value::Short(right)) => {
            Value::Short(integer_arithmetic!(operator, *left, *right))
        }
        (Value::Int(left), Value::Int(right)) => {
            Value::Int(integer_arithmetic!(operator, *left, *right))
        }
        (Value::BigInt(left), Value::BigInt(right)) => {
            Value::BigInt(integer_arithmetic!(operator, *left, *right))
        }
        (Value::Float(left), Value::Float(right)) => {
            Value::Float(float_arithmetic!(operator, *left, *right))
        }
        (Value::Double(left), Value::Double(right)) => {
            Value::Double(float_arithmetic!(operator, *left, *right))
        }
        (Value::String(left), Value::String(right)) if operator == Arithmetic::Add => {
            Value::String(Arc::from([&**left, &**right].concat()))
        }
        _ => return Err(MIXED_OPERANDS),
    })
}

/// `left operator right` between values of one type. A decimal that is not
/// a number (NaN) is neither smaller, equal nor larger than anything.
pub(super) fn compare(operator: Comparison, left: &Value, right: &Value) -> Result<bool, RunError> {
    let ordering = match (left, right) {
        (Value::Short(left), Value::Short(right)) => left.partial_cmp(right),
        (Value::Int(left), Value::Int(right)) => left.partial_cmp(right),
        (Value::BigInt(left), Value::BigInt(right)) => left.partial_cmp(right),
        (Value::Float(left), Value::Float(right)) => left.partial_cmp(right),
        (Value::Double(left), Value::Double(right)) => left.partial_cmp(right),
        (Value::String(left), Value::String(right)) => left.partial_cmp(right),
        (Value::Bool(left), Value::Bool(right)) => left.partial_cmp(right),
        _ => return Err(MIXED_OPERANDS),
    };
    Ok(match operator {
        Comparison::Less => ordering == Some(Ordering::Less),
        Comparison::LessEqual => matches!(ordering, Some(Ordering::Less | Ordering::Equal)),
        Comparison::Greater => ordering == Some(Ordering::Greater),
        Comparison::GreaterEqual => matches!(ordering, Some(Ordering::Greater | Ordering::Equal)),
    })
}

/// `-value`; integers wrap around (`-short.min` is `short.min`).
pub(super) fn negate(value: &Value) -> Result<Value, RunError> {
    Ok(match value {
        Value::Short(value) => Value::Short(value.wrapping_neg()),
        Value::Int(value) => Value::Int(value.wrapping_neg()),
        Value::BigInt(value) => Value::BigInt(value.wrapping_neg()),
        Value::Float(value) => Value::Float(-value),
        Value::Double(value) => Value::Double(-value),
        _ => {
            return Err(RunError::Internal(
                "negation of something that is not a number",
            ));
        }
    })
}

/// `value` as the number type `to`, as Rust's `as` converts it: integers
/// keep their lowest bits, decimals lose their fraction and saturate;
/// `null` stays `null`.
pub(super) fn convert(value: &Value, to: NumberType) -> Result<Value, RunError> {
    Ok(match value {
        // A nullable number read as a wider nullable one.
        Value::Null => Value::Null,
        Value::Short(value) => convert_number!(*value, to),
        Value::Int(value) => convert_number!(*value, to),
        Value::BigInt(value) => convert_number!(*value, to),
        Value::Float(value) => convert_number!(*value, to),
        Value::Double(value) => convert_number!(*value, to),
        _ => {
            return Err(RunError::Internal(
                "conversion of something that is not a number",
            ));
        }
    })
}

/// An integer of any type, widened to read it as a range's end.
pub(super) fn integer(value: &Value) -> Result<i64, RunError> {
    match value {
        Value::Short(value) => Ok(i64::from(*value)),
        Value::Int(value) => Ok(i64::from(*value)),
        Value::BigInt(value) => Ok(*value),
        _ => Err(RunError::Internal(
            "range of something that is not an integer",
        )),
    }
}

/// `value`, a number between two integers of `like`'s type, as that type.
pub(super) fn same_integer(like: &Value, value: i64) -> Result<Value, RunError> {
    let out_of_range = |_| RunError::Internal("range value out of its type");
    match like {
        Value::Short(_) => i16::try_from(value).map(Value::Short).map_err(out_of_range),
        Value::Int(_) => i32::try_from(value).map(Value::Int).map_err(out_of_range),
        Value::BigInt(_) => Ok(Value::BigInt(value)),
        _ => Err(RunError::Internal(
            "range of something that is not an integer",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_wrap_around() {
        let sum = arithmetic(Arithmetic::Add, &Value::Short(i16::MAX), &Value::Short(1));
        assert_eq!(sum.ok(), Some(Value::Short(i16::MIN)));
        assert_eq!(
            negate(&Value::Int(i32::MIN)).ok(),
            Some(Value::Int(i32::MIN))
        );
    }

    #[test]
    fn dividing_an_integer_by_zero_is_a_fault() {
        let result = arithmetic(Arithmetic::Divide, &Value::Int(1), &Value::Int(0));
        let Err(RunError::Fault(error)) = result else {
            panic!("expected a fault, got {result:?}");
        };
        assert_eq!(error.message(), "integer division by zero");
    }

    #[test]
    fn conversions_follow_rust() {
        assert_eq!(
            convert(&Value::Int(70000), NumberType::Short).ok(),
            Some(Value::Short(4464))
        );
        assert_eq!(
            convert(&Value::Double(1e10), NumberType::Int).ok(),
            Some(Value::Int(i32::MAX))
        );
        assert_eq!(
            convert(&Value::Float(-3.99), NumberType::BigInt).ok(),
            Some(Value::BigInt(-3))
        );
    }

    #[test]
    fn nan_is_never_ordered() {
        let nan = Value::Double(f64::NAN);
        for operator in [
            Comparison::Less,
            Comparison::LessEqual,
            Comparison::GreaterEqual,
        ] {
            assert_eq!(compare(operator, &nan, &nan).ok(), Some(false));
        }
    }

    #[test]
    fn mixed_operands_are_an_internal_error() {
        let result = arithmetic(Arithmetic::Add, &Value::Int(1), &Value::Short(1));
        assert!(matches!(result, Err(RunError::Internal(_))));
    }
}
