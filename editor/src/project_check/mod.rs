//! Checking a project's game without building it: its StarScript files and
//! blueprints, the input actions they use and the classes its scenes name.
//!
//! It is what «Compilar» runs: everything the StarScript compiler can tell
//! before Cargo builds the generated Rust.

mod input_actions;
mod job;
mod problem;
mod program;
mod scenes;

use std::fs;
use std::path::Path;

use star_script::codegen;
use star_script::{CompileError, GameSettings, check_program_with};

pub(crate) use self::job::{CheckJob, CheckStatus};
pub(crate) use self::problem::Problem;
use self::program::Program;
use crate::file_tree::{file_paths, scan_directory};
use crate::project::{
    ASSETS_DIRECTORY, BLUEPRINT_EXTENSION, MANIFEST_FILE_NAME, SCENE_EXTENSION, SCRIPT_EXTENSION,
};

/// What checking a project found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProjectCheck {
    /// How many StarScript files and blueprints were checked.
    pub(crate) scripts: usize,
    /// Every mistake found: in `project.toml` first, then in the scripts and
    /// blueprints, then in the scenes.
    pub(crate) problems: Vec<Problem>,
}

/// Checks the project in `directory`.
///
/// A project without scripts is a game written in Rust: its scenes are not
/// checked against StarScript classes, which such games ignore. If a
/// blueprint cannot become StarScript, only its problems are given for the
/// program: the rest would only miss its class.
pub(crate) fn check_project(directory: &Path) -> ProjectCheck {
    let mut problems = Vec::new();
    let settings = read_game_settings(directory, &mut problems);
    let assets = list_assets(directory, &mut problems);
    let program = read_program(directory, &assets, &mut problems);
    let scripts = assets.iter().filter(|path| is_program_file(path)).count();
    let checked = if program.sources().is_empty() || !program.is_complete() {
        None
    } else {
        check_program(&program, &settings, &mut problems)
    };
    if scripts > 0 {
        let classes = checked.as_ref().map(|checked| checked.analysis.classes());
        for path in assets.iter().filter(|path| path.ends_with(SCENE_EXTENSION)) {
            if let Some(text) = read_asset(directory, path, &mut problems) {
                problems.extend(scenes::check_scene(&asset_name(path), &text, classes));
            }
        }
    }
    ProjectCheck { scripts, problems }
}

/// The input actions of `project.toml`; none if it cannot be read.
fn read_game_settings(directory: &Path, problems: &mut Vec<Problem>) -> GameSettings {
    fs::read_to_string(directory.join(MANIFEST_FILE_NAME))
        .map_err(|error| error.to_string())
        .and_then(|text| input_actions::read_game_settings(&text))
        .unwrap_or_else(|message| {
            problems.push(Problem::in_file(MANIFEST_FILE_NAME, message));
            GameSettings::default()
        })
}

/// Every file of the assets folder, as paths inside it.
fn list_assets(directory: &Path, problems: &mut Vec<Problem>) -> Vec<String> {
    match scan_directory(&directory.join(ASSETS_DIRECTORY)) {
        Ok(nodes) => file_paths(&nodes),
        Err(error) => {
            problems.push(Problem::in_file(ASSETS_DIRECTORY, error.to_string()));
            Vec::new()
        }
    }
}

/// Whether the asset at `path` is part of the program.
fn is_program_file(path: &str) -> bool {
    path.ends_with(SCRIPT_EXTENSION) || path.ends_with(BLUEPRINT_EXTENSION)
}

/// The StarScript files and blueprints among `assets`, named as the
/// problems name them.
fn read_program(directory: &Path, assets: &[String], problems: &mut Vec<Problem>) -> Program {
    let mut program = Program::new();
    for path in assets.iter().filter(|path| is_program_file(path)) {
        let Some(text) = read_asset(directory, path, problems) else {
            continue;
        };
        if path.ends_with(BLUEPRINT_EXTENSION) {
            problems.extend(program.add_blueprint(asset_name(path), &text));
        } else {
            program.add_script(asset_name(path), text);
        }
    }
    program
}

/// The text of the file at `path` in the assets folder, or `None` (and a
/// problem) if it cannot be read.
fn read_asset(directory: &Path, path: &str, problems: &mut Vec<Problem>) -> Option<String> {
    fs::read_to_string(directory.join(ASSETS_DIRECTORY).join(path))
        .map_err(|error| problems.push(Problem::in_file(asset_name(path), error.to_string())))
        .ok()
}

/// How problems name a file of the assets folder: from the project's root.
fn asset_name(path: &str) -> String {
    format!("{ASSETS_DIRECTORY}/{path}")
}

/// Checks the program, and that it can be translated to Rust.
fn check_program(
    program: &Program,
    settings: &GameSettings,
    problems: &mut Vec<Problem>,
) -> Option<star_script::CheckedSource> {
    let report = |errors: Vec<CompileError>| {
        errors
            .iter()
            .map(|error| program.problem(error, ASSETS_DIRECTORY))
            .collect::<Vec<_>>()
    };
    match check_program_with(program.sources(), settings) {
        Ok(checked) => {
            if let Err(error) = codegen::generate(&checked.syntax, &checked.analysis) {
                problems.extend(report(vec![CompileError::Codegen(error)]));
            }
            Some(checked)
        }
        Err(errors) => {
            problems.extend(report(errors));
            None
        }
    }
}

#[cfg(test)]
mod tests;
