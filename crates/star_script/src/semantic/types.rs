//! The types values can have.

use super::ClassId;

/// Type of a value, as the compiler understands it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    /// `short`: 16-bit signed integer.
    Short,
    /// `int`: 32-bit signed integer, the usual one.
    Int,
    /// `bigint`: 64-bit signed integer.
    BigInt,
    /// `float`: 32-bit floating-point number, the usual one.
    Float,
    /// `double`: 64-bit floating-point number.
    Double,
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
            Self::Short
            | Self::Int
            | Self::BigInt
            | Self::Float
            | Self::Double
            | Self::Bool
            | Self::String
            | Self::Error => true,
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

    /// An integer type, `float` or `double`.
    #[must_use]
    pub const fn is_numeric(&self) -> bool {
        self.is_integer() || matches!(self, Self::Float | Self::Double)
    }

    /// Whether a number of this type converts by itself to `to`, a wider
    /// number type that holds every value of it exactly: `short` → `int`
    /// → `bigint`, `float` → `double`, `short` → `float`, `int` → `double`.
    #[must_use]
    pub const fn widens_to(&self, to: &Self) -> bool {
        matches!(
            (self, to),
            (
                Self::Short,
                Self::Int | Self::BigInt | Self::Float | Self::Double
            ) | (Self::Int, Self::BigInt | Self::Double)
                | (Self::Float, Self::Double)
        )
    }

    /// The number type two numbers meet in: the same type, or the one the
    /// other widens to. `None` if they are not numbers or neither widens.
    #[must_use]
    pub fn common_number(&self, other: &Self) -> Option<Self> {
        if !self.is_numeric() || !other.is_numeric() {
            None
        } else if self == other || other.widens_to(self) {
            Some(self.clone())
        } else if self.widens_to(other) {
            Some(other.clone())
        } else {
            None
        }
    }

    /// `short`, `int` or `bigint`.
    #[must_use]
    pub const fn is_integer(&self) -> bool {
        matches!(self, Self::Short | Self::Int | Self::BigInt)
    }

    /// The smallest and largest values of an integer type.
    #[must_use]
    pub const fn integer_range(&self) -> Option<(i64, i64)> {
        match self {
            Self::Short => Some((i16::MIN as i64, i16::MAX as i64)),
            Self::Int => Some((i32::MIN as i64, i32::MAX as i64)),
            Self::BigInt => Some((i64::MIN, i64::MAX)),
            _ => None,
        }
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
    }

    #[test]
    fn integer_types_know_their_range() {
        assert_eq!(Type::Short.integer_range(), Some((-32_768, 32_767)));
        assert_eq!(
            Type::Int.integer_range(),
            Some((-2_147_483_648, 2_147_483_647))
        );
        assert_eq!(Type::BigInt.integer_range(), Some((i64::MIN, i64::MAX)));
        assert_eq!(Type::Float.integer_range(), None);
        assert!(Type::Short.is_numeric() && Type::BigInt.is_integer());
        assert!(!Type::Float.is_integer());
    }

    #[test]
    fn numbers_widen_only_without_losing_values() {
        assert!(Type::Short.widens_to(&Type::BigInt));
        assert!(Type::Int.widens_to(&Type::Double));
        assert!(Type::Float.widens_to(&Type::Double));
        assert!(!Type::Int.widens_to(&Type::Float));
        assert!(!Type::BigInt.widens_to(&Type::Double));
        assert!(!Type::Int.widens_to(&Type::Short));
        assert_eq!(Type::Short.common_number(&Type::Int), Some(Type::Int));
        assert_eq!(Type::Double.common_number(&Type::Int), Some(Type::Double));
        assert_eq!(Type::Int.common_number(&Type::Float), None);
        assert_eq!(Type::Int.common_number(&Type::Bool), None);
        assert!(!Type::Class(ClassId::new(0)).has_default_value());
        assert!(Type::Class(ClassId::new(0)).nullable().has_default_value());
    }
}
