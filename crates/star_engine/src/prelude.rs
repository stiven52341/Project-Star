//! The most commonly used engine types, for glob import.

pub use crate::app::App;
pub use crate::game::{Game, GameContext};
pub use crate::{EngineWorld, MeshFile, SpriteFile};
pub use star_assets::{AssetFolder, AssetPath};
pub use star_audio::{Audio, PlaySettings, VoiceId};
pub use star_core::{ENGINE_NAME, ENGINE_VERSION, EngineError, EngineResult, Sound, Time};
pub use star_ecs::{Entity, Schedule, Stage, World};
pub use star_input::{
    ActionMap, Actions, Binding, Gamepad, GamepadAxis, GamepadButton, GamepadId, Gamepads, Input,
    Key, Keyboard, Mouse, MouseButton, Touch, TouchId, Touches,
};
pub use star_math::{Affine2, Mat4, Quat, Vec2, Vec3};
pub use star_physics::{
    BodyId, BodyId3d, BodyKind, BodySettings, Contact, ContactPhase, Physics2d, Physics3d, Shape2d,
    Shape3d, ShapeKind, ShapeKind3d,
};
pub use star_platform::TargetPlatform;
pub use star_render::{
    AmbientLight, Camera, Color, DirectionalLight, Frame, GraphicsBackend, Image, Light,
    MAX_LIGHTS, Material, Mesh3dId, MeshData, MeshId, MeshShape, MeshVertex, PointLight,
    Projection, Sprite, TextureFilter, TextureId, TextureRegion, Vertex,
};
pub use star_scene::{
    Body2D, Body3D, Children, GlobalTransform2D, GlobalTransform3D, Hierarchy, LightNode, MeshNode,
    Name, NodeContact, Parent, PhysicsNodes, PhysicsNodes3d, Scene, SceneOutline, SpriteNode,
    Transform2D, Transform3D,
};
