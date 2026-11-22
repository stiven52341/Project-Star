//! Scenes for ProjectStar.
//!
//! A scene is a tree of nodes (Godot style) whose behaviour is built from
//! components (Unity style). Every node is an entity of the ECS world, with a
//! [`Name`] and its place in the tree ([`Parent`] and [`Children`]), changed
//! through the [`Hierarchy`] methods of the world. A 2D node is placed by its
//! [`Transform2D`], relative to its parent; [`TransformPropagation`] computes
//! where it ends up in the world ([`GlobalTransform2D`]).

mod hierarchy;
mod transform;

pub use hierarchy::{Children, Hierarchy, Name, Parent};
pub use transform::{GlobalTransform2D, Transform2D, TransformPropagation};
