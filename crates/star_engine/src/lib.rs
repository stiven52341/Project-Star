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
pub mod game;
mod game_assets;
pub mod prelude;
mod runtime;
mod script_body;
pub mod script_nodes;
mod sound_library;
mod sprite_file;
mod texture_library;

pub use engine_world::EngineWorld;
pub use sprite_file::SpriteFile;
pub use star_assets;
pub use star_audio;
pub use star_core;
pub use star_ecs;
pub use star_input;
pub use star_math;
pub use star_physics;
pub use star_platform;
pub use star_render;
pub use star_scene;
