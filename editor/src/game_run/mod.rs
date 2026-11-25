//! “Play”: the player (`star_player`, decision 74) run in the project's
//! folder, where it reads the project and plays it with its StarScript
//! interpreted, in its own window, on a thread of its own so the editor
//! keeps drawing. Cargo first confirms the player is up to date (it is
//! built once, by the warm-up), so a changed engine never plays with an
//! old player.
//!
//! “Play compiled” (decision 75) goes through the same steps with the
//! game's generated Rust, built as a package of its own.
//!
//! The engine's crates are those of the ProjectStar repository the editor
//! was built from, so “Play” works while the editor runs from its sources;
//! shipping the engine with the editor will come with its packaging.

mod compiled;
mod player;
mod process;
mod warm_up;

use std::env;
use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};

use star_script::codegen::GeneratedProgram;

use self::process::{Ended, Events};
pub(crate) use self::warm_up::EngineWarmUp;
use crate::texts;

/// The `crates` folder of the repository, with the runtime and the engine.
const ENGINE_CRATES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../crates");
/// The repository's `Cargo.lock`: the player and compiled games build the
/// same versions as the engine and the editor, so they share one build.
const ENGINE_LOCKFILE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../Cargo.lock");
/// A crate that is always in the engine's folder, to tell that it is there.
const ENGINE_CRATE: &str = "star_engine";
/// Where the player and compiled games are written and built, inside the
/// repository's `target`: one build of the engine serves them all.
const BUILD_DIRECTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../target/star_editor_games");
/// The folder of Cargo's results, inside [`BUILD_DIRECTORY`].
const CARGO_TARGET: &str = "target";
/// Cargo's folder of builds without optimizations.
const DEBUG_PROFILE: &str = "debug";
/// The variable Cargo sets for the programs it runs, the editor included.
const CARGO_VARIABLE: &str = "CARGO";
/// Cargo, when the editor was not started by it.
const DEFAULT_CARGO: &str = "cargo";
/// Name of the thread that builds and runs the game.
const THREAD_NAME: &str = "play";

/// Who wrote a line of the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OutputKind {
    /// Cargo, building the player or the game.
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
    /// Cargo is building the player or the game, or confirming it is up
    /// to date.
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
    /// Cargo could not build the player or the game: its output says why.
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

/// The project to play.
pub(crate) struct PlayRequest {
    /// The project's folder: what is played runs in it, and finds its
    /// files there.
    pub(crate) directory: PathBuf,
    pub(crate) mode: PlayMode,
}

