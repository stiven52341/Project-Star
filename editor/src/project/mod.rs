//! Game projects: their settings and how they are created and opened.

mod asset_kinds;
mod creation;
mod dimension;
mod input_actions;
mod main_scene;
mod manifest;
mod name;
mod opening;
mod settings;
#[cfg(test)]
mod test_support;

pub(crate) use asset_kinds::{
    BLUEPRINT_EXTENSION, FRAMES_EXTENSION, RUST_EXTENSION, SCENE_EXTENSION, SCRIPT_EXTENSION,
};
pub(crate) use creation::{
    ASSETS_DIRECTORY, CreateProjectError, create_project, default_projects_location,
    project_directory,
};
pub(crate) use dimension::ProjectDimension;
pub(crate) use input_actions::add_missing_actions;
pub(crate) use main_scene::{read_main_scene, set_main_scene};
pub(crate) use manifest::MANIFEST_FILE_NAME;
pub(crate) use name::{MAX_NAME_LENGTH, ProjectNameError};
pub(crate) use opening::{OpenProjectError, Project, open_project};
pub(crate) use settings::{ProjectSettings, ProjectSettingsError};
#[cfg(test)]
pub(crate) use test_support::TestDirectory;
