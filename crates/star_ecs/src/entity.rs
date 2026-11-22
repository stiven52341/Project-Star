//! Entity handles and their allocator.

use star_core::{EngineError, EngineResult};

/// A lightweight handle that identifies an object in the world.
///
/// The generation detects stale handles: once an entity is destroyed, its index
/// may be reused, but old handles to it stop being alive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl Entity {
    /// Slot index of the entity.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }

    /// Generation of the slot when this handle was created.
    #[must_use]
    pub const fn generation(self) -> u32 {
        self.generation
    }
}

/// Creates and destroys entities, recycling freed slots.
#[derive(Debug, Default)]
pub struct EntityAllocator {
    generations: Vec<u32>,
    free_indices: Vec<u32>,
}

impl EntityAllocator {
    /// Creates an empty allocator.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a new entity, reusing a freed slot when available.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::CapacityExceeded`] if every `u32` index is in use.
    pub fn allocate(&mut self) -> EngineResult<Entity> {
        if let Some(index) = self.free_indices.pop() {
            let generation = self.generations[index as usize];
            return Ok(Entity { index, generation });
        }

        let index =
            u32::try_from(self.generations.len()).map_err(|_| EngineError::CapacityExceeded {
                resource: "entities",
            })?;
        self.generations.push(0);
        Ok(Entity {
            index,
            generation: 0,
        })
    }

    /// Destroys an entity. Returns `false` if it was already dead.
    pub fn deallocate(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        let generation = &mut self.generations[entity.index as usize];
        *generation = generation.wrapping_add(1);
        self.free_indices.push(entity.index);
        true
    }

    /// Whether the handle still refers to a living entity.
    #[must_use]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.generations
            .get(entity.index as usize)
            .is_some_and(|&generation| generation == entity.generation)
    }

    /// Number of living entities.
    #[must_use]
    pub fn alive_count(&self) -> usize {
        self.generations.len() - self.free_indices.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocated_entity_is_alive() {
        let mut allocator = EntityAllocator::new();

        let entity = allocator.allocate().unwrap();

        assert!(allocator.is_alive(entity));
        assert_eq!(allocator.alive_count(), 1);
    }

    #[test]
    fn deallocated_entity_is_dead_and_cannot_be_freed_twice() {
        let mut allocator = EntityAllocator::new();
        let entity = allocator.allocate().unwrap();

        assert!(allocator.deallocate(entity));
        assert!(!allocator.is_alive(entity));
        assert!(!allocator.deallocate(entity));
    }

    #[test]
    fn reused_slot_invalidates_stale_handle() {
        let mut allocator = EntityAllocator::new();
        let stale = allocator.allocate().unwrap();
        allocator.deallocate(stale);

        let fresh = allocator.allocate().unwrap();

        assert_eq!(fresh.index(), stale.index());
        assert_ne!(fresh.generation(), stale.generation());
        assert!(!allocator.is_alive(stale));
        assert!(allocator.is_alive(fresh));
    }
}
