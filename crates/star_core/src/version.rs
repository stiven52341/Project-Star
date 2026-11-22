//! Engine identity.

/// Public name of the engine.
pub const ENGINE_NAME: &str = "ProjectStar";

/// Engine version, taken from the workspace manifest.
pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
