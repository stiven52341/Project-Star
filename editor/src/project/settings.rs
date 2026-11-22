//! Everything chosen when creating a project.

use std::fmt;

use star_engine::prelude::TargetPlatform;

use super::ProjectDimension;
use super::name::{ProjectName, ProjectNameError};

/// Why a set of choices cannot become a project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProjectSettingsError {
    /// The project name is not valid.
    InvalidName(ProjectNameError),
    /// No target platform was selected.
    NoPlatformSelected,
}

impl fmt::Display for ProjectSettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidName(error) => error.fmt(formatter),
            Self::NoPlatformSelected => formatter.write_str("no target platform selected"),
        }
    }
}

impl std::error::Error for ProjectSettingsError {}

/// Validated settings of a new project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectSettings {
    name: ProjectName,
    dimension: ProjectDimension,
    platforms: Vec<TargetPlatform>,
}

impl ProjectSettings {
    /// Validates the user's choices.
    pub(crate) fn new(
        name: &str,
        dimension: ProjectDimension,
        platforms: Vec<TargetPlatform>,
    ) -> Result<Self, ProjectSettingsError> {
        let name = ProjectName::parse(name).map_err(ProjectSettingsError::InvalidName)?;
        if platforms.is_empty() {
            return Err(ProjectSettingsError::NoPlatformSelected);
        }
        Ok(Self {
            name,
            dimension,
            platforms,
        })
    }

    pub(crate) fn name(&self) -> &str {
        self.name.as_str()
    }

    pub(crate) fn dimension(&self) -> ProjectDimension {
        self.dimension
    }

    pub(crate) fn platforms(&self) -> &[TargetPlatform] {
        &self.platforms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_choices_become_settings() {
        let settings = ProjectSettings::new(
            "Space Game",
            ProjectDimension::ThreeD,
            vec![TargetPlatform::Linux],
        )
        .expect("valid settings");

        assert_eq!(settings.name(), "Space Game");
        assert_eq!(settings.dimension(), ProjectDimension::ThreeD);
        assert_eq!(settings.platforms(), [TargetPlatform::Linux]);
    }

    #[test]
    fn at_least_one_platform_is_required() {
        let result = ProjectSettings::new("Space Game", ProjectDimension::TwoD, Vec::new());

        assert_eq!(result, Err(ProjectSettingsError::NoPlatformSelected));
    }

    #[test]
    fn invalid_names_are_reported() {
        let result =
            ProjectSettings::new("", ProjectDimension::TwoD, vec![TargetPlatform::Windows]);

        assert_eq!(
            result,
            Err(ProjectSettingsError::InvalidName(ProjectNameError::Empty))
        );
    }
}
