//! The `project.toml` file that identifies a folder as a ProjectStar project.

use std::fmt;

use serde::{Deserialize, Serialize};
use star_engine::prelude::{ENGINE_VERSION, TargetPlatform};

use super::{ProjectDimension, ProjectSettings, ProjectSettingsError};

/// File name of the project manifest, at the root of the project folder.
pub(crate) const MANIFEST_FILE_NAME: &str = "project.toml";

const MANIFEST_HEADER: &str = "# ProjectStar project file.\n";

/// Why a manifest's text does not describe a valid project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ManifestError {
    /// The text is not valid TOML or lacks required fields.
    Syntax(String),
    /// The `dimension` field holds an unknown value.
    UnknownDimension(String),
    /// The `platforms` field holds an unknown value.
    UnknownPlatform(String),
    /// The values are well formed but do not make valid settings.
    InvalidSettings(ProjectSettingsError),
}

impl fmt::Display for ManifestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(reason) => formatter.write_str(reason),
            Self::UnknownDimension(id) => write!(formatter, "unknown dimension \"{id}\""),
            Self::UnknownPlatform(id) => write!(formatter, "unknown platform \"{id}\""),
            Self::InvalidSettings(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ManifestError {}

/// On-disk layout of the manifest.
#[derive(Debug, Serialize, Deserialize)]
struct ManifestFile {
    project: ManifestProject,
}

#[derive(Debug, Serialize, Deserialize)]
struct ManifestProject {
    name: String,
    engine_version: String,
    dimension: String,
    platforms: Vec<String>,
}

/// Renders the manifest of `settings` as TOML.
pub(crate) fn render_manifest(settings: &ProjectSettings) -> Result<String, toml::ser::Error> {
    let file = ManifestFile {
        project: ManifestProject {
            name: settings.name().to_owned(),
            engine_version: ENGINE_VERSION.to_owned(),
            dimension: settings.dimension().id().to_owned(),
            platforms: settings
                .platforms()
                .iter()
                .map(|platform| platform.id().to_owned())
                .collect(),
        },
    };
    Ok(format!("{MANIFEST_HEADER}{}", toml::to_string(&file)?))
}

/// Reads the project settings stored in a manifest's `text`.
pub(crate) fn parse_manifest(text: &str) -> Result<ProjectSettings, ManifestError> {
    let file: ManifestFile =
        toml::from_str(text).map_err(|error| ManifestError::Syntax(error.message().to_owned()))?;
    let project = file.project;
    let dimension = ProjectDimension::from_id(&project.dimension)
        .ok_or(ManifestError::UnknownDimension(project.dimension))?;
    let platforms = project
        .platforms
        .into_iter()
        .map(|id| TargetPlatform::from_id(&id).ok_or(ManifestError::UnknownPlatform(id)))
        .collect::<Result<Vec<_>, _>>()?;
    ProjectSettings::new(&project.name, dimension, platforms)
        .map_err(ManifestError::InvalidSettings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> ProjectSettings {
        ProjectSettings::new(
            "Space Game",
            ProjectDimension::ThreeD,
            vec![TargetPlatform::Windows, TargetPlatform::Android],
        )
        .expect("valid settings")
    }

    #[test]
    fn manifest_lists_every_setting() {
        let manifest = render_manifest(&settings()).expect("serializable");

        assert!(manifest.starts_with(MANIFEST_HEADER));
        assert!(manifest.contains("name = \"Space Game\""));
        assert!(manifest.contains("dimension = \"3d\""));
        assert!(manifest.contains(&format!("engine_version = \"{ENGINE_VERSION}\"")));
    }

    #[test]
    fn rendered_manifest_parses_back_to_the_same_settings() {
        let manifest = render_manifest(&settings()).expect("serializable");

        assert_eq!(parse_manifest(&manifest), Ok(settings()));
    }

    #[test]
    fn malformed_text_is_a_syntax_error() {
        assert!(matches!(
            parse_manifest("not = [toml"),
            Err(ManifestError::Syntax(_))
        ));
    }

    #[test]
    fn unknown_platforms_are_reported() {
        let text = "[project]\nname = \"A\"\nengine_version = \"0.1.0\"\n\
                    dimension = \"2d\"\nplatforms = [\"dreamcast\"]\n";

        assert_eq!(
            parse_manifest(text),
            Err(ManifestError::UnknownPlatform("dreamcast".to_owned()))
        );
    }

    #[test]
    fn unknown_dimensions_are_reported() {
        let text = "[project]\nname = \"A\"\nengine_version = \"0.1.0\"\n\
                    dimension = \"4d\"\nplatforms = [\"linux\"]\n";

        assert_eq!(
            parse_manifest(text),
            Err(ManifestError::UnknownDimension("4d".to_owned()))
        );
    }
}
