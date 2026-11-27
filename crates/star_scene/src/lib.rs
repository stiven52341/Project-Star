//! Scenes for ProjectStar.
//!
//! A scene is a tree of nodes (Godot style) whose behaviour is built from
//! components (Unity style). Every node is an entity of the ECS world, with a
//! [`Name`] and its place in the tree ([`Parent`] and [`Children`]), changed
//! through the [`Hierarchy`] methods of the world. A 2D node is placed by its
//! [`Transform2D`], relative to its parent; [`TransformPropagation`] computes
//! where it ends up in the world ([`GlobalTransform2D`]), where its
//! [`SpriteNode`] is drawn and its [`Light2dNode`] lights the sprites. A 3D node is placed by its [`Transform3D`]
//! ([`GlobalTransform3D`] in the world) and can show a [`MeshNode`] or shine
//! a [`LightNode`]. A [`Camera2dNode`] or a [`Camera3dNode`] makes a node a
//! point of view of the game. A [`Hidden`] node is not drawn, nor are its
//! children. A [`Body2D`] makes a 2D node a body of the 2D physics,
//! which [`step_node_physics`] moves it with, and a [`Body3D`] a 3D node a
//! body of the 3D physics ([`step_node_physics_3d`]). A [`Scene`] is a tree
//! of nodes saved in a `.scene` file (TOML) and instanced into a world as
//! many times as needed; its [`SceneOutline`] describes the file's nodes
//! without loading their images.

mod animation_states;
mod animator_node;
mod body_data;
mod body_node;
mod bone_node;
mod camera_node;
mod collision_mesh;
mod field_value;
mod hierarchy;
mod light_2d_node;
mod model_node;
mod model_pose;
mod node_3d;
mod node_3d_data;
mod node_physics;
mod node_physics_3d;
mod scene;
mod scene_file;
mod scene_outline;
mod shown_model;
mod sound_player_node;
mod sprite_body;
mod sprite_frames;
mod sprite_node;
mod track_animations;
mod track_sampling;
mod transform;
mod visibility;

pub use animation_states::{
    AnimationState, AnimationStates, BlendPoint, Condition, Motion, STATES_EXTENSION,
    StateTransition, StateValue, Test, ValueKind,
};
pub use animator_node::AnimatorOutline;
pub use body_node::{Body2D, Body3D, NodeBody};
pub use bone_node::{Bone2dNode, DEFAULT_BONE_LENGTH};
pub use camera_node::{Camera2dNode, Camera3dNode};
pub use collision_mesh::CollisionMesh;
pub use field_value::FieldValue;
pub use hierarchy::{Children, Hierarchy, Name, Parent, Tags};
pub use light_2d_node::Light2dNode;
pub use model_node::ModelNode;
pub use model_pose::{ClipAt, ModelPose, Poser};
pub use node_3d::{LightNode, MeshNode, draw_3d_nodes};
pub use node_3d_data::MeshOutline;
pub use node_physics::{BodyNodes, NodeContact, PhysicsNodes, PhysicsNodes3d, step_node_physics};
pub use node_physics_3d::step_node_physics_3d;
pub use scene::{Scene, SceneAssets, SceneSources};
pub use scene_outline::{NodeOutline, SceneOutline, SpriteOutline};
pub use shown_model::{ModelDrawing, ShownModel};
pub use sound_player_node::{DEFAULT_FAR, DEFAULT_NEAR, DEFAULT_PLAYER_VOLUME, SoundPlayerNode};
pub use sprite_body::{SpriteLook, shaped_by_sprite};
pub use sprite_frames::{DEFAULT_CLIP_SPEED, FRAMES_EXTENSION, FrameClip, SpriteFrames};
pub use sprite_node::{SpriteNode, draw_sprite_nodes};
pub use track_animations::{
    ANIMATIONS_EXTENSION, Interpolation, Key, MethodCall, PATH_SEPARATOR, PropertyTrack,
    TrackAnimation, TrackAnimations,
};
pub use transform::{
    GlobalTransform2D, GlobalTransform3D, Transform2D, Transform3D, TransformPropagation,
};
pub use visibility::{Hidden, HiddenInTree, is_shown};
