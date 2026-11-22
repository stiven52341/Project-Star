//! Storage of the components of one type: a sparse set.

use std::any::Any;

use crate::Entity;

/// Marks a free slot of the sparse array.
const NO_COMPONENT: u32 = u32::MAX;

/// The components of one type, packed together.
///
/// Components live in a dense array, so going through all of them reads
/// contiguous memory; a sparse array (indexed by entity) finds the position
/// of any entity's component in constant time. Adding and removing are also
/// constant time: removing moves the last component into the hole.
pub(crate) struct SparseSet<T> {
    /// Position in `dense` of the component of each entity index.
    sparse: Vec<u32>,
    dense: Vec<T>,
    /// The entity of each component in `dense`.
    entities: Vec<Entity>,
}

impl<T> SparseSet<T> {
    /// Gives `entity` the component `value`, returning the one it replaces.
    pub(crate) fn insert(&mut self, entity: Entity, value: T) -> Option<T> {
        if let Some(position) = self.position(entity) {
            return Some(std::mem::replace(&mut self.dense[position], value));
        }
        let slot = entity.index() as usize;
        if slot >= self.sparse.len() {
            self.sparse.resize(slot + 1, NO_COMPONENT);
        }
        // `dense` never grows past `u32::MAX` items: entity indices are `u32`
        // and each entity has at most one component of this type.
        self.sparse[slot] = u32::try_from(self.dense.len()).unwrap_or(NO_COMPONENT);
        self.dense.push(value);
        self.entities.push(entity);
        None
    }

    /// Takes the component of `entity` away.
    pub(crate) fn remove(&mut self, entity: Entity) -> Option<T> {
        let position = self.position(entity)?;
        self.sparse[entity.index() as usize] = NO_COMPONENT;
        let value = self.dense.swap_remove(position);
        self.entities.swap_remove(position);
        if let Some(moved) = self.entities.get(position) {
            self.sparse[moved.index() as usize] = u32::try_from(position).unwrap_or(NO_COMPONENT);
        }
        Some(value)
    }

    pub(crate) fn get(&self, entity: Entity) -> Option<&T> {
        self.position(entity).map(|position| &self.dense[position])
    }

    pub(crate) fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.position(entity)
            .map(|position| &mut self.dense[position])
    }

    /// The entities that have this component, in storage order.
    pub(crate) fn entities(&self) -> &[Entity] {
        &self.entities
    }

    fn position(&self, entity: Entity) -> Option<usize> {
        let position = *self.sparse.get(entity.index() as usize)?;
        let position = usize::try_from(position).ok()?;
        // The generation check rejects stale handles to a reused index.
        (self.entities.get(position) == Some(&entity)).then_some(position)
    }
}

impl<T> Default for SparseSet<T> {
    fn default() -> Self {
        Self {
            sparse: Vec::new(),
            dense: Vec::new(),
            entities: Vec::new(),
        }
    }
}

/// A [`SparseSet`] of any component type, as the world keeps them.
pub(crate) trait ComponentStorage: Any {
    /// Removes the component of `entity`, if it has one.
    fn remove_entity(&mut self, entity: Entity);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: 'static> ComponentStorage for SparseSet<T> {
    fn remove_entity(&mut self, entity: Entity) {
        self.remove(entity);
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EntityAllocator;

    fn entities(count: usize) -> Vec<Entity> {
        let mut allocator = EntityAllocator::new();
        (0..count).map(|_| allocator.allocate().unwrap()).collect()
    }

    #[test]
    fn components_are_found_by_entity() {
        let all = entities(2);
        let (first, second) = (all[0], all[1]);
        let mut set = SparseSet::default();

        set.insert(second, "b");
        set.insert(first, "a");

        assert_eq!(set.get(first), Some(&"a"));
        assert_eq!(set.get(second), Some(&"b"));
        assert_eq!(set.entities(), [second, first]);
    }

    #[test]
    fn inserting_again_replaces_the_component() {
        let entity = entities(1)[0];
        let mut set = SparseSet::default();
        set.insert(entity, 1);

        assert_eq!(set.insert(entity, 2), Some(1));
        assert_eq!(set.get(entity), Some(&2));
        assert_eq!(set.entities().len(), 1);
    }

    #[test]
    fn removing_keeps_the_other_components_reachable() {
        let all = entities(3);
        let mut set = SparseSet::default();
        for (value, &entity) in all.iter().enumerate() {
            set.insert(entity, value);
        }

        assert_eq!(set.remove(all[0]), Some(0));

        assert_eq!(set.get(all[0]), None);
        assert_eq!(set.get(all[1]), Some(&1));
        assert_eq!(set.get_mut(all[2]), Some(&mut 2));
        assert_eq!(set.remove(all[0]), None);
    }

    #[test]
    fn stale_handles_do_not_see_the_new_component() {
        let mut allocator = EntityAllocator::new();
        let stale = allocator.allocate().unwrap();
        allocator.deallocate(stale);
        let fresh = allocator.allocate().unwrap();
        let mut set = SparseSet::default();

        set.insert(fresh, "fresh");

        assert_eq!(set.get(stale), None);
    }
}
