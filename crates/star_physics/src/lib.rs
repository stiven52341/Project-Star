//! Physics for ProjectStar.
//!
//! A [`Physics2d`] holds bodies, each with one [`Shape2d`]: dynamic ones
//! fall and are pushed, static ones never move, and kinematic ones follow
//! the velocity the game gives them. Time advances in fixed steps, and the
//! bodies that start or stop touching are reported as [`Contact`]s;
//! sensors detect what overlaps them without colliding. The `rapier2d`
//! crate does the simulation and stays hidden behind these types, so it
//! can be replaced freely.

mod body;
mod contact;
mod physics_2d;
mod shape;

pub use body::{BodyKind, BodySettings, DEFAULT_DENSITY, DEFAULT_FRICTION};
pub use contact::{Contact, ContactPhase};
pub use physics_2d::{BodyId, DEFAULT_GRAVITY, FIXED_STEP, MAX_STEPS_PER_ADVANCE, Physics2d};
pub use shape::{Shape2d, ShapeKind};
