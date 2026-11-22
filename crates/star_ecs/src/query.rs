//! Queries: which components a system reads or changes, entity by entity.
//!
//! A query is `&T` (read the components `T`), `&mut T` (change them) or a
//! tuple of those, and visits every entity that has all of them. The world
//! keeps each component type in its own `RefCell`, so one query can change
//! some types while reading others, all checked without `unsafe`: asking for
//! the same type twice in conflicting ways is reported as an error.

use std::any::type_name;
use std::cell::{Ref, RefMut};

use star_core::{EngineError, EngineResult};

use crate::sparse_set::SparseSet;
use crate::{Entity, World};

/// The components of type `T` while a query reads them.
pub struct ReadComponents<'w, T>(Ref<'w, SparseSet<T>>);

/// The components of type `T` while a query changes them.
pub struct WriteComponents<'w, T>(RefMut<'w, SparseSet<T>>);

/// One element of a query: `&T` or `&mut T`.
pub trait QueryParam {
    /// The borrowed components while the query runs.
    type Borrow<'w>;
    /// What the query gives for each entity.
    type Item<'b>;

    /// Borrows the components from `world`; `None` if no entity has them.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Ecs`] if they are already borrowed in a conflicting way.
    fn borrow(world: &World) -> EngineResult<Option<Self::Borrow<'_>>>;

    /// The entities that have the component.
    fn entities<'b>(borrow: &'b Self::Borrow<'_>) -> &'b [Entity];

    /// The component of `entity`, if it has one.
    fn item<'b>(borrow: &'b mut Self::Borrow<'_>, entity: Entity) -> Option<Self::Item<'b>>;
}

impl<T: 'static> QueryParam for &T {
    type Borrow<'w> = ReadComponents<'w, T>;
    type Item<'b> = &'b T;

    fn borrow(world: &World) -> EngineResult<Option<Self::Borrow<'_>>> {
        let Some(cell) = world.storage::<T>() else {
            return Ok(None);
        };
        let storage = cell.try_borrow().map_err(|_| conflict::<T>())?;
        Ok(
            Ref::filter_map(storage, |storage| storage.as_any().downcast_ref())
                .ok()
                .map(ReadComponents),
        )
    }

    fn entities<'b>(borrow: &'b Self::Borrow<'_>) -> &'b [Entity] {
        borrow.0.entities()
    }

    fn item<'b>(borrow: &'b mut Self::Borrow<'_>, entity: Entity) -> Option<Self::Item<'b>> {
        borrow.0.get(entity)
    }
}

impl<T: 'static> QueryParam for &mut T {
    type Borrow<'w> = WriteComponents<'w, T>;
    type Item<'b> = &'b mut T;

    fn borrow(world: &World) -> EngineResult<Option<Self::Borrow<'_>>> {
        let Some(cell) = world.storage::<T>() else {
            return Ok(None);
        };
        let storage = cell.try_borrow_mut().map_err(|_| conflict::<T>())?;
        Ok(
            RefMut::filter_map(storage, |storage| storage.as_any_mut().downcast_mut())
                .ok()
                .map(WriteComponents),
        )
    }

    fn entities<'b>(borrow: &'b Self::Borrow<'_>) -> &'b [Entity] {
        borrow.0.entities()
    }

    fn item<'b>(borrow: &'b mut Self::Borrow<'_>, entity: Entity) -> Option<Self::Item<'b>> {
        borrow.0.get_mut(entity)
    }
}

/// What [`World::each`] visits: a [`QueryParam`] or a tuple of up to 8 of them.
pub trait Query {
    /// The borrowed components while the query runs.
    type Borrows<'w>;
    /// What the query gives for each entity.
    type Items<'b>;

    /// Borrows every component of the query; `None` if some type has no
    /// components at all, so no entity can match.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Ecs`] if a component type is borrowed in a
    /// conflicting way, by this query or by one still running.
    fn borrow(world: &World) -> EngineResult<Option<Self::Borrows<'_>>>;

    /// The entities worth visiting: those of the rarest component.
    fn entities<'b>(borrows: &'b Self::Borrows<'_>) -> &'b [Entity];

    /// The components of `entity`, if it has all of them.
    fn items<'b>(borrows: &'b mut Self::Borrows<'_>, entity: Entity) -> Option<Self::Items<'b>>;
}

impl<P: QueryParam> Query for P {
    type Borrows<'w> = P::Borrow<'w>;
    type Items<'b> = P::Item<'b>;

    fn borrow(world: &World) -> EngineResult<Option<Self::Borrows<'_>>> {
        P::borrow(world)
    }

    fn entities<'b>(borrows: &'b Self::Borrows<'_>) -> &'b [Entity] {
        P::entities(borrows)
    }

    fn items<'b>(borrows: &'b mut Self::Borrows<'_>, entity: Entity) -> Option<Self::Items<'b>> {
        P::item(borrows, entity)
    }
}

macro_rules! tuple_query {
    ($($param:ident),+) => {
        // Each borrow is bound to a variable named like its type parameter.
        #[allow(non_snake_case)]
        impl<$($param: QueryParam),+> Query for ($($param,)+) {
            type Borrows<'w> = ($($param::Borrow<'w>,)+);
            type Items<'b> = ($($param::Item<'b>,)+);

            fn borrow(world: &World) -> EngineResult<Option<Self::Borrows<'_>>> {
                Ok(Some(($(
                    match $param::borrow(world)? {
                        Some(borrow) => borrow,
                        None => return Ok(None),
                    },
                )+)))
            }

            fn entities<'b>(borrows: &'b Self::Borrows<'_>) -> &'b [Entity] {
                let ($($param,)+) = borrows;
                [$($param::entities($param)),+]
                    .into_iter()
                    .min_by_key(|entities| entities.len())
                    .unwrap_or_default()
            }

            fn items<'b>(
                borrows: &'b mut Self::Borrows<'_>,
                entity: Entity,
            ) -> Option<Self::Items<'b>> {
                let ($($param,)+) = borrows;
                Some(($($param::item($param, entity)?,)+))
            }
        }
    };
}

tuple_query!(A);
tuple_query!(A, B);
tuple_query!(A, B, C);
tuple_query!(A, B, C, D);
tuple_query!(A, B, C, D, E);
tuple_query!(A, B, C, D, E, F);
tuple_query!(A, B, C, D, E, F, G);
tuple_query!(A, B, C, D, E, F, G, H);

fn conflict<T>() -> EngineError {
    EngineError::Ecs(format!(
        "the components `{}` are already in use: a query asks for them twice, \
         or changes them while another query is running",
        type_name::<T>()
    ))
}
