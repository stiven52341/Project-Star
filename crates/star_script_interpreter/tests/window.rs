//! Every game of `star_script`'s `engine_game` test, interpreted in a real
//! window of the engine through `star_script_engine`, as compiled games
//! are played (decisions 70 to 72): the same programs and scenes, from
//! `star_script/tests/games`, expecting the same output.
//!
//! It needs a screen and a GPU (or a software renderer), and the window
//! needs the main thread, so it has no test harness and only plays when
//! asked: `cargo test -p star_script_interpreter --test window -- --ignored`.
//! The engine's window can be opened once per process, so the test plays
//! each game in a child process of itself (`window --play <game>`), one
//! after another.

// Test code: a failed step should stop the test with its explanation.
#![allow(clippy::expect_used, clippy::panic)]

use std::cell::RefCell;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::rc::Rc;

use star_engine::app::App;
use star_engine::star_core::{EngineError, EngineResult};
use star_engine::star_input::{ActionMap, Binding};
use star_script::{GameSettings, InputAction, MainScene, check_source_with};
use star_script_interpreter::host::Interpreted;
use star_script_interpreter::{Output, Program, end_game};

/// The folder of the games shared with `engine_game`.
macro_rules! game_file {
    ($name:literal) => {
        include_str!(concat!("../../star_script/tests/games/", $name))
    };
}

/// The bindings of `jump` that the engine knows.
const BINDINGS: [&str; 4] = ["Space", "MouseLeft", "Touch", "GamepadSouth"];

/// How a game must end.
enum Ending {
    /// It ends well, having printed this.
    Prints(&'static str),
    /// It stops with an error that says this.
    Fails(&'static str),
}

/// A game of `engine_game`, as it is played there.
struct Game {
    name: &'static str,
    source: &'static str,
    /// The scenes of its `levels` folder: their names and contents.
    scenes: &'static [(&'static str, &'static str)],
    /// The bindings of its action `jump`.
    bindings: &'static [&'static str],
    /// The main scene of its project, if it has one.
    main_scene: Option<&'static str>,
    ending: Ending,
    /// What its error output must say besides, such as a warning.
    warns: Option<&'static str>,
}

const COUNTING_GAME: &str = game_file!("counting_game.star");

const GAMES: [Game; 11] = [
    Game {
        name: "counting_game",
        source: COUNTING_GAME,
        scenes: &[],
        bindings: &BINDINGS,
        main_scene: None,
        ending: Ending::Prints(game_file!("counting_game.out")),
        warns: None,
    },
    Game {
        name: "misspelled_game",
        source: COUNTING_GAME,
        scenes: &[],
        bindings: &["Spcae"],
        main_scene: None,
        ending: Ending::Fails("unknown input binding \"Spcae\""),
        warns: None,
    },
    Game {
        name: "node_game",
        source: game_file!("node_game.star"),
        scenes: &[
            ("fleet.scene", game_file!("fleet.scene")),
            ("misspelled.scene", game_file!("misspelled.scene")),
        ],
        bindings: &BINDINGS,
        main_scene: None,
        ending: Ending::Prints(game_file!("node_game.out")),
        warns: Some("node \"Drone\": there is no class \"Dron\""),
    },
    Game {
        name: "physics_game",
        source: game_file!("physics_game.star"),
        scenes: &[("floor.scene", game_file!("floor.scene"))],
        bindings: &BINDINGS,
        main_scene: None,
        ending: Ending::Prints(game_file!("physics_game.out")),
        warns: None,
    },
    Game {
        name: "physics_3d_game",
        source: game_file!("physics_3d_game.star"),
        scenes: &[("floor_3d.scene", game_file!("floor_3d.scene"))],
        bindings: &BINDINGS,
        main_scene: None,
        ending: Ending::Prints(game_file!("physics_3d_game.out")),
        warns: None,
    },
    Game {
        name: "coroutine_game",
        source: game_file!("coroutine_game.star"),
        scenes: &[],
        bindings: &BINDINGS,
        main_scene: None,
        ending: Ending::Prints(game_file!("coroutine_game.out")),
        warns: None,
    },
    Game {
        name: "animation_game",
        source: game_file!("animation_game.star"),
        scenes: &[("hero.frames", game_file!("hero.frames"))],
        bindings: &BINDINGS,
        main_scene: None,
        ending: Ending::Prints(game_file!("animation_game.out")),
        warns: None,
    },
    Game {
        name: "animator_game",
        source: game_file!("animator_game.star"),
        scenes: &[
            ("knight.scene", game_file!("knight.scene")),
            ("knight.anim", game_file!("knight.anim")),
        ],
        bindings: &BINDINGS,
        main_scene: None,
        ending: Ending::Prints(game_file!("animator_game.out")),
        warns: None,
    },
    Game {
        name: "tween_game",
        source: game_file!("tween_game.star"),
        scenes: &[],
        bindings: &BINDINGS,
        main_scene: None,
        ending: Ending::Prints(game_file!("tween_game.out")),
        warns: None,
    },
    Game {
        name: "states_game",
        source: game_file!("states_game.star"),
        scenes: &[
            ("guard.scene", game_file!("guard.scene")),
            ("guard.anim", game_file!("guard.anim")),
            ("guard.states", game_file!("guard.states")),
        ],
        bindings: &BINDINGS,
        main_scene: None,
        ending: Ending::Prints(game_file!("states_game.out")),
        warns: None,
    },
    // Never quits by itself: played only with a scene it cannot load.
    Game {
        name: "scene_game",
        source: "",
        scenes: &[],
        bindings: &BINDINGS,
        main_scene: Some("levels/nowhere.scene"),
        ending: Ending::Fails("the main scene, \"levels/nowhere.scene\", could not be loaded"),
        warns: None,
    },
];

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().collect();
    if let Some(position) = arguments.iter().position(|argument| argument == "--play") {
        let name = arguments.get(position + 1).expect("the name of a game");
        let game = GAMES
            .iter()
            .find(|game| game.name == name)
            .expect("a game of the test");
        return match play(game) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("{error}");
                ExitCode::FAILURE
            }
        };
    }
    if !arguments.iter().any(|argument| argument == "--ignored") {
        println!("window: ignored (pass `-- --ignored` to open a window)");
        return ExitCode::SUCCESS;
    }
    let failures: Vec<String> = GAMES.iter().filter_map(|game| check(game).err()).collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
    println!("window: ok ({} games)", GAMES.len());
    ExitCode::SUCCESS
}

