//! Engine-wide error type.

use std::error::Error;
use std::fmt;

/// Result alias used across the engine.
pub type EngineResult<T> = Result<T, EngineError>;

/// Errors that any engine subsystem can report.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EngineError {
    /// The current platform is not supported by the engine or a feature.
    UnsupportedPlatform(String),
    /// A subsystem failed to initialize.
    Initialization {
        /// Name of the subsystem that failed.
        subsystem: &'static str,
        /// Human-readable cause of the failure.
        reason: String,
    },
    /// The operating system or windowing layer reported an error.
    Platform(String),
    /// The GPU or the rendering backend reported an error.
    Rendering(String),
    /// An input setting is not valid, e.g. two actions with the same name.
    Input(String),
    /// An asset cannot be used: a missing file, a corrupt image…
    Asset(String),
    /// Entities, components or systems were misused, e.g. a component added
    /// to a destroyed entity.
    Ecs(String),
    /// A scene or its node tree was misused or is invalid, e.g. a node made
    /// a child of its own descendant, or a corrupt scene file.
    Scene(String),
    /// A fixed-capacity resource has no room left.
    CapacityExceeded {
        /// Name of the exhausted resource.
        resource: &'static str,
    },
}

impl fmt::Display for EngineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPlatform(platform) => {
                write!(formatter, "unsupported platform: {platform}")
            }
            Self::Initialization { subsystem, reason } => {
                write!(formatter, "failed to initialize {subsystem}: {reason}")
            }
            Self::Platform(reason) => write!(formatter, "platform error: {reason}"),
            Self::Rendering(reason) => write!(formatter, "rendering error: {reason}"),
            Self::Input(reason) => write!(formatter, "input error: {reason}"),
            Self::Asset(reason) => write!(formatter, "asset error: {reason}"),
            Self::Ecs(reason) => write!(formatter, "ECS error: {reason}"),
            Self::Scene(reason) => write!(formatter, "scene error: {reason}"),
            Self::CapacityExceeded { resource } => {
                write!(formatter, "capacity exceeded for {resource}")
            }
        }
    }
}

impl Error for EngineError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_includes_subsystem_and_reason() {
        let error = EngineError::Initialization {
            subsystem: "renderer",
            reason: "no GPU found".to_owned(),
        };

        assert_eq!(
            error.to_string(),
            "failed to initialize renderer: no GPU found"
        );
    }

    #[test]
    fn input_errors_explain_the_reason() {
        let error = EngineError::Input("the action \"jump\" already exists".to_owned());

        assert_eq!(
            error.to_string(),
            "input error: the action \"jump\" already exists"
        );
    }

    #[test]
    fn asset_errors_explain_the_reason() {
        let error = EngineError::Asset("\"hero.png\" does not exist".to_owned());

        assert_eq!(
            error.to_string(),
            "asset error: \"hero.png\" does not exist"
        );
    }

    #[test]
    fn scene_errors_explain_the_reason() {
        let error = EngineError::Scene("\"Hero\" cannot be its own parent".to_owned());

        assert_eq!(
            error.to_string(),
            "scene error: \"Hero\" cannot be its own parent"
        );
    }
}
