//! Core primitives shared by every ProjectStar crate.
//!
//! This crate must stay dependency-free and platform-agnostic.

pub mod error;
pub mod image;
pub mod model;
pub mod sound;
pub mod time;
pub mod version;

pub use error::{EngineError, EngineResult};
pub use image::Image;
pub use model::{Model, ModelMaterial, ModelPart, ModelVertex};
pub use sound::Sound;
pub use time::Time;
pub use version::{ENGINE_NAME, ENGINE_VERSION};
