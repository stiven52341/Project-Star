//! The most commonly used engine types, for glob import.

pub use crate::app::App;
pub use crate::game::{Game, GameContext};
pub use star_core::{ENGINE_NAME, ENGINE_VERSION, EngineError, EngineResult, Time};
pub use star_ecs::Entity;
pub use star_input::{
    ActionMap, Actions, Binding, Input, Key, Keyboard, Mouse, MouseButton, Touch, TouchId, Touches,
};
pub use star_math::{Mat4, Vec2, Vec3};
pub use star_platform::TargetPlatform;
pub use star_render::{Camera, Color, Frame, GraphicsBackend, MeshId, Projection, Vertex};
