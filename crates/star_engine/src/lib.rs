//! ProjectStar: a cross-platform 2D and 3D game engine.
//!
//! This crate is the public entry point. Games normally only need:
//!
//! ```
//! use star_engine::prelude::*;
//! ```

pub mod app;
mod engine_world;
pub mod game;
pub mod prelude;
mod runtime;
mod texture_library;

pub use engine_world::EngineWorld;
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
