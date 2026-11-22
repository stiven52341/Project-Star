//! Core primitives shared by every ProjectStar crate.
//!
//! This crate must stay dependency-free and platform-agnostic.

pub mod error;
pub mod time;
pub mod version;

pub use error::{EngineError, EngineResult};
pub use time::Time;
pub use version::{ENGINE_NAME, ENGINE_VERSION};
