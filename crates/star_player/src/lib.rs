//! The player of ProjectStar: plays a project with its StarScript
//! interpreted, so “Play” in the editor starts any project at once
//! (decision 74).
//!
//! It reads the project from its folder ([`project::Project`]), checks
//! its program with `star_script`, prepares it with
//! `star_script_interpreter` and plays it in a window of the engine
//! through `star_script_engine`, as a compiled game is played: the same
//! window title, input actions, assets folder, output and runtime errors.
//!
//! The editor builds a program whose `main` is [`main`], outside its own
//! workspace (decision 63), and runs it in the project's folder.

pub mod project;
mod settings;

use std::cell::RefCell;
use std::fmt;
use std::path::Path;
use std::process::ExitCode;
use std::rc::Rc;

use star_engine::app::App;
use star_engine::star_core::{EngineError, EngineResult};
use star_engine::star_input::{ActionMap, Binding};
use star_script::InputAction;
use star_script_interpreter::host::Interpreted;
use star_script_interpreter::{Output, Program, ProgramKind, RunError, end_game};

use crate::project::{Problems, Project};

/// Why a project could not be played to its end.
#[derive(Debug)]
pub enum PlayError {
    /// The folder the player runs in could not be found.
    Directory(std::io::Error),
    /// The project has mistakes, so it cannot start.
    Project(Problems),
    /// The engine could not play it, as a compiled game could not: a
    /// binding it does not know, a window it cannot open…
    Engine(EngineError),
    /// It stopped with a runtime error of StarScript, as compiled.
    Run(RunError),
}

impl fmt::Display for PlayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Directory(error) => write!(formatter, "the project's folder is unknown: {error}"),
            Self::Project(problems) => problems.fmt(formatter),
            Self::Engine(error) => error.fmt(formatter),
            Self::Run(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for PlayError {}

impl From<Problems> for PlayError {
    fn from(problems: Problems) -> Self {
        Self::Project(problems)
    }
}

impl From<EngineError> for PlayError {
    fn from(error: EngineError) -> Self {
        Self::Engine(error)
    }
}

/// Plays the project in the current folder, as the editor runs the player:
/// why it stopped goes to the error output, and the program ends with
/// failure then, as a compiled game does.
pub fn main() -> ExitCode {
    let played = std::env::current_dir()
        .map_err(PlayError::Directory)
        .and_then(|directory| play(&directory));
    match played {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

/// Plays the project in `directory` until it ends: a game in a window, or a
/// console program on the standard output. `Debug.print` writes to the
/// standard output.
///
/// # Errors
///
/// The project's mistakes, if it cannot start; why the engine could not
/// play it; or the runtime error it stopped with.
pub fn play(directory: &Path) -> Result<(), PlayError> {
    let project = Project::read(directory)?;
    let program = project.program()?;
    let output: Output = Rc::new(RefCell::new(std::io::stdout()));
    match program.kind() {
        ProgramKind::Console => program.run(&output).map_err(PlayError::Run),
        ProgramKind::Game => play_game(&project, &program, &output),
    }
}

/// Plays the game `program` of `project` in a window of the engine.
fn play_game(project: &Project, program: &Program, output: &Output) -> Result<(), PlayError> {
    let actions = &project.settings().actions;
    let action_map = action_map(actions)?;
    let names: Vec<&str> = actions.iter().map(|action| action.name.as_str()).collect();
    star_script_runtime::input::declare_actions(&names);
    let app = App::new()
        .with_title(program.title().unwrap_or_default())
        .with_actions(action_map)
        .with_asset_folder(project.assets_folder());
    // A runtime error while the game is created is the game's, not the
    // engine's: it is printed as a compiled game prints it.
    let mut failed_start = None;
    let played = star_script_engine::game::run::<Interpreted>(app, &names, || {
        program.new_game(output).map_err(|error| {
            let reason = error.to_string();
            failed_start = Some(error);
            EngineError::Initialization {
                subsystem: "player",
                reason,
            }
        })
    });
    if let Some(error) = failed_start {
        return Err(PlayError::Run(error));
    }
    played?;
    end_game().map_err(PlayError::Run)
}

/// The project's input actions, as the engine reads them; a binding it
/// does not know stops the game, as compiled.
fn action_map(actions: &[InputAction]) -> EngineResult<ActionMap> {
    let mut map = ActionMap::new();
    for action in actions {
        let bindings = action
            .bindings
            .iter()
            .map(|binding| binding.parse::<Binding>())
            .collect::<EngineResult<Vec<_>>>()?;
        map.add(&action.name, &bindings)?;
    }
    Ok(map)
}
