//! The values a running program works with.

use std::any::Any;
use std::cell::{Cell, Ref, RefCell, RefMut};
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

use star_script_runtime::color::ColorFields;
use star_script_runtime::game::GameFields;

use crate::error::RunError;
use crate::host::{BodyObject, Interpreted, NodeParts, SoundObject};
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
    String(Arc<str>),
    /// `[T]`: shared, as in the compiled game, so after `var a = b` a
    /// `push` through `a` is seen through `b`.
    List(Elements),
    /// An object of a class of the program: shared, and equal only to
    /// itself.
    Object(Rc<Object>),
    /// A `Color`, with the runtime's fields of colors.
    Color(Rc<Object<ColorFields>>),
    /// A `Node`, a `Camera2D` or an object of a class that extends one,
    /// with the runtime's fields of nodes.
    Node(Rc<Object<NodeParts>>),
    /// The game: an object of the class that extends `Game`, with the
    /// runtime's fields of games.
    Game(Rc<Object<GameFields<Interpreted>>>),
    /// A `Body`, with the runtime's fields of bodies.
    Body(Rc<BodyObject>),
    /// A `Sound`, with the runtime's fields of sounds.
    Sound(Rc<SoundObject>),
}

/// An object: its class, the fields its script classes declare, and the
/// runtime's fields of the engine class it is (`E`), which the runtime
/// reads by reference (decision 70). Without an engine class, `E` is
/// `()`; code that only needs the script fields sees any object as an
/// `Object<dyn Any>`.
pub struct Object<E: ?Sized = ()> {
    /// Its class: the one it was created as, or the same class of a
    /// program reloaded since (decision 76).
    class: Cell<ClassRef>,
    /// The script fields of every class of its hierarchy, the root's first.
    fields: RefCell<Vec<Value>>,
    /// The runtime's fields of its engine class.
    pub(crate) engine: E,
}

impl<E> Object<E> {
    pub(crate) const fn new(class: ClassRef, fields: Vec<Value>, engine: E) -> Self {
        Self {
            class: Cell::new(class),
            fields: RefCell::new(fields),
            engine,
        }
    }
}

impl<E: ?Sized> Object<E> {
    /// Its class.
    pub(crate) fn class(&self) -> ClassRef {
        self.class.get()
    }

    /// Makes it an object of `class` of a reloaded program, with `fields`
    /// laid out as that class lays them out.
    pub(crate) fn reshape(&self, class: ClassRef, fields: Vec<Value>) -> Result<(), RunError> {
        *self
            .fields
            .try_borrow_mut()
            .map_err(|_| OVERLAPPING_FIELDS)? = fields;
        self.class.set(class);
        Ok(())
    }

    /// The fields it has, by their places.
    pub(crate) fn fields(&self) -> Result<Vec<Value>, RunError> {
        Ok(self
            .fields
            .try_borrow()
            .map_err(|_| OVERLAPPING_FIELDS)?
            .clone())
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
impl<E: ?Sized> fmt::Debug for Object<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Object")
            .field("class", &self.class.get())
            .finish_non_exhaustive()
    }
}

/// Objects are equal only to themselves.
impl<E: ?Sized> PartialEq for Object<E> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::addr_eq(self, other)
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
    #[cfg(test)]
    pub(crate) fn text(text: &str) -> Self {
        Self::String(Arc::from(text))
    }

    /// A new list with these elements.
    pub(crate) fn list(elements: Vec<Self>) -> Self {
        Self::List(Rc::new(RefCell::new(elements)))
    }

    /// The object the value refers to, whatever its engine class, for
    /// what only needs its class and script fields.
    pub(crate) fn object(&self) -> Option<&Object<dyn Any>> {
        match self {
            Self::Object(object) => Some(&**object),
            Self::Color(object) => Some(&**object),
            Self::Node(object) => Some(&**object),
            Self::Game(object) => Some(&**object),
            Self::Body(object) => Some(&**object),
            Self::Sound(object) => Some(&**object),
            _ => None,
        }
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
            (Self::Color(left), Self::Color(right)) => Rc::ptr_eq(left, right),
            (Self::Node(left), Self::Node(right)) => Rc::ptr_eq(left, right),
            (Self::Game(left), Self::Game(right)) => Rc::ptr_eq(left, right),
            (Self::Body(left), Self::Body(right)) => Rc::ptr_eq(left, right),
            (Self::Sound(left), Self::Sound(right)) => Rc::ptr_eq(left, right),
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
    pub(crate) fn to_text(&self) -> Result<Arc<str>, RunError> {
        Ok(match self {
            Self::String(text) => Arc::clone(text),
            Self::Short(value) => Arc::from(value.to_string()),
            Self::Int(value) => Arc::from(value.to_string()),
            Self::BigInt(value) => Arc::from(value.to_string()),
            Self::Float(value) => Arc::from(star_script_runtime::float_to_string(value)),
            Self::Double(value) => Arc::from(star_script_runtime::float_to_string(value)),
            Self::Bool(value) => Arc::from(value.to_string()),
            Self::Null
            | Self::List(_)
            | Self::Object(_)
            | Self::Color(_)
            | Self::Node(_)
            | Self::Game(_)
            | Self::Body(_)
            | Self::Sound(_) => {
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
