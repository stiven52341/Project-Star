//! Resources: single, global values of the world (the time, the input, a score…).

use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;

/// At most one value of each type, each borrowed on its own.
#[derive(Default)]
pub(crate) struct Resources {
    values: HashMap<TypeId, RefCell<Box<dyn Any>>>,
}

impl Resources {
    /// Stores `value`, returning the previous resource of its type.
    pub(crate) fn insert<T: 'static>(&mut self, value: T) -> Option<T> {
        self.values
            .insert(TypeId::of::<T>(), RefCell::new(Box::new(value)))
            .and_then(|previous| previous.into_inner().downcast().ok())
            .map(|previous| *previous)
    }

    pub(crate) fn remove<T: 'static>(&mut self) -> Option<T> {
        self.values
            .remove(&TypeId::of::<T>())
            .and_then(|value| value.into_inner().downcast().ok())
            .map(|value| *value)
    }

    /// The resource of type `T`; `None` if there is none or it is borrowed mutably.
    pub(crate) fn get<T: 'static>(&self) -> Option<Ref<'_, T>> {
        let cell = self.values.get(&TypeId::of::<T>())?;
        Ref::filter_map(cell.try_borrow().ok()?, |value| value.downcast_ref()).ok()
    }

    /// The resource of type `T`; `None` if there is none or it is borrowed.
    pub(crate) fn get_mut<T: 'static>(&self) -> Option<RefMut<'_, T>> {
        let cell = self.values.get(&TypeId::of::<T>())?;
        RefMut::filter_map(cell.try_borrow_mut().ok()?, |value| value.downcast_mut()).ok()
    }

    pub(crate) fn contains<T: 'static>(&self) -> bool {
        self.values.contains_key(&TypeId::of::<T>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    struct Score(u32);

    #[test]
    fn resources_are_found_by_type() {
        let mut resources = Resources::default();

        assert_eq!(resources.insert(Score(3)), None);

        assert_eq!(*resources.get::<Score>().unwrap(), Score(3));
        assert!(resources.get::<String>().is_none());
    }

    #[test]
    fn inserting_again_replaces_the_resource() {
        let mut resources = Resources::default();
        resources.insert(Score(1));

        assert_eq!(resources.insert(Score(2)), Some(Score(1)));
        assert_eq!(resources.remove::<Score>(), Some(Score(2)));
        assert!(!resources.contains::<Score>());
    }

    #[test]
    fn resources_can_be_changed() {
        let mut resources = Resources::default();
        resources.insert(Score(1));

        resources.get_mut::<Score>().unwrap().0 += 1;

        assert_eq!(*resources.get::<Score>().unwrap(), Score(2));
    }

    #[test]
    fn a_resource_being_changed_cannot_be_read() {
        let mut resources = Resources::default();
        resources.insert(Score(1));

        let _changing = resources.get_mut::<Score>().unwrap();

        assert!(resources.get::<Score>().is_none());
    }
}
