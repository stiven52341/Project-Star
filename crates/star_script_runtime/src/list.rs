//! `[T]`: lists shared by reference.

use std::cell::RefCell;
use std::rc::Rc;

use crate::{StarEq, fault};

/// A StarScript list.
///
/// Lists are shared: after `var a = b`, both names refer to the same list
/// and `a.push(1)` is visible through `b`. Cloning a `List` clones the
/// reference, not the elements.
///
/// Every method borrows the elements only while it runs and never calls
/// StarScript code meanwhile, so accesses can never overlap.
#[derive(Debug)]
pub struct List<T> {
    elements: Rc<RefCell<Vec<T>>>,
}

impl<T> Clone for List<T> {
    fn clone(&self) -> Self {
        Self {
            elements: Rc::clone(&self.elements),
        }
    }
}

impl<T> Default for List<T> {
    fn default() -> Self {
        Self {
            elements: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

impl<T: Clone> List<T> {
    /// An empty list: `[]`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A list with these elements: `[a, b, c]`.
    #[must_use]
    pub fn from_vec(elements: Vec<T>) -> Self {
        Self {
            elements: Rc::new(RefCell::new(elements)),
        }
    }

    /// `list[index]`.
    #[must_use]
    #[track_caller]
    pub fn get(&self, index: i64) -> T {
        self.read(|elements| {
            let position = checked_index(index, elements.len());
            elements[position].clone()
        })
    }

    /// `list[index] = value`.
    #[track_caller]
    pub fn set(&self, index: i64, value: T) {
        self.write(|elements| {
            let position = checked_index(index, elements.len());
            elements[position] = value;
        });
    }

    /// `length()`: number of elements.
    #[must_use]
    pub fn length(&self) -> i64 {
        self.read(|elements| i64::try_from(elements.len()).unwrap_or(i64::MAX))
    }

    /// `is_empty()`: whether there are no elements.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.read(Vec::is_empty)
    }

    /// `push(value)`: adds an element at the end.
    pub fn push(&self, value: T) {
        self.write(|elements| elements.push(value));
    }

    /// `pop()`: removes the last element; `null` if there is none.
    pub fn pop(&self) -> Option<T> {
        self.write(Vec::pop)
    }

    /// `clear()`: removes every element.
    pub fn clear(&self) {
        self.write(Vec::clear);
    }

    /// The current elements, for a `for` loop: changing the list inside the
    /// loop does not change what the loop goes through.
    #[must_use]
    pub fn snapshot(&self) -> Vec<T> {
        self.read(Vec::clone)
    }

    fn read<R>(&self, action: impl FnOnce(&Vec<T>) -> R) -> R {
        match self.elements.try_borrow() {
            Ok(elements) => action(&elements),
            Err(_) => fault(OVERLAPPING_ACCESS),
        }
    }

    fn write<R>(&self, action: impl FnOnce(&mut Vec<T>) -> R) -> R {
        match self.elements.try_borrow_mut() {
            Ok(mut elements) => action(&mut elements),
            Err(_) => fault(OVERLAPPING_ACCESS),
        }
    }
}

impl<T: Clone + StarEq> List<T> {
    /// `contains(value)`: whether an element is equal to `value`.
    #[must_use]
    pub fn contains(&self, value: &T) -> bool {
        self.read(|elements| elements.iter().any(|element| element.star_eq(value)))
    }
}

/// Lists are equal if they have equal elements in the same order.
impl<T: Clone + StarEq> StarEq for List<T> {
    fn star_eq(&self, other: &Self) -> bool {
        if Rc::ptr_eq(&self.elements, &other.elements) {
            return true;
        }
        let (mine, theirs) = (self.snapshot(), other.snapshot());
        mine.len() == theirs.len() && mine.iter().zip(&theirs).all(|(a, b)| a.star_eq(b))
    }
}

/// See [`crate::Field`]: reaching this would be a bug in this crate.
const OVERLAPPING_ACCESS: &str = "internal compiler error: overlapping list access";

#[track_caller]
fn checked_index(index: i64, length: usize) -> usize {
    match usize::try_from(index) {
        Ok(position) if position < length => position,
        _ => fault(&format!(
            "index {index} is out of range for a list of length {length}"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_are_shared_by_reference() {
        let list = List::new();
        let alias = list.clone();

        alias.push(1_i64);

        assert_eq!(list.length(), 1);
        assert_eq!(list.get(0), 1);
    }

    #[test]
    fn elements_can_be_replaced_and_removed() {
        let list = List::from_vec(vec![1_i64, 2]);

        list.set(1, 5);

        assert_eq!(list.pop(), Some(5));
        assert_eq!(list.pop(), Some(1));
        assert_eq!(list.pop(), None);
        assert!(list.is_empty());
    }

    #[test]
    #[should_panic(expected = "index 2 is out of range for a list of length 2")]
    fn reading_past_the_end_is_a_fault() {
        let _ = List::from_vec(vec![1_i64, 2]).get(2);
    }

    #[test]
    #[should_panic(expected = "index -1 is out of range")]
    fn negative_indices_are_a_fault() {
        let _ = List::from_vec(vec![1_i64]).get(-1);
    }

    #[test]
    fn lists_compare_by_content() {
        let first = List::from_vec(vec![1_i64, 2]);
        let second = List::from_vec(vec![1_i64, 2]);

        assert!(first.star_eq(&second));
        assert!(first.contains(&2));
        second.clear();
        assert!(!first.star_eq(&second));
    }

    #[test]
    fn snapshots_do_not_follow_later_changes() {
        let list = List::from_vec(vec![1_i64]);
        let snapshot = list.snapshot();

        list.push(2);

        assert_eq!(snapshot, vec![1]);
    }
}
