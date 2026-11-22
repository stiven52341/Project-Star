//! Scenes for ProjectStar.
//!
//! A scene is a tree of nodes (Godot style) whose behaviour is built from
//! components (Unity style). Every node is an entity of the ECS world, with a
//! [`Name`] and its place in the tree ([`Parent`] and [`Children`]), changed
//! through the [`Hierarchy`] methods of the world. A 2D node is placed by its
//! [`Transform2D`], relative to its parent; [`TransformPropagation`] computes
//! where it ends up in the world ([`GlobalTransform2D`]), where its
//! [`SpriteNode`] is drawn. A [`Scene`] is a tree of nodes saved in a
//! `.scene` file (TOML) and instanced into a world as many times as needed;
//! its [`SceneOutline`] describes the file's nodes without loading their images.

mod hierarchy;
mod scene;
mod scene_file;
mod scene_outline;
mod sprite_node;
mod transform;

pub use hierarchy::{Children, Hierarchy, Name, Parent};
pub use scene::{Scene, TextureLoader};
pub use scene_outline::{NodeOutline, SceneOutline, SpriteOutline};
pub use sprite_node::{SpriteNode, draw_sprite_nodes};
pub use transform::{GlobalTransform2D, Transform2D, TransformPropagation};
