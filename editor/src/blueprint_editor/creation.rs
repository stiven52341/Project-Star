//! New blueprints: a node that does nothing yet, ready to be added to a
//! scene or to the game.

use std::fs;
use std::io;
use std::path::Path;

use star_blueprint::{Blueprint, Literal, Parameter, is_valid_name};

use super::edit;
use crate::project::BLUEPRINT_EXTENSION;

/// The class new blueprints extend.
const NEW_BLUEPRINT_BASE: &str = "Node";
/// The events of a node, which new blueprints redefine to fill them in.
const START_EVENT: &str = "start";
const UPDATE_EVENT: &str = "update";
const DELTA_PARAMETER: &str = "delta";
const DELTA_TYPE: &str = "float";

/// Why a blueprint could not be created.
#[derive(Debug)]
pub(crate) enum NewBlueprintError {
    /// StarScript would not accept the class's name.
    InvalidName,
    /// A file with the blueprint's name already exists.
    AlreadyExists(String),
    /// The file could not be written.
    Io(io::Error),
}

/// Creates the blueprint of the class `class` in the `assets` folder, as
/// `<class>.blueprint`, and returns that path inside the folder. It never
/// overwrites a file.
pub(crate) fn create_blueprint(assets: &Path, class: &str) -> Result<String, NewBlueprintError> {
    if !is_valid_name(class) {
        return Err(NewBlueprintError::InvalidName);
    }
    let path = format!("{class}{BLUEPRINT_EXTENSION}");
    let text = new_blueprint(class)
        .to_text()
        .map_err(|error| NewBlueprintError::Io(io::Error::other(error)))?;
    fs::create_dir_all(assets).map_err(NewBlueprintError::Io)?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(assets.join(&path))
        .map_err(|error| match error.kind() {
            io::ErrorKind::AlreadyExists => NewBlueprintError::AlreadyExists(path.clone()),
            _ => NewBlueprintError::Io(error),
        })?;
    io::Write::write_all(&mut file, text.as_bytes()).map_err(NewBlueprintError::Io)?;
    Ok(path)
}

/// A node called `class` that redefines `start` and `update`, empty.
fn new_blueprint(class: &str) -> Blueprint {
    let start = edit::new_graph(START_EVENT.to_owned());
    let mut update = edit::new_graph(UPDATE_EVENT.to_owned());
    update.parameters.push(Parameter {
        name: DELTA_PARAMETER.to_owned(),
        ty: DELTA_TYPE.to_owned(),
    });
    let mut graphs = vec![start, update];
    for graph in &mut graphs {
        graph.is_override = true;
    }
    Blueprint {
        class: class.to_owned(),
        extends: Some(NEW_BLUEPRINT_BASE.to_owned()),
        base_arguments: vec![Literal::String(class.to_owned())],
        variables: Vec::new(),
        graphs,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::TestDirectory;

    #[test]
    fn new_blueprints_compile() {
        let translation = star_blueprint::translate(&new_blueprint("Rock")).expect("translatable");

        let checked = star_script::check_source(translation.source());

        assert!(checked.is_ok(), "{}\n{checked:?}", translation.source());
    }

    #[test]
    fn new_blueprints_are_written_in_the_assets_folder() {
        let assets = TestDirectory::new("new_blueprint");

        let path = create_blueprint(assets.path(), "Rock").expect("created");

        assert_eq!(path, "Rock.blueprint");
        let text = fs::read_to_string(assets.path().join(&path)).expect("written");
        assert_eq!(
            Blueprint::from_text(&text).ok(),
            Some(new_blueprint("Rock"))
        );
    }

    #[test]
    fn existing_files_are_not_overwritten() {
        let assets = TestDirectory::new("new_blueprint_exists");
        fs::create_dir_all(assets.path()).expect("folder");
        fs::write(assets.path().join("Rock.blueprint"), "mine").expect("file");

        let result = create_blueprint(assets.path(), "Rock");

        assert!(matches!(result, Err(NewBlueprintError::AlreadyExists(_))));
        let text = fs::read_to_string(assets.path().join("Rock.blueprint")).expect("kept");
        assert_eq!(text, "mine");
    }

    #[test]
    fn class_names_must_be_valid() {
        let assets = TestDirectory::new("new_blueprint_name");

        let result = create_blueprint(assets.path(), "big rock");

        assert!(matches!(result, Err(NewBlueprintError::InvalidName)));
    }
}
