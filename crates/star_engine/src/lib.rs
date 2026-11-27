//! ProjectStar: a cross-platform 2D and 3D game engine.
//!
//! This crate is the public entry point. Games normally only need:
//!
//! ```
//! use star_engine::prelude::*;
//! ```

pub mod app;
mod engine_world;
mod fallback_logger;
mod fps_counter;
pub mod game;
mod game_assets;
mod mesh_file;
mod model_library;
pub mod prelude;
mod runtime;
mod script_3d;
mod script_body;
pub mod script_nodes;
mod sound_library;
mod sprite_file;
mod texture_library;
mod texture_slot;

pub use engine_world::EngineWorld;
pub use fps_counter::{FPS_UPDATE_SECONDS, FpsDisplay};
pub use mesh_file::MeshFile;
pub use sprite_file::SpriteFile;
pub use star_assets;
pub use star_audio;
pub use star_core;
pub use star_ecs;
pub use star_input;
pub use star_math;
pub use star_physics;
pub use star_platform;
pub use star_platform::android_main;
pub use star_render;
pub use star_scene;
