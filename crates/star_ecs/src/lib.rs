//! Entity-component-system foundation for ProjectStar.
//!
//! A [`World`] holds entities, their components (any `'static` type, kept
//! per type in sparse sets) and resources (global values). Queries visit the
//! entities that have a set of components:
//!
//! ```
//! use star_ecs::World;
//!
//! struct Position(f32);
//! struct Velocity(f32);
//!
//! let mut world = World::new();
//! let ball = world.spawn()?;
//! world.insert(ball, Position(0.0))?;
//! world.insert(ball, Velocity(2.0))?;
//!
//! world.each::<(&mut Position, &Velocity)>(|_, (position, velocity)| {
//!     position.0 += velocity.0;
//! })?;
//!
//! assert_eq!(world.get::<Position>(ball).map(|position| position.0), Some(2.0));
//! # Ok::<(), star_core::EngineError>(())
//! ```

mod entity;
mod query;
mod resources;
mod sparse_set;
mod world;

pub use entity::{Entity, EntityAllocator};
pub use query::{Query, QueryParam, ReadComponents, WriteComponents};
pub use world::World;
