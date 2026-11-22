//! The world: every entity, its components and the global resources.

use std::any::TypeId;
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;

use star_core::{EngineError, EngineResult};

use crate::query::Query;
use crate::resources::Resources;
use crate::sparse_set::{ComponentStorage, SparseSet};
use crate::{Entity, EntityAllocator};

/// Entities with their components, and resources.
///
/// Any `'static` type can be a component (attached to an entity, at most one
/// of each type) or a resource (one value for the whole world). Spawning,
/// despawning and adding or removing components need `&mut World`; reading
/// and changing existing ones only `&World`, so queries can run inside
/// queries as long as they do not change the same component types.
#[derive(Default)]
pub struct World {
    entities: EntityAllocator,
    components: HashMap<TypeId, RefCell<Box<dyn ComponentStorage>>>,
    resources: Resources,
}

impl World {
    /// An empty world.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an entity without components.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::CapacityExceeded`] if there are too many entities.
    pub fn spawn(&mut self) -> EngineResult<Entity> {
        self.entities.allocate()
    }

    /// Destroys `entity` and its components. Returns `false` if it was not alive.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.entities.deallocate(entity) {
            return false;
        }
        for storage in self.components.values_mut() {
            storage.get_mut().remove_entity(entity);
        }
        true
    }

    /// Whether `entity` exists.
    #[must_use]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.entities.is_alive(entity)
    }

    /// Number of living entities.
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.entities.alive_count()
    }

    /// Gives `entity` the component `component`, returning the one of the
    /// same type it replaces.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Ecs`] if `entity` is not alive.
    pub fn insert<T: 'static>(&mut self, entity: Entity, component: T) -> EngineResult<Option<T>> {
        if !self.is_alive(entity) {
            return Err(EngineError::Ecs(format!(
                "cannot add a component to {entity:?}: it is not alive"
            )));
        }
        let storage = self
            .components
            .entry(TypeId::of::<T>())
            .or_insert_with(|| RefCell::new(Box::new(SparseSet::<T>::default())));
        Ok(storage
            .get_mut()
            .as_any_mut()
            .downcast_mut::<SparseSet<T>>()
            .and_then(|storage| storage.insert(entity, component)))
    }

    /// Takes the component `T` away from `entity`.
    pub fn remove<T: 'static>(&mut self, entity: Entity) -> Option<T> {
        self.components
            .get_mut(&TypeId::of::<T>())?
            .get_mut()
            .as_any_mut()
            .downcast_mut::<SparseSet<T>>()?
            .remove(entity)
    }

    /// Whether `entity` has a component `T`.
    #[must_use]
    pub fn has<T: 'static>(&self, entity: Entity) -> bool {
        self.storage::<T>().is_some_and(|cell| {
            cell.try_borrow().is_ok_and(|storage| {
                storage
                    .as_any()
                    .downcast_ref::<SparseSet<T>>()
                    .is_some_and(|storage| storage.get(entity).is_some())
            })
        })
    }

    /// The component `T` of `entity`. `None` if it has none, or if a running
    /// query is changing the components `T`.
    #[must_use]
    pub fn get<T: 'static>(&self, entity: Entity) -> Option<Ref<'_, T>> {
        let storage = self.storage::<T>()?.try_borrow().ok()?;
        Ref::filter_map(storage, |storage| {
            storage
                .as_any()
                .downcast_ref::<SparseSet<T>>()
                .and_then(|storage| storage.get(entity))
        })
        .ok()
    }

    /// The component `T` of `entity`, to change it. `None` if it has none, or
    /// if a running query is using the components `T`.
    #[must_use]
    pub fn get_mut<T: 'static>(&self, entity: Entity) -> Option<RefMut<'_, T>> {
        let storage = self.storage::<T>()?.try_borrow_mut().ok()?;
        RefMut::filter_map(storage, |storage| {
            storage
                .as_any_mut()
                .downcast_mut::<SparseSet<T>>()
                .and_then(|storage| storage.get_mut(entity))
        })
        .ok()
    }

    /// Calls `visit` with every entity that has all the components of `Q`,
    /// and those components: `world.each::<(&mut Position, &Velocity)>(|entity,
    /// (position, velocity)| …)`. Entities are visited in no particular order.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Ecs`] if `Q` asks for a component type twice
    /// with at least one `&mut`, or changes a type that a running query uses.
    pub fn each<Q: Query>(&self, mut visit: impl FnMut(Entity, Q::Items<'_>)) -> EngineResult<()> {
        let Some(mut borrows) = Q::borrow(self)? else {
            return Ok(());
        };
        // The entities cannot change while the query borrows the world, so
        // walking them by position is safe and needs no copy of the list.
        let count = Q::entities(&borrows).len();
        for position in 0..count {
            let Some(&entity) = Q::entities(&borrows).get(position) else {
                break;
            };
            if let Some(items) = Q::items(&mut borrows, entity) {
                visit(entity, items);
            }
        }
        Ok(())
    }

    /// Stores the resource `value`, returning the previous one of its type.
    pub fn insert_resource<T: 'static>(&mut self, value: T) -> Option<T> {
        self.resources.insert(value)
    }

    /// Takes the resource of type `T` out of the world.
    pub fn remove_resource<T: 'static>(&mut self) -> Option<T> {
        self.resources.remove()
    }

    /// Whether there is a resource of type `T`.
    #[must_use]
    pub fn has_resource<T: 'static>(&self) -> bool {
        self.resources.contains::<T>()
    }

    /// The resource of type `T`. `None` if there is none, or if it is being changed.
    #[must_use]
    pub fn resource<T: 'static>(&self) -> Option<Ref<'_, T>> {
        self.resources.get()
    }

    /// The resource of type `T`, to change it. `None` if there is none, or
    /// if it is in use.
    #[must_use]
    pub fn resource_mut<T: 'static>(&self) -> Option<RefMut<'_, T>> {
        self.resources.get_mut()
    }

    pub(crate) fn storage<T: 'static>(&self) -> Option<&RefCell<Box<dyn ComponentStorage>>> {
        self.components.get(&TypeId::of::<T>())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Position(f32);

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Velocity(f32);

    #[derive(Debug, PartialEq)]
    struct Name(&'static str);

    fn spawn_with(world: &mut World, position: f32, velocity: Option<f32>) -> Entity {
        let entity = world.spawn().unwrap();
        world.insert(entity, Position(position)).unwrap();
        if let Some(velocity) = velocity {
            world.insert(entity, Velocity(velocity)).unwrap();
        }
        entity
    }

    #[test]
    fn components_belong_to_their_entity() {
        let mut world = World::new();
        let entity = spawn_with(&mut world, 1.0, None);

        assert_eq!(*world.get::<Position>(entity).unwrap(), Position(1.0));
        assert!(world.has::<Position>(entity));
        assert!(!world.has::<Velocity>(entity));
    }

    #[test]
    fn inserting_again_replaces_the_component() {
        let mut world = World::new();
        let entity = spawn_with(&mut world, 1.0, None);

        let previous = world.insert(entity, Position(2.0)).unwrap();

        assert_eq!(previous, Some(Position(1.0)));
        assert_eq!(*world.get::<Position>(entity).unwrap(), Position(2.0));
    }

    #[test]
    fn components_can_be_changed_and_removed() {
        let mut world = World::new();
        let entity = spawn_with(&mut world, 1.0, None);

        world.get_mut::<Position>(entity).unwrap().0 = 5.0;

        assert_eq!(world.remove::<Position>(entity), Some(Position(5.0)));
        assert!(world.get::<Position>(entity).is_none());
    }

    #[test]
    fn despawning_removes_the_components() {
        let mut world = World::new();
        let entity = spawn_with(&mut world, 1.0, Some(2.0));

        assert!(world.despawn(entity));

        assert!(!world.is_alive(entity));
        assert!(!world.despawn(entity));
        assert!(world.get::<Velocity>(entity).is_none());
        assert_eq!(world.entity_count(), 0);
    }

    #[test]
    fn dead_entities_cannot_get_components() {
        let mut world = World::new();
        let entity = world.spawn().unwrap();
        world.despawn(entity);

        let error = world.insert(entity, Position(0.0)).unwrap_err();

        assert!(matches!(error, EngineError::Ecs(_)));
    }

    #[test]
    fn queries_visit_entities_with_every_component() {
        let mut world = World::new();
        let moving = spawn_with(&mut world, 1.0, Some(2.0));
        spawn_with(&mut world, 3.0, None);

        let mut visited = Vec::new();
        world
            .each::<(&Position, &Velocity)>(|entity, (position, velocity)| {
                visited.push((entity, *position, *velocity));
            })
            .unwrap();

        assert_eq!(visited, [(moving, Position(1.0), Velocity(2.0))]);
    }

    #[test]
    fn queries_change_some_components_while_reading_others() {
        let mut world = World::new();
        let entity = spawn_with(&mut world, 1.0, Some(2.0));

        world
            .each::<(&mut Position, &Velocity)>(|_, (position, velocity)| position.0 += velocity.0)
            .unwrap();

        assert_eq!(*world.get::<Position>(entity).unwrap(), Position(3.0));
    }

    #[test]
    fn single_component_queries_need_no_tuple() {
        let mut world = World::new();
        spawn_with(&mut world, 1.0, None);
        spawn_with(&mut world, 2.0, None);

        let mut total = 0.0;
        world
            .each::<&Position>(|_, position| total += position.0)
            .unwrap();

        assert_eq!(total, 3.0);
    }

    #[test]
    fn queries_of_missing_components_visit_nothing() {
        let mut world = World::new();
        spawn_with(&mut world, 1.0, None);

        let mut visits = 0;
        world
            .each::<(&Position, &Name)>(|_, _| visits += 1)
            .unwrap();

        assert_eq!(visits, 0);
    }

    #[test]
    fn conflicting_queries_are_errors() {
        let mut world = World::new();
        spawn_with(&mut world, 1.0, None);

        let result = world.each::<(&mut Position, &Position)>(|_, _| {});

        assert!(matches!(result, Err(EngineError::Ecs(_))));
    }

    #[test]
    fn queries_can_read_other_entities_while_running() {
        let mut world = World::new();
        let leader = spawn_with(&mut world, 10.0, None);
        let follower = world.spawn().unwrap();
        world.insert(follower, Velocity(0.0)).unwrap();

        world
            .each::<&mut Velocity>(|_, velocity| {
                velocity.0 = world
                    .get::<Position>(leader)
                    .map_or(0.0, |position| position.0);
            })
            .unwrap();

        assert_eq!(*world.get::<Velocity>(follower).unwrap(), Velocity(10.0));
    }

    #[test]
    fn queries_cannot_change_what_a_running_query_uses() {
        let mut world = World::new();
        spawn_with(&mut world, 1.0, None);

        let mut inner = Ok(());
        world
            .each::<&Position>(|_, _| inner = world.each::<&mut Position>(|_, _| {}))
            .unwrap();

        assert!(inner.is_err());
    }

    #[test]
    fn rare_components_are_found_among_many_entities() {
        let mut world = World::new();
        for value in 0..100_u8 {
            spawn_with(&mut world, f32::from(value), None);
        }
        let named = spawn_with(&mut world, 0.5, None);
        world.insert(named, Name("rare")).unwrap();

        let mut visits = 0;
        world
            .each::<(&Position, &Name)>(|entity, _| {
                visits += 1;
                assert_eq!(entity, named);
            })
            .unwrap();

        assert_eq!(visits, 1);
    }

    #[test]
    fn resources_are_shared_by_the_world() {
        let mut world = World::new();
        world.insert_resource(Name("game"));

        assert_eq!(*world.resource::<Name>().unwrap(), Name("game"));
        assert!(world.has_resource::<Name>());
        assert_eq!(world.remove_resource::<Name>(), Some(Name("game")));
        assert!(world.resource_mut::<Name>().is_none());
    }
}
