//! Game projects: their settings and how they are created and opened.

mod creation;
mod dimension;
mod manifest;
mod name;
mod opening;
mod settings;
#[cfg(test)]
mod test_support;

pub(crate) use creation::{
    ASSETS_DIRECTORY, CreateProjectError, create_project, default_projects_location,
    project_directory,
};
pub(crate) use dimension::ProjectDimension;
pub(crate) use manifest::MANIFEST_FILE_NAME;
pub(crate) use name::{MAX_NAME_LENGTH, ProjectNameError};
pub(crate) use opening::{OpenProjectError, Project, open_project};
pub(crate) use settings::{ProjectSettings, ProjectSettingsError};
#[cfg(test)]
pub(crate) use test_support::TestDirectory;
