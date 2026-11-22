//! “Play”: the game's generated Rust built with Cargo and run in its own
//! window, on a thread of its own so the editor keeps drawing.
//!
//! The engine's crates are those of the ProjectStar repository the editor
//! was built from, so “Play” works while the editor runs from its sources;
//! shipping the engine with the editor will come with its packaging.

mod process;

use std::env;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};

use star_script::codegen::{EntryPoint, GeneratedProgram};
use star_script::package::Package;

use self::process::{Ended, Events};
use crate::texts;

/// The `crates` folder of the repository, with the runtime and the engine.
const ENGINE_CRATES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../crates");
/// A crate that is always in the engine's folder, to tell that it is there.
const ENGINE_CRATE: &str = "star_engine";
/// Where games are written and built, inside the repository's `target`: one
/// build of the engine is shared by every project.
const BUILD_DIRECTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../target/star_editor_games");
/// The folder of Cargo's results, inside [`BUILD_DIRECTORY`].
const CARGO_TARGET: &str = "target";
/// Cargo's folder of builds without optimizations.
const DEBUG_PROFILE: &str = "debug";
/// The variable Cargo sets for the programs it runs, the editor included.
const CARGO_VARIABLE: &str = "CARGO";
/// Cargo, when the editor was not started by it.
const DEFAULT_CARGO: &str = "cargo";
/// Name of the package of a project whose name has no letter to keep.
const FALLBACK_PACKAGE_NAME: &str = "game";
/// Name of the thread that builds and runs the game.
const THREAD_NAME: &str = "play";

/// Who wrote a line of the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputKind {
    /// Cargo, building the game.
    Build,
    /// The game.
    Game,
}

/// One line written while playing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutputLine {
    pub(crate) kind: OutputKind,
    pub(crate) text: String,
}

/// What a run is doing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RunPhase {
    /// Cargo is building the game.
    Building,
    /// The game is running.
    Running,
    /// It is over.
    Finished(RunEnd),
}

/// How a run ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RunEnd {
    /// The game closed; with its exit code if it has one.
    Closed { success: bool, code: Option<i32> },
    /// Cargo could not build the game: its output says why.
    BuildFailed,
    /// The user stopped it.
    Stopped,
    /// It could not go on, for this reason.
    Failed(String),
}

/// Something that happened while playing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RunEvent {
    Output(OutputLine),
    Phase(RunPhase),
}

/// The game of a project, ready to play.
pub(crate) struct PlayRequest {
    /// The project's name, which names its package.
    pub(crate) name: String,
    /// The project's folder: the game runs in it, so it finds `assets/`.
    pub(crate) directory: PathBuf,
    /// The game's Rust code.
    pub(crate) program: GeneratedProgram,
}

/// A game being built or played.
pub(crate) struct GameRun {
    receiver: Receiver<RunEvent>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl GameRun {
    /// Starts building and then playing `request`; `notify` runs each time
    /// something happens (e.g. to redraw the editor).
    ///
    /// # Errors
    ///
    /// Returns the error of the operating system if it refused the thread.
    pub(crate) fn start(
        request: PlayRequest,
        notify: impl Fn() + Send + Sync + 'static,
    ) -> io::Result<Self> {
        let (sender, receiver) = mpsc::channel();
        let events = Events {
            sender,
            notify: Arc::new(notify),
        };
        let stop = Arc::new(AtomicBool::new(false));
        let stop_asked = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name(THREAD_NAME.to_owned())
            .spawn(move || {
                let end = build_and_play(&request, &events, &stop_asked);
                events.send(RunEvent::Phase(RunPhase::Finished(end)));
            })?;
        Ok(Self {
            receiver,
            stop,
            thread: Some(thread),
        })
    }

    /// What happened since the last call, without waiting.
    pub(crate) fn events(&self) -> impl Iterator<Item = RunEvent> + '_ {
        self.receiver.try_iter()
    }

