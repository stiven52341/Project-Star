//! Creation of a project's folder structure on disk.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{self, Write as _};
use std::path::{Path, PathBuf};

use star_engine::star_platform::user_directories::home_directory;

use super::ProjectSettings;
use super::manifest::{MANIFEST_FILE_NAME, render_manifest};

/// Folder, inside the project, where games keep their resources: the one
/// the player reads.
pub(crate) use star_player::project::ASSETS_DIRECTORY;
/// Folder, inside the user's home, offered by default to store projects.
const DEFAULT_PROJECTS_DIRECTORY: &str = "ProjectStar Projects";

/// Why a project could not be created.
#[derive(Debug)]
pub(crate) enum CreateProjectError {
    /// A file or folder with the project's name already exists.
    AlreadyExists(PathBuf),
    /// The operating system refused a file operation.
    Io(io::Error),
}

impl fmt::Display for CreateProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyExists(path) => write!(formatter, "{} already exists", path.display()),
            Self::Io(error) => write!(formatter, "cannot write the project: {error}"),
        }
    }
}

impl std::error::Error for CreateProjectError {}

impl From<io::Error> for CreateProjectError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

/// Folder where a project named like `settings` would be created inside `location`.
pub(crate) fn project_directory(settings: &ProjectSettings, location: &Path) -> PathBuf {
    location.join(settings.name())
}

/// Creates the project folder inside `location` (created if missing) with its
/// manifest and an empty assets folder. Returns the project folder.
///
/// Never overwrites: fails if the project folder already exists.
pub(crate) fn create_project(
    settings: &ProjectSettings,
    location: &Path,
) -> Result<PathBuf, CreateProjectError> {
    let directory = project_directory(settings, location);
    if directory.exists() {
        return Err(CreateProjectError::AlreadyExists(directory));
    }
    fs::create_dir_all(directory.join(ASSETS_DIRECTORY))?;
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.join(MANIFEST_FILE_NAME))?
        .write_all(
            render_manifest(settings)
                .map_err(io::Error::other)?
                .as_bytes(),
        )?;
    Ok(directory)
}

/// Suggested place to store projects: a folder in the user's home.
pub(crate) fn default_projects_location() -> PathBuf {
    home_directory()
        .unwrap_or_default()
        .join(DEFAULT_PROJECTS_DIRECTORY)
}

#[cfg(test)]
mod tests {
    use star_engine::prelude::TargetPlatform;

    use super::*;
    use crate::project::ProjectDimension;
    use crate::project::TestDirectory;

    fn settings() -> ProjectSettings {
        ProjectSettings::new(
            "My Game",
            ProjectDimension::TwoD,
            vec![TargetPlatform::Linux],
        )
        .expect("valid settings")
    }

    #[test]
    fn creates_folder_with_manifest_and_assets() {
        let location = TestDirectory::new("creates_folder");

        let project = create_project(&settings(), location.path()).expect("project created");

        assert_eq!(project, location.path().join("My Game"));
        assert!(project.join(ASSETS_DIRECTORY).is_dir());
        let manifest = fs::read_to_string(project.join(MANIFEST_FILE_NAME)).expect("manifest");
        assert_eq!(
            manifest,
            render_manifest(&settings()).expect("serializable")
        );
    }

    #[test]
    fn refuses_to_overwrite_an_existing_project() {
        let location = TestDirectory::new("refuses_overwrite");
        create_project(&settings(), location.path()).expect("first creation");

        let result = create_project(&settings(), location.path());

        assert!(matches!(result, Err(CreateProjectError::AlreadyExists(_))));
    }

    #[test]
    fn default_location_ends_in_projects_folder() {
        assert!(default_projects_location().ends_with(DEFAULT_PROJECTS_DIRECTORY));
    }
}
