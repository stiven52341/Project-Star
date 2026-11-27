//! Physics for ProjectStar, in 2D and 3D.
//!
//! A [`Physics2d`] holds bodies, each with one [`Shape2d`], and a
//! [`Physics3d`] the same in space, each with one [`Shape3d`]: dynamic
//! ones fall and are pushed, static ones never move, and kinematic ones
//! follow the velocity the game gives them. Time advances in fixed steps,
//! and the bodies that start or stop touching are reported as
//! [`Contact`]s; sensors detect what overlaps them without colliding. The
//! `rapier2d` and `rapier3d` crates do the simulation and stay hidden
//! behind these types, so they can be replaced freely.

mod body;
mod contact;
mod physics_2d;
mod physics_3d;
mod shape;
mod shape_3d;
mod step_clock;
mod triangle_mesh;

pub use body::{BodyKind, BodySettings, DEFAULT_DENSITY, DEFAULT_FRICTION};
pub use contact::{Contact, ContactPhase};
pub use physics_2d::{BodyId, DEFAULT_GRAVITY, Physics2d};
pub use physics_3d::{BodyId3d, DEFAULT_GRAVITY_3D, Physics3d};
pub use shape::{MIN_SHAPE_SCALE, Shape2d, ShapeKind};
pub use shape_3d::{Shape3d, ShapeKind3d};
pub use step_clock::{FIXED_STEP, MAX_STEPS_PER_ADVANCE};
pub use triangle_mesh::TriangleMesh;