/// How a project is played.
pub(crate) enum PlayMode {
    /// By the player, with its StarScript interpreted: “Play”.
    Interpreted,
    /// Built with the compiler and Cargo, as it ships: “Play compiled”.
    Compiled {
        /// The project's name, which names its package.
        name: String,
        /// The game's Rust, from the project's check.
        program: GeneratedProgram,
    },
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
    if !engine_is_available() {
        return RunEnd::Failed(texts::current().engine_not_found(engine));
    }
    events.send(RunEvent::Phase(RunPhase::Building));
    let (package, program) = match write_package(&request.mode) {
        Ok(written) => written,
        Err(end) => return end,
    };
    let output = Path::new(BUILD_DIRECTORY);
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
    let mut game = Command::new(binary(output, &program));
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

/// Writes the package that plays as `mode` asks: its folder, and the name
/// of its program.
fn write_package(mode: &PlayMode) -> Result<(PathBuf, String), RunEnd> {
    match mode {
        PlayMode::Interpreted => player::write_player()
            .map(|package| (package, player::PLAYER_PACKAGE.to_owned()))
            .map_err(|error| RunEnd::Failed(texts::current().player_not_written(&error))),
        PlayMode::Compiled { name, program } => compiled::write_game(name, program)
            .map_err(|error| RunEnd::Failed(texts::current().game_not_written(&error))),
    }
}

/// Whether the engine's crates are where the player is built from.
fn engine_is_available() -> bool {
    Path::new(ENGINE_CRATES).join(ENGINE_CRATE).is_dir()
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

/// The executable of the package `name`, once built.
fn binary(output: &Path, name: &str) -> PathBuf {
    output
        .join(CARGO_TARGET)
        .join(DEBUG_PROFILE)
        .join(format!("{name}{}", env::consts::EXE_SUFFIX))
}

#[cfg(test)]
mod tests {
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::Duration;

    use super::*;

    /// Long enough for Cargo to answer on a slow machine.
    const TIMEOUT: Duration = Duration::from_secs(60);
    /// Long enough for Cargo to build the engine the first time.
    const BUILD_TIMEOUT: Duration = Duration::from_mins(30);
    /// Set for the copy of the tests that [`never_ends`] keeps running.
    const NEVER_END_VARIABLE: &str = "STAR_EDITOR_TEST_NEVER_END";

    #[test]
    fn the_engine_is_found_in_the_repository() {
        assert!(engine_is_available());
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

    /// Plays a project with `files` (paths inside it) and gives what
    /// happened, until the run finished: interpreted, or compiled from
    /// the Rust its check gives.
    fn play_project(name: &str, files: &[(&str, &str)], compiled: bool) -> Vec<RunEvent> {
        let project = crate::project::TestDirectory::new(name);
        std::fs::create_dir_all(project.path().join("assets")).expect("assets folder");
        for (path, text) in files {
            let path = project.path().join(path);
            std::fs::create_dir_all(path.parent().expect("a folder")).expect("folder");
            std::fs::write(path, text).expect("file");
        }
        let mode = if compiled {
            let check = crate::project_check::check_project_with(
                project.path(),
                &crate::project_check::OpenTexts::new(),
                crate::project_check::CheckGoal::Compile,
            );
            let problems: Vec<String> = check.problems.iter().map(ToString::to_string).collect();
            PlayMode::Compiled {
                name: format!("Editor test {name}"),
                program: check.program.unwrap_or_else(|| panic!("{problems:?}")),
            }
        } else {
            PlayMode::Interpreted
        };
        let request = PlayRequest {
            directory: project.path().to_path_buf(),
            mode,
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
        events
    }

    /// What the game wrote, its standard and error output together.
    fn game_output(events: &[RunEvent]) -> String {
        events
            .iter()
            .filter_map(|event| match event {
                RunEvent::Output(line) if line.kind == OutputKind::Game => Some(line.text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    const CONSOLE_PROJECT: [(&str, &str); 2] = [
        ("project.toml", "[project]\nname = \"Console\"\n"),
        (
            "assets/main.star",
            "class Main {\n static func main() {\n Debug.print(\"hello from the game\")\n }\n}\n",
        ),
    ];

    /// A console program that prints numbers, texts, lists and objects,
    /// with a coroutine that goes on after `main`.
    const BOTH_WAYS_PROJECT: [(&str, &str); 2] = [
        ("project.toml", "[project]\nname = \"Both ways\"\n"),
        (
            "assets/main.star",
            "class Point {\n var x: int\n init(x: int) {\n self.x = x\n }\n}\n\
             class Main {\n static async func later() {\n await Time.next_frame()\n Debug.print(\"later\")\n }\n \
             static func main() {\n const points = [Point(1), Point(2)]\n var total = 0\n \
             for point in points {\n total += point.x\n }\n \
             Debug.print(\"total \" + total.to_string() + \" \" + (7 / 2).to_string() + \" \" + (7.0 / 2.0).to_string())\n \
             later()\n Debug.print(\"main ends\")\n }\n}\n",
        ),
    ];

    #[test]
    fn projects_are_played_by_the_player_in_their_folder() {
        let events = play_project("play_console", &CONSOLE_PROJECT, false);

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
    fn main_scenes_are_played_by_the_player() {
        // Missing, so the player stops at once instead of playing it.
        let events = play_project(
            "play_scene",
            &[(
                "project.toml",
                "[project]\nname = \"Scene\"\nmain_scene = \"scenes/missing.scene\"\n",
            )],
            false,
        );

        let output = game_output(&events);
        assert!(
            output.contains("\"scenes/missing.scene\", could not be loaded"),
            "{output}"
        );
        assert!(matches!(
            events.last(),
            Some(RunEvent::Phase(RunPhase::Finished(RunEnd::Closed {
                success: false,
                ..
            })))
        ));
    }

    #[test]
    fn projects_are_built_and_played_compiled_in_their_folder() {
        let events = play_project("play_compiled", &CONSOLE_PROJECT, true);

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

    /// Rule 3.7 of `CLAUDE.md`, as the editor plays: the same project
    /// prints the same played interpreted and compiled.
    #[test]
    fn projects_play_the_same_interpreted_and_compiled() {
        let interpreted = play_project("both_interpreted", &BOTH_WAYS_PROJECT, false);
        let compiled = play_project("both_compiled", &BOTH_WAYS_PROJECT, true);

        let printed = game_output(&interpreted);
        assert_eq!(printed, "total 3 3 3.5\nmain ends\nlater");
        assert_eq!(game_output(&compiled), printed);
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
