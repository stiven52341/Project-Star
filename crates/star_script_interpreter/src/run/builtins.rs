//! The methods of basic types and lists, as the generated Rust runs them.

use std::sync::Arc;

use star_script::semantic::BuiltinMethod as Method;
use star_script_runtime::text_length;

use crate::error::RunError;
use crate::value::{Elements, Value, read, write};

/// A method called on something that does not have it: lowering only
/// calls each method on the types analysis allowed.
const WRONG_RECEIVER: RunError = RunError::Internal("method of another type");

/// `receiver.method(argument)`; the argument is already of the type the
/// method takes.
pub(super) fn call(
    method: Method,
    receiver: &Value,
    argument: Option<Value>,
) -> Result<Value, RunError> {
    match (method, receiver) {
        (Method::ToString, _) => Ok(Value::String(receiver.to_text()?)),
        (_, Value::String(text)) => text_method(method, text, argument),
        (_, Value::List(elements)) => list_method(method, elements, argument),
        _ => number_method(method, receiver, argument),
    }
}

fn text_method(method: Method, text: &str, argument: Option<Value>) -> Result<Value, RunError> {
    Ok(match (method, argument) {
        (Method::Length, None) => Value::Int(text_length(text)),
        (Method::IsEmpty, None) => Value::Bool(text.is_empty()),
        (Method::Contains, Some(Value::String(part))) => Value::Bool(text.contains(&*part)),
        (Method::ToUpper, None) => Value::String(Arc::from(text.to_uppercase())),
        (Method::ToLower, None) => Value::String(Arc::from(text.to_lowercase())),
        _ => return Err(WRONG_RECEIVER),
    })
}

fn list_method(
    method: Method,
    elements: &Elements,
    argument: Option<Value>,
) -> Result<Value, RunError> {
    Ok(match (method, argument) {
        (Method::Length, None) => {
            Value::Int(i32::try_from(read(elements)?.len()).unwrap_or(i32::MAX))
        }
        (Method::IsEmpty, None) => Value::Bool(read(elements)?.is_empty()),
        (Method::Contains, Some(value)) => {
            for element in read(elements)?.iter() {
                if element.star_eq(&value)? {
                    return Ok(Value::Bool(true));
                }
            }
            Value::Bool(false)
        }
        (Method::Push, Some(value)) => {
            write(elements)?.push(value);
            Value::Null
        }
        (Method::Pop, None) => write(elements)?.pop().unwrap_or(Value::Null),
        (Method::Clear, None) => {
            write(elements)?.clear();
            Value::Null
        }
        _ => return Err(WRONG_RECEIVER),
    })
}

fn number_method(
    method: Method,
    receiver: &Value,
    argument: Option<Value>,
) -> Result<Value, RunError> {
    Ok(match (method, receiver, argument) {
        (Method::Abs, Value::Short(value), None) => Value::Short(value.wrapping_abs()),
        (Method::Abs, Value::Int(value), None) => Value::Int(value.wrapping_abs()),
        (Method::Abs, Value::BigInt(value), None) => Value::BigInt(value.wrapping_abs()),
        (Method::Abs, Value::Float(value), None) => Value::Float(value.abs()),
        (Method::Abs, Value::Double(value), None) => Value::Double(value.abs()),
        (Method::Min | Method::Max, _, Some(other)) => extreme(method, receiver, &other)?,
        (Method::Floor, Value::Float(value), None) => Value::Float(value.floor()),
        (Method::Floor, Value::Double(value), None) => Value::Double(value.floor()),
        (Method::Ceil, Value::Float(value), None) => Value::Float(value.ceil()),
        (Method::Ceil, Value::Double(value), None) => Value::Double(value.ceil()),
        (Method::Round, Value::Float(value), None) => Value::Float(value.round()),
        (Method::Round, Value::Double(value), None) => Value::Double(value.round()),
        (Method::Sqrt, Value::Float(value), None) => Value::Float(value.sqrt()),
        (Method::Sqrt, Value::Double(value), None) => Value::Double(value.sqrt()),
        _ => return Err(WRONG_RECEIVER),
    })
}

/// `min` or `max` of two numbers of one type, as Rust's methods give it
/// (between decimals, a NaN loses to a number).
fn extreme(method: Method, left: &Value, right: &Value) -> Result<Value, RunError> {
    let smaller = method == Method::Min;
    Ok(match (left, right) {
        (Value::Short(a), Value::Short(b)) => {
            Value::Short(if smaller { *a.min(b) } else { *a.max(b) })
        }
        (Value::Int(a), Value::Int(b)) => Value::Int(if smaller { *a.min(b) } else { *a.max(b) }),
        (Value::BigInt(a), Value::BigInt(b)) => {
            Value::BigInt(if smaller { *a.min(b) } else { *a.max(b) })
        }
        (Value::Float(a), Value::Float(b)) => {
            Value::Float(if smaller { a.min(*b) } else { a.max(*b) })
        }
        (Value::Double(a), Value::Double(b)) => {
            Value::Double(if smaller { a.min(*b) } else { a.max(*b) })
        }
        _ => return Err(WRONG_RECEIVER),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(method: Method, receiver: &Value, argument: Option<Value>) -> Value {
        call(method, receiver, argument).expect("a method of the receiver")
    }

    #[test]
    fn numbers_have_their_methods() {
        assert_eq!(
            run(Method::Abs, &Value::Short(i16::MIN), None),
            Value::Short(i16::MIN)
        );
        assert_eq!(
            run(Method::Max, &Value::Int(2), Some(Value::Int(5))),
            Value::Int(5)
        );
        assert_eq!(
            run(
                Method::Min,
                &Value::Double(f64::NAN),
                Some(Value::Double(1.0))
            ),
            Value::Double(1.0)
        );
        assert_eq!(
            run(Method::Round, &Value::Float(-2.5), None),
            Value::Float(-3.0)
        );
    }

    #[test]
    fn texts_count_characters() {
        assert_eq!(
            run(Method::Length, &Value::text("año"), None),
            Value::Int(3)
        );
        assert_eq!(
            run(Method::ToUpper, &Value::text("año"), None),
            Value::text("AÑO")
        );
        assert_eq!(
            run(
                Method::Contains,
                &Value::text("Star"),
                Some(Value::text("ta"))
            ),
            Value::Bool(true)
        );
    }

    #[test]
    fn lists_change_in_place() {
        let list = Value::list(vec![Value::Int(1)]);
        run(Method::Push, &list, Some(Value::Int(2)));
        assert_eq!(run(Method::Length, &list, None), Value::Int(2));
        assert_eq!(
            run(Method::Contains, &list, Some(Value::Int(2))),
            Value::Bool(true)
        );
        assert_eq!(run(Method::Pop, &list, None), Value::Int(2));
        run(Method::Clear, &list, None);
        assert_eq!(run(Method::Pop, &list, None), Value::Null);
        assert_eq!(run(Method::IsEmpty, &list, None), Value::Bool(true));
    }
}
