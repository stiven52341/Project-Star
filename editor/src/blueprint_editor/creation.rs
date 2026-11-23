//! New blueprints: a node that does nothing yet, ready to be added to a
//! scene or to the game.

use star_blueprint::{Blueprint, Literal, Parameter, is_valid_name};

use super::edit;

/// The class new blueprints extend.
const NEW_BLUEPRINT_BASE: &str = "Node";
/// The events of a node, which new blueprints redefine to fill them in.
const START_EVENT: &str = "start";
const UPDATE_EVENT: &str = "update";
const DELTA_PARAMETER: &str = "delta";
const DELTA_TYPE: &str = "float";

/// The text of the blueprint of a new class called `class`: a node that
/// does nothing yet. `None` if StarScript would not accept the name.
pub(crate) fn new_blueprint_text(class: &str) -> Option<String> {
    if !is_valid_name(class) {
        return None;
    }
    new_blueprint(class)
        .to_text()
        .inspect_err(|error| log::warn!("{error}"))
        .ok()
}

/// The StarScript of a new class called `class`, the same a new blueprint
/// of it translates to. `None` if StarScript would not accept the name.
pub(crate) fn new_script_text(class: &str) -> Option<String> {
    if !is_valid_name(class) {
        return None;
    }
    star_blueprint::translate(&new_blueprint(class))
        .inspect_err(|errors| log::warn!("{errors:?}"))
        .ok()
        .map(|translation| translation.source().to_owned())
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

    #[test]
    fn new_blueprints_compile() {
        let translation = star_blueprint::translate(&new_blueprint("Rock")).expect("translatable");

        let checked = star_script::check_source(translation.source());

        assert!(checked.is_ok(), "{}\n{checked:?}", translation.source());
    }

    #[test]
    fn new_blueprint_texts_read_back_as_the_new_class() {
        let text = new_blueprint_text("Rock").unwrap();

        assert_eq!(
            Blueprint::from_text(&text).ok(),
            Some(new_blueprint("Rock"))
        );
    }

    #[test]
    fn new_scripts_compile() {
        let source = new_script_text("Rock").unwrap();

        let checked = star_script::check_source(&source);

        assert!(checked.is_ok(), "{source}\n{checked:?}");
        assert!(source.contains("class Rock"));
    }

    #[test]
    fn class_names_must_be_valid() {
        assert_eq!(new_blueprint_text("big rock"), None);
        assert_eq!(new_script_text("big rock"), None);
    }
}