/// Plays `game` in a child process and checks how it ended.
fn check(game: &Game) -> Result<(), String> {
    let executable = std::env::current_exe().expect("the test's own executable");
    let output = Command::new(executable)
        .args(["--play", game.name])
        .output()
        .expect("the child process runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let name = game.name;
    match (&game.ending, output.status.success()) {
        (Ending::Prints(expected), true) if stdout == *expected => {}
        (Ending::Prints(expected), true) => {
            return Err(format!(
                "{name}: unexpected output\n--- expected\n{expected}--- found\n{stdout}"
            ));
        }
        (Ending::Prints(_), false) => return Err(format!("{name} failed:\n{stderr}")),
        (Ending::Fails(reason), false) if stderr.contains(reason) => {}
        (Ending::Fails(reason), _) => {
            return Err(format!(
                "{name}: should have stopped saying `{reason}`:\n{stderr}"
            ));
        }
    }
    match game.warns {
        Some(warning) if !stderr.contains(warning) => {
            Err(format!("{name}: should have warned `{warning}`:\n{stderr}"))
        }
        _ => Ok(()),
    }
}

/// Plays `game` in a window of this process, as its compiled program does:
/// `Debug.print` writes to the standard output.
fn play(game: &Game) -> EngineResult<()> {
    let settings = GameSettings {
        actions: vec![InputAction {
            name: "jump".to_owned(),
            bindings: game
                .bindings
                .iter()
                .map(|&binding| binding.to_owned())
                .collect(),
        }],
        main_scene: game.main_scene.map(|path| MainScene {
            path: path.to_owned(),
            title: game.name.to_owned(),
        }),
    };
    let checked = check_source_with(game.source, &settings).expect("a valid game");
    let program = Program::lower(&checked).expect("a game the interpreter runs");
    let action_map = action_map(&settings.actions)?;
    let actions = ["jump"];
    star_script_runtime::input::declare_actions(&actions);
    let output: Output = Rc::new(RefCell::new(std::io::stdout()));
    let app = App::new()
        .with_title(game.name)
        .with_actions(action_map)
        .with_asset_folder(asset_folder(game));
    star_script_engine::game::run::<Interpreted>(app, &actions, || {
        program
            .new_game(&output)
            .map_err(|error| EngineError::Initialization {
                subsystem: "interpreter",
                reason: error.to_string(),
            })
    })?;
    end_game().map_err(|error| EngineError::Initialization {
        subsystem: "interpreter",
        reason: error.to_string(),
    })
}

/// The actions of the project, as the engine reads them; a binding it does
/// not know stops the game, as compiled.
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

/// An asset folder of its own for `game`, with its scenes (and other
/// files, such as `.frames`) in `levels`.
fn asset_folder(game: &Game) -> PathBuf {
    let folder = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("interpreted_games")
        .join(game.name);
    let levels = folder.join("levels");
    std::fs::create_dir_all(&levels).expect("asset folder created");
    for (name, contents) in game.scenes {
        std::fs::write(levels.join(name), contents).expect("scene written");
    }
    folder
}
