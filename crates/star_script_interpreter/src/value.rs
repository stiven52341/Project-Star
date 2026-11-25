//! The values a running program works with.

use std::rc::Rc;

/// A value of StarScript while the interpreter runs.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Value {
    /// `null`; also what a function without a result gives, and what a
    /// local holds before its declaration runs.
    Null,
    /// `short`.
    Short(i16),
    /// `int`.
    Int(i32),
    /// `bigint`.
    BigInt(i64),
    /// `float`.
    Float(f32),
    /// `double`.
    Double(f64),
    /// `bool`.
    Bool(bool),
    /// `string`. Texts never change, so copies share them.
    String(Rc<str>),
}

impl Value {
    /// A text value.
    pub(crate) fn text(text: &str) -> Self {
        Self::String(Rc::from(text))
    }

    /// `self == other` in StarScript: values by content. Lowering converts
    /// both sides to one type first, so values of different types are
    /// never compared.
    #[allow(clippy::float_cmp)] // StarScript's `==` is exact, like Rust's.
    pub(crate) fn star_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Short(left), Self::Short(right)) => left == right,
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::BigInt(left), Self::BigInt(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => left == right,
            (Self::Double(left), Self::Double(right)) => left == right,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::String(left), Self::String(right)) => left == right,
            _ => false,
        }
    }

    /// `self.to_string()`, written as the generated Rust writes it.
    pub(crate) fn to_text(&self) -> Rc<str> {
        match self {
            Self::String(text) => Rc::clone(text),
            Self::Null => Rc::from("null"),
            Self::Short(value) => Rc::from(value.to_string()),
            Self::Int(value) => Rc::from(value.to_string()),
            Self::BigInt(value) => Rc::from(value.to_string()),
            Self::Float(value) => Rc::from(star_script_runtime::float_to_string(value)),
            Self::Double(value) => Rc::from(star_script_runtime::float_to_string(value)),
            Self::Bool(value) => Rc::from(value.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_compare_by_content() {
        assert!(Value::text("a").star_eq(&Value::text("a")));
        assert!(!Value::Int(1).star_eq(&Value::Int(2)));
        assert!(!Value::Double(f64::NAN).star_eq(&Value::Double(f64::NAN)));
        assert!(Value::Null.star_eq(&Value::Null));
    }

    #[test]
    fn numbers_read_as_the_compiled_ones() {
        assert_eq!(&*Value::Float(2.0).to_text(), "2.0");
        assert_eq!(&*Value::Double(0.1 + 0.2).to_text(), "0.30000000000000004");
        assert_eq!(&*Value::Short(-3).to_text(), "-3");
        assert_eq!(&*Value::Bool(true).to_text(), "true");
    }
}
