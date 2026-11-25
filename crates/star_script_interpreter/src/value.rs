//! The values a running program works with.

use std::cell::{Ref, RefCell, RefMut};
use std::fmt;
use std::rc::Rc;

use crate::error::RunError;
use crate::tree::ClassRef;

/// The elements of a list, shared by every value that refers to it.
pub(crate) type Elements = Rc<RefCell<Vec<Value>>>;

/// A value of StarScript while the interpreter runs.
///
/// A value of a nullable type (`int?`) is the value itself or
/// [`Value::Null`]: nothing wraps it.
#[derive(Debug, Clone)]
#[cfg_attr(test, derive(PartialEq))]
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
    /// `[T]`: shared, as in the compiled game, so after `var a = b` a
    /// `push` through `a` is seen through `b`.
    List(Elements),
    /// An object: shared, and equal only to itself.
    Object(Rc<Object>),
}

/// An object of a class of the program.
pub(crate) struct Object {
    /// The class it was created as.
    pub(crate) class: ClassRef,
    /// The fields of every class of its hierarchy, the root's first.
    fields: RefCell<Vec<Value>>,
}

impl Object {
    pub(crate) const fn new(class: ClassRef, fields: Vec<Value>) -> Self {
        Self {
            class,
            fields: RefCell::new(fields),
        }
    }

    /// A copy of the field at `position`. Like the compiled `Field`, it
    /// never keeps the fields borrowed after it returns.
    pub(crate) fn field(&self, position: usize) -> Result<Value, RunError> {
        self.fields
            .try_borrow()
            .map_err(|_| OVERLAPPING_FIELDS)?
            .get(position)
            .cloned()
            .ok_or(MISSING_FIELD)
    }

    /// Replaces the field at `position`.
    pub(crate) fn set_field(&self, position: usize, value: Value) -> Result<(), RunError> {
        let mut fields = self
            .fields
            .try_borrow_mut()
            .map_err(|_| OVERLAPPING_FIELDS)?;
        let field = fields.get_mut(position).ok_or(MISSING_FIELD)?;
        *field = value;
        Ok(())
    }
}

/// Objects can refer to each other, so they show only their class: showing
/// their fields could go round forever.
impl fmt::Debug for Object {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Object")
            .field("class", &self.class)
            .finish_non_exhaustive()
    }
}

/// Objects are equal only to themselves.
impl PartialEq for Object {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

/// The fields of an object borrowed while another access runs: the
/// interpreter never runs StarScript code while it holds them.
const OVERLAPPING_FIELDS: RunError = RunError::Internal("overlapping field access");
/// A field position outside the object: lowering gives every field of the
/// hierarchy its place.
const MISSING_FIELD: RunError = RunError::Internal("field outside its object");

impl Value {
    /// A text value.
    pub(crate) fn text(text: &str) -> Self {
        Self::String(Rc::from(text))
    }

    /// A new list with these elements.
    pub(crate) fn list(elements: Vec<Self>) -> Self {
        Self::List(Rc::new(RefCell::new(elements)))
    }

    /// `self == other` in StarScript: values by content, lists by their
    /// elements in order. Lowering converts both sides to one type first,
    /// so values of different types only meet `null`.
    #[allow(clippy::float_cmp)] // StarScript's `==` is exact, like Rust's.
    pub(crate) fn star_eq(&self, other: &Self) -> Result<bool, RunError> {
        Ok(match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Short(left), Self::Short(right)) => left == right,
            (Self::Int(left), Self::Int(right)) => left == right,
            (Self::BigInt(left), Self::BigInt(right)) => left == right,
            (Self::Float(left), Self::Float(right)) => left == right,
            (Self::Double(left), Self::Double(right)) => left == right,
            (Self::Bool(left), Self::Bool(right)) => left == right,
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Object(left), Self::Object(right)) => Rc::ptr_eq(left, right),
            (Self::List(left), Self::List(right)) => {
                if Rc::ptr_eq(left, right) {
                    return Ok(true);
                }
                let (left, right) = (read(left)?, read(right)?);
                if left.len() != right.len() {
                    return Ok(false);
                }
                for (left, right) in left.iter().zip(right.iter()) {
                    if !left.star_eq(right)? {
                        return Ok(false);
                    }
                }
                true
            }
            _ => false,
        })
    }

    /// `self.to_string()`, written as the generated Rust writes it.
    pub(crate) fn to_text(&self) -> Result<Rc<str>, RunError> {
        Ok(match self {
            Self::String(text) => Rc::clone(text),
            Self::Short(value) => Rc::from(value.to_string()),
            Self::Int(value) => Rc::from(value.to_string()),
            Self::BigInt(value) => Rc::from(value.to_string()),
            Self::Float(value) => Rc::from(star_script_runtime::float_to_string(value)),
            Self::Double(value) => Rc::from(star_script_runtime::float_to_string(value)),
            Self::Bool(value) => Rc::from(value.to_string()),
            Self::Null | Self::List(_) | Self::Object(_) => {
                return Err(RunError::Internal("`to_string` of a value without text"));
            }
        })
    }
}

/// The elements of a list, to read them. The interpreter never runs
/// StarScript code while it holds them, so this cannot fail unless the
/// interpreter itself is wrong.
pub(crate) fn read(elements: &Elements) -> Result<Ref<'_, Vec<Value>>, RunError> {
    elements
        .try_borrow()
        .map_err(|_| RunError::Internal("overlapping list access"))
}

/// The elements of a list, to change them (see [`read`]).
pub(crate) fn write(elements: &Elements) -> Result<RefMut<'_, Vec<Value>>, RunError> {
    elements
        .try_borrow_mut()
        .map_err(|_| RunError::Internal("overlapping list access"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn equal(left: &Value, right: &Value) -> bool {
        left.star_eq(right).expect("comparable values")
    }

    #[test]
    fn values_compare_by_content() {
        assert!(equal(&Value::text("a"), &Value::text("a")));
        assert!(!equal(&Value::Int(1), &Value::Int(2)));
        assert!(!equal(&Value::Double(f64::NAN), &Value::Double(f64::NAN)));
        assert!(equal(&Value::Null, &Value::Null));
        assert!(!equal(&Value::Int(0), &Value::Null));
    }

    #[test]
    fn lists_compare_by_elements() {
        let list = Value::list(vec![Value::Int(1), Value::Null]);
        assert!(equal(&list, &list.clone()));
        assert!(equal(&list, &Value::list(vec![Value::Int(1), Value::Null])));
        assert!(!equal(&list, &Value::list(vec![Value::Int(1)])));
    }

    #[test]
    fn numbers_read_as_the_compiled_ones() {
        let text = |value: Value| value.to_text().expect("a value with text");
        assert_eq!(&*text(Value::Float(2.0)), "2.0");
        assert_eq!(&*text(Value::Double(0.1 + 0.2)), "0.30000000000000004");
        assert_eq!(&*text(Value::Short(-3)), "-3");
        assert_eq!(&*text(Value::Bool(true)), "true");
    }
}
