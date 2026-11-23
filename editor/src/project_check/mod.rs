//! Checking a project's game without building it: its StarScript files and
//! blueprints, the input actions they use and the classes its scenes name.
//!
//! It is what “Compile” runs: everything the StarScript compiler can tell
//! before Cargo builds the generated Rust. When it finds nothing wrong, it
//! keeps that Rust, for “Play” to build.

mod input_actions;
mod job;
mod problem;
mod program;
mod scenes;
mod worker;

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use star_script::codegen::{self, GeneratedProgram};
use star_script::semantic::ClassTable;
use star_script::{
    CompileError, GameSettings, Position, SourceSet, Span, analyze_program_with, check_program_with,
};

pub(crate) use self::job::{CheckJob, CheckStatus};
pub(crate) use self::problem::Problem;
use self::program::Program;
pub(crate) use self::worker::CheckWorker;
use crate::file_tree::{file_paths, scan_directory};
use crate::project::{
    ASSETS_DIRECTORY, BLUEPRINT_EXTENSION, MANIFEST_FILE_NAME, SCENE_EXTENSION, SCRIPT_EXTENSION,
};

/// What checking a project found.
#[derive(Debug, Clone)]
pub(crate) struct ProjectCheck {
    /// How many StarScript files and blueprints were checked.
    pub(crate) scripts: usize,
    /// Every mistake found: in `project.toml` first, then in the scripts and
    /// blueprints, then in the scenes.
    pub(crate) problems: Vec<Problem>,
    /// The Rust code of the game, if there are scripts and no problems.
    pub(crate) program: Option<GeneratedProgram>,
    /// What the program declares, if its files could be read, even with
    /// mistakes in their meaning.
    pub(crate) outline: Option<ProgramOutline>,
}

/// The classes a program declares (with the engine's) and their members,
/// and the texts they were read from, to find where each is written: what
/// suggestions and “Go to definition” need.
#[derive(Debug, Clone)]
pub(crate) struct ProgramOutline {
    pub(crate) classes: ClassTable,
    sources: SourceSet,
}

impl ProgramOutline {
    /// The file (as a path inside the assets folder) and the line and
    /// column where the program's place `span` is.
    pub(crate) fn locate(&self, span: Span) -> Option<(String, Position)> {
        let location = self.sources.locate(span)?;
        let path = location
            .file
            .strip_prefix(ASSETS_DIRECTORY)?
            .strip_prefix('/')?;
        Some((path.to_owned(), location.position))
    }
}

/// The texts of files open in the editor, by their path inside the assets
/// folder: what a check reads instead of their files, which may be older.
pub(crate) type OpenTexts = HashMap<String, String>;

/// What a check is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckGoal {
    /// “Compile”: everything, and the program translated to Rust for
    /// “Play”.
    Compile,
    /// Marking mistakes and suggesting names while scripts are written:
    /// only the program's code is checked, and it is not translated (the
    /// translation only finds constructs Rust cannot have yet, which
    /// “Compile” reports).
    Writing,
}

/// Checks the project in `directory` for `goal`, reading the files of
/// `open` from there instead of from the disk.
///
/// A project without scripts is a game written in Rust: its scenes are not
/// checked against StarScript classes, which such games ignore. If a
/// blueprint cannot become StarScript, only its problems are given for the
/// program: the rest would only miss its class.
pub(crate) fn check_project_with(
    directory: &Path,
    open: &OpenTexts,
    goal: CheckGoal,
) -> ProjectCheck {
    let assets_folder = AssetsFolder { directory, open };
    let mut problems = Vec::new();
    let settings = read_game_settings(directory, &mut problems);
    let assets = list_assets(directory, &mut problems);
    let program = read_program(&assets_folder, &assets, &mut problems);
    let scripts = assets.iter().filter(|path| is_program_file(path)).count();
    let readable = !program.sources().is_empty() && program.is_complete();
    let (checked, generated) = if readable {
        check_program(&program, &settings, goal, &mut problems)
    } else {
        (None, None)
    };
    if scripts > 0 && goal == CheckGoal::Compile {
        let classes = checked.as_ref().map(|checked| checked.analysis.classes());
        for path in assets.iter().filter(|path| path.ends_with(SCENE_EXTENSION)) {
            if let Some(text) = assets_folder.read(path, &mut problems) {
                problems.extend(scenes::check_scene(&asset_name(path), &text, classes));
            }
        }
    }
    let classes = match &checked {
        Some(checked) => Some(checked.analysis.classes().clone()),
        None if readable => analyze_program_with(program.sources(), &settings)
            .map(|analysis| analysis.classes().clone()),
        None => None,
    };
    let outline = classes.map(|classes| ProgramOutline {
        classes,
        sources: program.sources().clone(),
    });
    let program = generated.filter(|_| problems.is_empty());
    ProjectCheck {
        scripts,
        problems,
        program,
        outline,
    }
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
fn read_program(
    folder: &AssetsFolder<'_>,
    assets: &[String],
    problems: &mut Vec<Problem>,
) -> Program {
    let mut program = Program::new();
    for path in assets.iter().filter(|path| is_program_file(path)) {
        let Some(text) = folder.read(path, problems) else {
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

/// The assets folder of the project in `directory`, with the texts open in
/// the editor.
struct AssetsFolder<'check> {
    directory: &'check Path,
    open: &'check OpenTexts,
}

impl AssetsFolder<'_> {
    /// The text of the file at `path` in the assets folder (as it is open
    /// in the editor, if it is), or `None` (and a problem) if it cannot be
    /// read.
    fn read(&self, path: &str, problems: &mut Vec<Problem>) -> Option<String> {
        if let Some(text) = self.open.get(path) {
            return Some(text.clone());
        }
        fs::read_to_string(self.directory.join(ASSETS_DIRECTORY).join(path))
            .map_err(|error| problems.push(Problem::in_file(asset_name(path), error.to_string())))
            .ok()
    }
}

/// How problems name a file of the assets folder: from the project's root.
fn asset_name(path: &str) -> String {
    format!("{ASSETS_DIRECTORY}/{path}")
}

/// Checks the program, and translates it to Rust if `goal` asks for it.
fn check_program(
    program: &Program,
    settings: &GameSettings,
    goal: CheckGoal,
    problems: &mut Vec<Problem>,
) -> (Option<star_script::CheckedSource>, Option<GeneratedProgram>) {
    let report = |errors: Vec<CompileError>| {
        errors
            .iter()
            .map(|error| program.problem(error, ASSETS_DIRECTORY))
            .collect::<Vec<_>>()
    };
    match check_program_with(program.sources(), settings) {
        Ok(checked) if goal == CheckGoal::Writing => (Some(checked), None),
        Ok(checked) => match codegen::generate(&checked.syntax, &checked.analysis) {
            Ok(generated) => (Some(checked), Some(generated)),
            Err(error) => {
                problems.extend(report(vec![CompileError::Codegen(error)]));
                (Some(checked), None)
            }
        },
        Err(errors) => {
            problems.extend(report(errors));
            (None, None)
        }
    }
}

#[cfg(test)]
mod tests;
