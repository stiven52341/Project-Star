//! Fields of objects, which change through shared references.

use std::cell::RefCell;

use crate::fault;

/// A field of a StarScript object.
///
/// Objects are shared (several variables may hold the same one), so their
/// fields must change through a shared reference. Reading copies the value
/// out and writing replaces it, so no borrow ever outlives the call and
/// two accesses can never overlap.
#[derive(Debug, Default)]
pub struct Field<T> {
    value: RefCell<T>,
}

impl<T: Clone> Field<T> {
    /// A field holding `value`.
    #[must_use]
    pub const fn new(value: T) -> Self {
        Self {
            value: RefCell::new(value),
        }
    }

    /// A copy of the current value. Lists and objects are copied as
    /// references to the same list or object, as StarScript expects.
    #[must_use]
    pub fn get(&self) -> T {
        match self.value.try_borrow() {
            Ok(value) => value.clone(),
            Err(_) => fault(OVERLAPPING_ACCESS),
        }
    }

    /// Replaces the value.
    pub fn set(&self, value: T) {
        match self.value.try_borrow_mut() {
            Ok(mut current) => *current = value,
            Err(_) => fault(OVERLAPPING_ACCESS),
        }
    }
}

/// Accesses never overlap, since no borrow outlives `get` or `set`; reaching
/// this would be a bug in the compiler or in this crate.
const OVERLAPPING_ACCESS: &str = "internal compiler error: overlapping field access";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_change_through_shared_references() {
        let field = Field::new(1);
        let shared = &field;

        shared.set(shared.get() + 1);

        assert_eq!(field.get(), 2);
    }
}
