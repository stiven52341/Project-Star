//! `==` and `!=` as StarScript defines them.

use std::rc::Rc;

/// Equality in StarScript: values by content, objects by identity.
///
/// - `int`, `float`, `bool` and `string` compare their values.
/// - Lists compare their elements, in order.
/// - Objects are equal only if they are the same object.
/// - `null` is only equal to `null`.
pub trait StarEq {
    /// Whether `self == other` in StarScript.
    fn star_eq(&self, other: &Self) -> bool;
}

macro_rules! value_equality {
    ($($ty:ty),*) => {
        $(
            impl StarEq for $ty {
                #[allow(clippy::float_cmp)] // StarScript's `==` is exact, like Rust's.
                fn star_eq(&self, other: &Self) -> bool {
                    self == other
                }
            }
        )*
    };
}

value_equality!(i64, f32, bool, String);

impl<T: StarEq> StarEq for Option<T> {
    fn star_eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Some(left), Some(right)) => left.star_eq(right),
            (None, None) => true,
            _ => false,
        }
    }
}

/// Objects: the same object, whatever class the references are seen as.
impl<T: ?Sized> StarEq for Rc<T> {
    fn star_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_compare_by_content() {
        assert!(3_i64.star_eq(&3));
        assert!("a".to_owned().star_eq(&"a".to_owned()));
        assert!(!1.5_f32.star_eq(&2.5));
    }

    #[test]
    fn objects_compare_by_identity() {
        let first = Rc::new(1);
        let same = Rc::clone(&first);
        let equal_but_other = Rc::new(1);

        assert!(first.star_eq(&same));
        assert!(!first.star_eq(&equal_but_other));
    }

    #[test]
    fn null_is_only_equal_to_null() {
        assert!(None::<i64>.star_eq(&None));
        assert!(!Some(1_i64).star_eq(&None));
        assert!(Some(1_i64).star_eq(&Some(1)));
    }
}
