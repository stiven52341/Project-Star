//! The node game of `star_script`'s `engine_game` test, interpreted in a
//! real window with its scenes (decisions 70 and 71): the same program
//! and scenes, expecting the same output as the compiled game.
//!
//! It needs a screen and a GPU (or a software renderer). The engine's
//! window can be opened once per process, so this game has a test of its
//! own beside `window`; it plays only when asked:
//! `cargo test -p star_script_interpreter --test window_scene -- --ignored`.

// Test code: a failed step should stop the test with its explanation.
#![allow(clippy::expect_used, clippy::panic)]

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use star_engine::app::App;
use star_engine::star_core::EngineError;
use star_script::{GameSettings, InputAction, check_source_with};
use star_script_interpreter::host::Interpreted;
use star_script_interpreter::{Output, Program, end_game};

/// The node game of `engine_game.rs`, and the scenes it loads.
const NODE_GAME: &str = include_str!("../../star_script/tests/games/node_game.star");
const FLEET_SCENE: &str = include_str!("../../star_script/tests/games/fleet.scene");
const MISSPELLED_SCENE: &str = include_str!("../../star_script/tests/games/misspelled.scene");

/// What the compiled game prints, as `engine_game.rs` expects it.
const EXPECTED: &str = "Fleet 4\n2 true\nsprites/leader.png 1.5 0.5\nspot 4.0 0.5\ntrue\ntrue\n\
     game frame 1\nship start\nship x 1.0\ndrone start Wingman -2.0 4.0 wing 0.25\n\
     game frame 2\ncamera Lookout 6.0\nship x 2.0\n\
     game frame 3\ncamera Lookout 6.0\nship x 3.0\n";

/// Writes the scenes the game loads into an asset folder of its own.
fn asset_folder() -> &'static Path {
    let folder = Path::new(env!("CARGO_TARGET_TMPDIR")).join("interpreted_node_game");
    let levels = folder.join("levels");
    std::fs::create_dir_all(&levels).expect("asset folder created");
    std::fs::write(levels.join("fleet.scene"), FLEET_SCENE).expect("scene written");
    std::fs::write(levels.join("misspelled.scene"), MISSPELLED_SCENE).expect("scene written");
    Box::leak(folder.into_boxed_path())
}

fn main() {
    if !std::env::args().any(|argument| argument == "--ignored") {
        println!("window_scene: ignored (pass `-- --ignored` to open a window)");
        return;
    }
    let settings = GameSettings {
        actions: vec![InputAction {
            name: "jump".to_owned(),
            bindings: vec!["Space".to_owned()],
        }],
        main_scene: None,
    };
    let checked = check_source_with(NODE_GAME, &settings).expect("a valid game");
    let program = Program::lower(&checked).expect("a game the interpreter runs");
    let buffer = Rc::new(RefCell::new(Vec::new()));
    let output: Output = Rc::clone(&buffer) as Output;
    let actions = ["jump"];
    star_script_runtime::input::declare_actions(&actions);
    let app = App::new()
        .with_title("Interpreted node game")
        .with_asset_folder(asset_folder());
    star_script_engine::game::run::<Interpreted>(app, &actions, || {
        program
            .new_game(&output)
            .map_err(|error| EngineError::Initialization {
                subsystem: "interpreter",
                reason: error.to_string(),
            })
    })
    .expect("the game plays in a window");
    end_game().expect("a game that ends without errors");
    let printed = String::from_utf8(buffer.take()).expect("UTF-8 output");
    assert_eq!(printed, EXPECTED);
    println!("window_scene: ok");
}
