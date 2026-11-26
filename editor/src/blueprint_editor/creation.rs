//! New blueprints: a node that does nothing yet, ready to be added to a
//! scene or to the game.

use star_blueprint::{Blueprint, Graph, Literal, Parameter, is_valid_name};
use star_script::semantic::native_classes;

use super::edit;
use super::library::EVENTS;

/// The class new blueprints extend.
const NEW_BLUEPRINT_BASE: &str = "Node";
/// The events of a node, which new blueprints redefine to fill them in.
pub(super) const START_EVENT: &str = "start";
pub(super) const UPDATE_EVENT: &str = "update";
/// The event of a sprite whose clip ended.
pub(super) const ANIMATION_FINISHED_EVENT: &str = "animation_finished";
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

/// The engine's events that the base of `blueprint` has and it does not
/// redefine yet, as new graphs that redefine them, with their parameters.
/// Only an engine base is known here: a blueprint that extends a script
/// class gets none.
pub(super) fn missing_events(blueprint: &Blueprint) -> Vec<Graph> {
    let classes = native_classes();
    let base = blueprint.extends.as_deref().unwrap_or(NEW_BLUEPRINT_BASE);
    let Some(base) = classes.find(base) else {
        return Vec::new();
    };
    let defined = |name: &str| blueprint.graphs.iter().any(|graph| graph.name == name);
    classes
        .ancestors(base)
        .flat_map(|class| &classes.class(class).methods)
        .filter(|method| EVENTS.contains(&method.name.as_str()) && !defined(&method.name))
        .map(|method| {
            let mut graph = edit::new_graph(method.name.clone());
            graph.is_override = true;
            graph.parameters = method
                .parameters
                .iter()
                .map(|parameter| Parameter {
                    name: parameter.name.clone(),
                    ty: classes.type_name(&parameter.ty),
                })
                .collect();
            graph
        })
        .collect()
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
    fn nodes_have_no_event_left_to_add() {
        assert_eq!(missing_events(&new_blueprint("Rock")), Vec::new());
    }

    #[test]
    fn sprites_can_redefine_the_end_of_their_clips() {
        let mut hero = new_blueprint("Hero");
        hero.extends = Some("Sprite".to_owned());
        hero.base_arguments = vec![
            Literal::String("Hero".to_owned()),
            Literal::String("hero.png".to_owned()),
        ];

        let events = missing_events(&hero);

        let [finished] = &events[..] else {
            panic!("one event expected: {events:?}");
        };
        assert_eq!(finished.name, ANIMATION_FINISHED_EVENT);
        assert!(finished.is_override);
        let parameters: Vec<(&str, &str)> = finished
            .parameters
            .iter()
            .map(|parameter| (parameter.name.as_str(), parameter.ty.as_str()))
            .collect();
        assert_eq!(parameters, [("name", "string")]);
        hero.graphs.extend(events);
        let translation = star_blueprint::translate(&hero).expect("translatable");
        let checked = star_script::check_source(translation.source());
        assert!(checked.is_ok(), "{}\n{checked:?}", translation.source());
        assert_eq!(missing_events(&hero), Vec::new());
    }

    #[test]
    fn deleted_events_can_be_added_again() {
        let mut rock = new_blueprint("Rock");
        rock.graphs.retain(|graph| graph.name != UPDATE_EVENT);

        let events = missing_events(&rock);

        let names: Vec<&str> = events.iter().map(|graph| graph.name.as_str()).collect();
        assert_eq!(names, [UPDATE_EVENT]);
        assert_eq!(events[0].parameters[0].ty, "float");
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
