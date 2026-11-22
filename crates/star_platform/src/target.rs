//! Detection of the platform the engine is running on.

use std::fmt;

/// Operating systems the engine can target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum TargetPlatform {
    /// Microsoft Windows.
    Windows,
    /// Apple macOS.
    MacOs,
    /// Linux desktop distributions.
    Linux,
    /// Google Android.
    Android,
    /// Apple iOS.
    Ios,
    /// A platform the engine does not support yet.
    Unknown,
}

impl TargetPlatform {
    /// Every platform the engine supports, desktop first.
    pub const SUPPORTED: [Self; 5] = [
        Self::Windows,
        Self::MacOs,
        Self::Linux,
        Self::Android,
        Self::Ios,
    ];

    /// Platform the current binary was compiled for.
    #[must_use]
    pub const fn current() -> Self {
        if cfg!(target_os = "windows") {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::MacOs
        } else if cfg!(target_os = "android") {
            Self::Android
        } else if cfg!(target_os = "ios") {
            Self::Ios
        } else if cfg!(target_os = "linux") {
            Self::Linux
        } else {
            Self::Unknown
        }
    }

    /// Whether the platform is a desktop operating system.
    #[must_use]
    pub const fn is_desktop(self) -> bool {
        matches!(self, Self::Windows | Self::MacOs | Self::Linux)
    }

    /// Whether the platform is a mobile operating system.
    #[must_use]
    pub const fn is_mobile(self) -> bool {
        matches!(self, Self::Android | Self::Ios)
    }

    /// Whether the engine supports this platform.
    #[must_use]
    pub const fn is_supported(self) -> bool {
        self.is_desktop() || self.is_mobile()
    }

    /// Human-readable name of the platform.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Windows => "Windows",
            Self::MacOs => "macOS",
            Self::Linux => "Linux",
            Self::Android => "Android",
            Self::Ios => "iOS",
            Self::Unknown => "Unknown",
        }
    }

    /// Platform whose [`id`](Self::id) is `id`, among the supported ones.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Self> {
        Self::SUPPORTED
            .into_iter()
            .find(|platform| platform.id() == id)
    }

    /// Stable lowercase identifier, suitable for configuration files.
    #[must_use]
    pub const fn id(self) -> &'static str {
        match self {
            Self::Windows => "windows",
            Self::MacOs => "macos",
            Self::Linux => "linux",
            Self::Android => "android",
            Self::Ios => "ios",
            Self::Unknown => "unknown",
        }
    }
}

impl fmt::Display for TargetPlatform {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platforms_are_either_desktop_or_mobile() {
        for platform in TargetPlatform::SUPPORTED {
            assert_ne!(platform.is_desktop(), platform.is_mobile(), "{platform}");
        }
    }

    #[test]
    fn unknown_platform_is_not_supported() {
        assert!(!TargetPlatform::Unknown.is_supported());
    }

    #[test]
    fn every_listed_platform_is_supported() {
        assert!(
            TargetPlatform::SUPPORTED
                .iter()
                .all(|platform| platform.is_supported())
        );
    }

    #[test]
    fn identifiers_round_trip() {
        for platform in TargetPlatform::SUPPORTED {
            assert_eq!(TargetPlatform::from_id(platform.id()), Some(platform));
        }
        assert_eq!(TargetPlatform::from_id("dreamcast"), None);
    }

    #[test]
    fn identifiers_are_lowercase_names() {
        for platform in TargetPlatform::SUPPORTED {
            assert_eq!(platform.id(), platform.name().to_lowercase());
        }
    }
}
