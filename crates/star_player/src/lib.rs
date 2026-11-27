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
//! While a game plays, the scripts and blueprints saved meanwhile are
//! reloaded between two frames, keeping the game's objects (hot reload,
//! decision 76).
//!
//! The editor builds a program whose `main` is [`main`], outside its own
//! workspace (decision 63), and runs it in the project's folder.

pub mod project;
mod settings;
mod watch;

use std::cell::RefCell;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::rc::Rc;

use star_engine::app::{App, played_from_editor};
use star_engine::star_core::{EngineError, EngineResult};
use star_engine::star_input::{ActionMap, Binding};
use star_script::{InputAction, ShowFps};
use star_script_interpreter::host::Interpreted;
use star_script_interpreter::{Output, Program, ProgramKind, RunError, end_game, reload};

use crate::project::{Problems, Project};
use crate::watch::Scripts;

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
        ProgramKind::Game => play_game(directory, &project, &program, &output),
    }
}

/// Plays the game `program` of `project`, in `directory`, in a window of
/// the engine, reloading its scripts when they are saved.
fn play_game(
    directory: &Path,
    project: &Project,
    program: &Program,
    output: &Output,
) -> Result<(), PlayError> {
    let actions = &project.settings().actions;
    let action_map = action_map(actions)?;
    let names: Vec<&str> = actions.iter().map(|action| action.name.as_str()).collect();
    star_script_runtime::input::declare_actions(&names);
    let fps_shown = match project.settings().show_fps {
        ShowFps::Editor => played_from_editor(),
        ShowFps::Always => true,
        ShowFps::Never => false,
    };
    let app = App::new()
        .with_title(program.title().unwrap_or_default())
        .with_actions(action_map)
        .with_asset_folder(project.assets_folder())
        .with_fps_shown(fps_shown)
        .with_sound_groups(project.settings().sound_groups.clone());
    // A runtime error while the game is created is the game's, not the
    // engine's: it is printed as a compiled game prints it.
    let mut failed_start = None;
    let between_frames = reloader(directory.to_owned(), project.assets_folder());
    let played = star_script_engine::game::run_with_step::<Interpreted>(
        app,
        &names,
        || {
            program.new_game(output).map_err(|error| {
                let reason = error.to_string();
                failed_start = Some(error);
                EngineError::Initialization {
                    subsystem: "player",
                    reason,
                }
            })
        },
        between_frames,
    );
    if let Some(error) = failed_start {
        return Err(PlayError::Run(error));
    }
    played?;
    end_game().map_err(PlayError::Run)
}

/// What runs between two frames of a game: when its scripts or blueprints
/// changed, it reloads the project in `directory` and says how it went on
/// the error output (the standard output is the game's own).
fn reloader(directory: PathBuf, assets: PathBuf) -> impl FnMut(f32) + 'static {
    let mut scripts = Scripts::new(assets);
    move |seconds| {
        if scripts.changed(seconds) {
            eprintln!("{}", reload_project(&directory));
        }
    }
}

/// Reloads the program of the project in `directory` into the game this
/// thread plays: what happened, to be told to whoever plays it.
fn reload_project(directory: &Path) -> String {
    let program = match Project::read(directory).and_then(|project| project.program()) {
        Ok(program) => program,
        Err(problems) => {
            return format!(
                "{problems}\nthe scripts were not reloaded: the game goes on with the code it had"
            );
        }
    };
    match reload(&program) {
        Ok(reloaded) if reloaded.coroutines_ended > 0 => format!(
            "scripts reloaded: {} objects kept, {} coroutines ended (play again to start them anew)",
            reloaded.objects, reloaded.coroutines_ended
        ),
        Ok(reloaded) => format!("scripts reloaded: {} objects kept", reloaded.objects),
        Err(error) => format!(
            "the scripts were not reloaded: {error}; the game goes on with the code it had (play again to use the new code)"
        ),
    }
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

#[cfg(test)]
mod tests;
