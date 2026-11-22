//! The most commonly used engine types, for glob import.

pub use crate::EngineWorld;
pub use crate::app::App;
pub use crate::game::{Game, GameContext};
pub use star_core::{ENGINE_NAME, ENGINE_VERSION, EngineError, EngineResult, Time};
pub use star_ecs::{Entity, Schedule, Stage, World};
pub use star_input::{
    ActionMap, Actions, Binding, Gamepad, GamepadAxis, GamepadButton, GamepadId, Gamepads, Input,
    Key, Keyboard, Mouse, MouseButton, Touch, TouchId, Touches,
};
pub use star_math::{Mat4, Vec2, Vec3};
pub use star_platform::TargetPlatform;
pub use star_render::{
    Camera, Color, Frame, GraphicsBackend, Image, MeshId, Projection, Sprite, TextureFilter,
    TextureId, TextureRegion, Vertex,
};
