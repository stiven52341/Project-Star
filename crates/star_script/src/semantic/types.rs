//! The types values can have.

use super::ClassId;

/// Type of a value, as the compiler understands it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// `int`: 64-bit signed integer.
    Int,
    /// `float`: 32-bit floating-point number.
    Float,
    /// `bool`
    Bool,
    /// `string`
    String,
    /// What a method without `->` returns: no value at all.
    Void,
    /// Type of the `null` literal, before it is given a nullable type.
    Null,
    /// `a..b`, which only exists as the iterable of a `for`.
    Range,
    /// An object of a class (never `null`).
    Class(ClassId),
    /// `[Element]`
    List(Box<Type>),
    /// `Inner?`: a value of `Inner` or `null`.
    Nullable(Box<Type>),
    /// Type of code that already has an error. It is compatible with
    /// everything, so one mistake does not cause a cascade of messages.
    Error,
}

impl Type {
    /// Whether values of the type can go to another thread, copied: numbers,
    /// `bool`, `string`, and lists and nullables of them. Not objects.
    #[must_use]
    pub fn is_plain_value(&self) -> bool {
        match self {
            Self::Int | Self::Float | Self::Bool | Self::String | Self::Error => true,
            Self::List(inner) | Self::Nullable(inner) => inner.is_plain_value(),
            Self::Void | Self::Null | Self::Range | Self::Class(_) => false,
        }
    }

    /// `[element]`
    #[must_use]
    pub fn list(element: Self) -> Self {
        Self::List(Box::new(element))
    }

    /// The same type, also allowing `null`.
    #[must_use]
    pub fn nullable(self) -> Self {
        match self {
            Self::Nullable(_) | Self::Null | Self::Void | Self::Error => self,
            other => Self::Nullable(Box::new(other)),
        }
    }

    /// The same type without `null`: `Player?` → `Player`.
    #[must_use]
    pub fn non_null(&self) -> Self {
        match self {
            Self::Nullable(inner) => (**inner).clone(),
            other => other.clone(),
        }
    }

    /// Whether `null` is a possible value.
    #[must_use]
    pub const fn is_nullable(&self) -> bool {
        matches!(self, Self::Nullable(_) | Self::Null)
    }

    /// Whether the type comes from code with an error.
    #[must_use]
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Error)
    }

    /// `int` or `float`.
    #[must_use]
    pub const fn is_numeric(&self) -> bool {
        matches!(self, Self::Int | Self::Float)
    }

    /// Whether a variable of this type can start without an explicit value:
    /// `0`, `0.0`, `false`, `""`, `[]` or `null`. Objects cannot.
    #[must_use]
    pub const fn has_default_value(&self) -> bool {
        !matches!(self, Self::Class(_))
    }

    /// The class of an object type, ignoring `?`.
    #[must_use]
    pub fn class(&self) -> Option<ClassId> {
        match self {
            Self::Class(id) => Some(*id),
            Self::Nullable(inner) => inner.class(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nullable_wraps_only_once() {
        assert_eq!(Type::Int.nullable().nullable(), Type::Int.nullable());
        assert_eq!(Type::Null.nullable(), Type::Null);
    }

    #[test]
    fn non_null_removes_the_question_mark() {
        assert_eq!(Type::Int.nullable().non_null(), Type::Int);
        assert_eq!(Type::Float.non_null(), Type::Float);
    }

    #[test]
    fn only_objects_need_an_explicit_value() {
        assert!(Type::list(Type::Int).has_default_value());
        assert!(!Type::Class(ClassId::new(0)).has_default_value());
        assert!(Type::Class(ClassId::new(0)).nullable().has_default_value());
    }
}
