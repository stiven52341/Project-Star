//! Loading an existing project from disk.

use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::ProjectSettings;
use super::manifest::{MANIFEST_FILE_NAME, ManifestError, parse_manifest};

/// A project loaded from disk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Project {
    directory: PathBuf,
    settings: ProjectSettings,
}

impl Project {
    /// Root folder of the project, where `project.toml` lives.
    pub(crate) fn directory(&self) -> &Path {
        &self.directory
    }

    pub(crate) fn settings(&self) -> &ProjectSettings {
        &self.settings
    }
}

/// Why a folder could not be opened as a project.
#[derive(Debug)]
pub(crate) enum OpenProjectError {
    /// The folder has no `project.toml`.
    NotAProject(PathBuf),
    /// The manifest exists but could not be read.
    Io(io::Error),
    /// The manifest's content is invalid.
    InvalidManifest(ManifestError),
}

impl fmt::Display for OpenProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAProject(path) => {
                write!(formatter, "{} is not a ProjectStar project", path.display())
            }
            Self::Io(error) => write!(formatter, "cannot read the project: {error}"),
            Self::InvalidManifest(error) => {
                write!(formatter, "invalid {MANIFEST_FILE_NAME}: {error}")
            }
        }
    }
}

impl std::error::Error for OpenProjectError {}

/// Loads the project whose root folder is `directory`.
pub(crate) fn open_project(directory: &Path) -> Result<Project, OpenProjectError> {
    let manifest_path = directory.join(MANIFEST_FILE_NAME);
    let text = fs::read_to_string(&manifest_path).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            OpenProjectError::NotAProject(directory.to_path_buf())
        } else {
            OpenProjectError::Io(error)
        }
    })?;
    let settings = parse_manifest(&text).map_err(OpenProjectError::InvalidManifest)?;
    Ok(Project {
        directory: directory.to_path_buf(),
        settings,
    })
}

#[cfg(test)]
mod tests {
    use star_engine::prelude::TargetPlatform;

    use super::*;
    use crate::project::TestDirectory;
    use crate::project::{ProjectDimension, create_project};

    #[test]
    fn opens_a_created_project() {
        let location = TestDirectory::new("opens_created");
        let settings =
            ProjectSettings::new("Racer", ProjectDimension::ThreeD, vec![TargetPlatform::Ios])
                .expect("valid settings");
        let directory = create_project(&settings, location.path()).expect("created");

        let project = open_project(&directory).expect("opened");

        assert_eq!(project.settings(), &settings);
        assert_eq!(project.directory(), directory);
    }

    #[test]
    fn folders_without_manifest_are_not_projects() {
        let location = TestDirectory::new("not_a_project");
        fs::create_dir_all(location.path()).expect("folder");

        let result = open_project(location.path());

        assert!(matches!(result, Err(OpenProjectError::NotAProject(_))));
    }

    #[test]
    fn corrupt_manifests_are_reported() {
        let location = TestDirectory::new("corrupt_manifest");
        fs::create_dir_all(location.path()).expect("folder");
        fs::write(location.path().join(MANIFEST_FILE_NAME), "garbage =").expect("written");

        let result = open_project(location.path());

        assert!(matches!(result, Err(OpenProjectError::InvalidManifest(_))));
    }
}
