//! The game's program: its StarScript files and its blueprints, translated
//! to StarScript, checked together as one program.

use std::collections::HashMap;

use star_blueprint::{Blueprint, BlueprintError, Origin, Translation, translate};
use star_script::{CompileError, SourceSet};

use super::Problem;

/// The files of a program, as the StarScript compiler reads them.
pub(super) struct Program {
    sources: SourceSet,
    /// The translation of each blueprint, by the name of its file, to place
    /// the compiler's mistakes back on its nodes.
    translations: HashMap<String, Translation>,
    /// Whether a blueprint could not be read or translated: then the
    /// program is incomplete, and checking it would only report classes
    /// missing because of it.
    incomplete: bool,
}

impl Program {
    pub(super) fn new() -> Self {
        Self {
            sources: SourceSet::new(),
            translations: HashMap::new(),
            incomplete: false,
        }
    }

    pub(super) const fn sources(&self) -> &SourceSet {
        &self.sources
    }

    /// Whether every file could become StarScript.
    pub(super) const fn is_complete(&self) -> bool {
        !self.incomplete
    }

    /// Adds the StarScript file called `name`.
    pub(super) fn add_script(&mut self, name: String, text: String) {
        self.sources.add(name, text);
    }

    /// Adds the blueprint file called `name`, translated to StarScript.
    /// Returns its problems if it cannot be read or translated.
    pub(super) fn add_blueprint(&mut self, name: String, text: &str) -> Vec<Problem> {
        let blueprint = match Blueprint::from_text(text) {
            Ok(blueprint) => blueprint,
            Err(error) => {
                self.incomplete = true;
                return vec![Problem::in_file(name, error.to_string())];
            }
        };
        match translate(&blueprint) {
            Ok(translation) => {
                self.sources.add(name.clone(), translation.source());
                self.translations.insert(name, translation);
                Vec::new()
            }
            Err(errors) => {
                self.incomplete = true;
                errors
                    .iter()
                    .map(|error| {
                        Problem::in_file(name.clone(), error.to_string())
                            .placed_on(error_origin(error))
                    })
                    .collect()
            }
        }
    }

    /// `error` in the file, line and column where it is; in a blueprint, in
    /// the node, variable or graph its code was translated from.
    pub(super) fn problem(&self, error: &CompileError, fallback_file: &str) -> Problem {
        let message = error.to_string();
        let Some(location) = self.sources.locate(error.span()) else {
            return Problem::in_file(fallback_file, message);
        };
        match self.translations.get(location.file) {
            Some(translation) => {
                let origin = translation.origin(location.offset);
                let message = match origin {
                    Some(origin) => format!("{origin}: {message}"),
                    None => message,
                };
                Problem::in_file(location.file, message).placed_on(origin.cloned())
            }
            None => Problem::at(location.file, location.position, message),
        }
    }
}

/// The part of its blueprint a broken rule is in.
fn error_origin(error: &BlueprintError) -> Option<Origin> {
    let graph = error.graph.clone()?;
    Some(match error.node {
        Some(node) => Origin::Node { graph, node },
        None => Origin::Graph(graph),
    })
}
