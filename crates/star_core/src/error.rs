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
}
