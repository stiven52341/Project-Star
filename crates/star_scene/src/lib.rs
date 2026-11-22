//! Scenes for ProjectStar.
//!
//! A scene is a tree of nodes (Godot style) whose behaviour is built from
//! components (Unity style). Every node is an entity of the ECS world, with a
//! [`Name`] and its place in the tree ([`Parent`] and [`Children`]), changed
//! through the [`Hierarchy`] methods of the world.

mod hierarchy;

pub use hierarchy::{Children, Hierarchy, Name, Parent};
