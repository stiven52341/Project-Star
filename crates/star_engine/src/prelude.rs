//! The most commonly used engine types, for glob import.

pub use crate::app::App;
pub use crate::game::{Game, GameContext};
pub use crate::{EngineWorld, SpriteFile};
pub use star_assets::{AssetFolder, AssetPath};
pub use star_audio::{Audio, PlaySettings, VoiceId};
pub use star_core::{ENGINE_NAME, ENGINE_VERSION, EngineError, EngineResult, Sound, Time};
pub use star_ecs::{Entity, Schedule, Stage, World};
pub use star_input::{
    ActionMap, Actions, Binding, Gamepad, GamepadAxis, GamepadButton, GamepadId, Gamepads, Input,
    Key, Keyboard, Mouse, MouseButton, Touch, TouchId, Touches,
};
pub use star_math::{Affine2, Mat4, Vec2, Vec3};
pub use star_physics::{
    BodyId, BodyKind, BodySettings, Contact, ContactPhase, Physics2d, Shape2d, ShapeKind,
};
pub use star_platform::TargetPlatform;
pub use star_render::{
    Camera, Color, Frame, GraphicsBackend, Image, Material, Mesh3dId, MeshData, MeshId, MeshVertex,
    Projection, Sprite, TextureFilter, TextureId, TextureRegion, Vertex,
};
pub use star_scene::{
    Body2D, Children, GlobalTransform2D, Hierarchy, Name, NodeContact, Parent, PhysicsNodes, Scene,
    SceneOutline, SpriteNode, Transform2D,
};