    /// Stops the build or the game; its end arrives as an event.
    pub(crate) fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

impl Drop for GameRun {
    /// A closed project or editor closes its game too, before going.
    fn drop(&mut self) {
        self.stop();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn build_and_play(request: &PlayRequest, events: &Events, stop: &AtomicBool) -> RunEnd {
    let engine = Path::new(ENGINE_CRATES);
    if !engine.join(ENGINE_CRATE).is_dir() {
        return RunEnd::Failed(texts::current().engine_not_found(engine));
    }
    // Without a game or a `main` the crate has no `fn main`, and Cargo's
    // error would not say what is missing.
    if request.program.entry_point == EntryPoint::None {
        return RunEnd::Failed(texts::current().no_game_to_play.to_owned());
    }
    events.send(RunEvent::Phase(RunPhase::Building));
    let name = package_name(&request.name);
    let output = Path::new(BUILD_DIRECTORY);
    let package = output.join(&name);
    if let Err(error) = write_package(&name, &request.program, engine, &package) {
        return RunEnd::Failed(texts::current().game_not_written(&error));
    }
    match process::run(
        build_command(&package, output),
        OutputKind::Build,
        events,
        stop,
    ) {
        Ok(Ended::Exited(status)) if status.success() => {}
        Ok(Ended::Exited(_)) => return RunEnd::BuildFailed,
        Ok(Ended::Stopped) => return RunEnd::Stopped,
        Err(error) => return RunEnd::Failed(texts::current().cargo_not_started(&error)),
    }
    events.send(RunEvent::Phase(RunPhase::Running));
    let mut game = Command::new(binary(output, &name));
    game.current_dir(&request.directory);
    match process::run(game, OutputKind::Game, events, stop) {
        Ok(Ended::Exited(status)) => RunEnd::Closed {
            success: status.success(),
            code: status.code(),
        },
        Ok(Ended::Stopped) => RunEnd::Stopped,
        Err(error) => RunEnd::Failed(texts::current().game_not_started(&error)),
    }
}

fn write_package(
    name: &str,
    program: &GeneratedProgram,
    engine: &Path,
    directory: &Path,
) -> io::Result<()> {
    let mut package = Package::new(name, engine)?;
    package.add_program(name, program.clone())?;
    package.write(directory)
}

/// `cargo build` of the package in `package`, with its results in `output`.
fn build_command(package: &Path, output: &Path) -> Command {
    let cargo = env::var_os(CARGO_VARIABLE).unwrap_or_else(|| OsString::from(DEFAULT_CARGO));
    let mut command = Command::new(cargo);
    command
        .args(["build", "--color", "never", "--message-format", "short"])
        .arg("--manifest-path")
        .arg(package.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", output.join(CARGO_TARGET));
    command
}

/// The game's executable, once built.
fn binary(output: &Path, name: &str) -> PathBuf {
    output
        .join(CARGO_TARGET)
        .join(DEBUG_PROFILE)
        .join(format!("{name}{}", env::consts::EXE_SUFFIX))
}

/// A valid package name from a project's name: `My Game` → `my_game`.
fn package_name(project: &str) -> String {
    let name: String = project
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        name
    } else if name.chars().any(|c| c.is_ascii_alphanumeric()) {
        format!("{FALLBACK_PACKAGE_NAME}_{name}")
    } else {
        FALLBACK_PACKAGE_NAME.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::Duration;

    use super::*;

    /// Long enough for Cargo to answer on a slow machine.
    const TIMEOUT: Duration = Duration::from_secs(60);
    /// Long enough for Cargo to build the runtime the first time.
    const BUILD_TIMEOUT: Duration = Duration::from_secs(600);
    /// Set for the copy of the tests that [`never_ends`] keeps running.
    const NEVER_END_VARIABLE: &str = "STAR_EDITOR_TEST_NEVER_END";

    #[test]
    fn package_names_come_from_the_project_name() {
        assert_eq!(package_name("My Game"), "my_game");
        assert_eq!(package_name("2048"), "game_2048");
        assert_eq!(package_name("¡Ñ!"), "game");
    }

    #[test]
    fn the_engine_is_found_in_the_repository() {
        assert!(Path::new(ENGINE_CRATES).join(ENGINE_CRATE).is_dir());
    }

    #[test]
    fn programs_send_their_output_and_how_they_ended() {
        let (sender, receiver) = mpsc::channel();
        let events = Events {
            sender,
            notify: Arc::new(|| {}),
        };
        let cargo = env::var_os(CARGO_VARIABLE).unwrap_or_else(|| OsString::from(DEFAULT_CARGO));
        let mut command = Command::new(cargo);
        command.arg("--version");

        let ended = process::run(command, OutputKind::Build, &events, &AtomicBool::new(false))
            .expect("cargo runs");

        assert!(matches!(ended, Ended::Exited(status) if status.success()));
        drop(events);
        match receiver.recv_timeout(TIMEOUT) {
            Ok(RunEvent::Output(line)) => {
                assert_eq!(line.kind, OutputKind::Build);
                assert!(line.text.starts_with("cargo "), "{}", line.text);
            }
            other => panic!("expected cargo's version, got {other:?}"),
        }
        assert_eq!(
            receiver.recv_timeout(TIMEOUT),
            Err(RecvTimeoutError::Disconnected)
        );
    }

    #[test]
    fn programs_are_built_and_played_in_their_project() {
        let project = crate::project::TestDirectory::new("play_console");
        std::fs::create_dir_all(project.path()).expect("project folder");
        let program = star_script::compile_source(
            "class Main {\n static func main() {\n Debug.print(\"hello from the game\")\n }\n}\n",
        )
        .expect("valid program");
        let request = PlayRequest {
            name: "Editor test console".to_owned(),
            directory: project.path().to_path_buf(),
            program,
        };

        let run = GameRun::start(request, || {}).expect("thread started");

        let mut events = Vec::new();
        while let Ok(event) = run.receiver.recv_timeout(BUILD_TIMEOUT) {
            let finished = matches!(event, RunEvent::Phase(RunPhase::Finished(_)));
            events.push(event);
            if finished {
                break;
            }
        }
        let printed = RunEvent::Output(OutputLine {
            kind: OutputKind::Game,
            text: "hello from the game".to_owned(),
        });
        assert!(events.contains(&printed), "{events:#?}");
        assert_eq!(
            events.last(),
            Some(&RunEvent::Phase(RunPhase::Finished(RunEnd::Closed {
                success: true,
                code: Some(0)
            })))
        );
    }

    #[test]
    fn programs_without_a_game_are_not_built() {
        let program = star_script::compile_source(
            "class Rock extends Node {\n init() { super.init(\"Rock\") }\n}\n",
        )
        .expect("valid program");
        let request = PlayRequest {
            name: "Editor test no game".to_owned(),
            directory: std::env::temp_dir(),
            program,
        };

        let run = GameRun::start(request, || {}).expect("thread started");

        assert_eq!(
            run.receiver.recv_timeout(TIMEOUT),
            Ok(RunEvent::Phase(RunPhase::Finished(RunEnd::Failed(
                texts::current().no_game_to_play.to_owned()
            ))))
        );
    }

    #[test]
    fn a_stopped_program_says_so() {
        let (sender, _receiver) = mpsc::channel();
        let events = Events {
            sender,
            notify: Arc::new(|| {}),
        };
        // The tests themselves, running `never_ends` alone, would run
        // forever: only a stop ends them.
        let mut command = Command::new(env::current_exe().expect("test binary"));
        command
            .args(["--exact", "game_run::tests::never_ends"])
            .env(NEVER_END_VARIABLE, "1");

        let ended = process::run(command, OutputKind::Game, &events, &AtomicBool::new(true))
            .expect("the program starts");

        assert!(matches!(ended, Ended::Stopped));
    }

    /// Runs forever in the copy started by `a_stopped_program_says_so`;
    /// passes at once anywhere else.
    #[test]
    fn never_ends() {
        while env::var_os(NEVER_END_VARIABLE).is_some() {
            thread::sleep(Duration::from_secs(1));
        }
    }
}
