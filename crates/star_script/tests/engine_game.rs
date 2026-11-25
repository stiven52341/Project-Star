//! A StarScript game run in a real window of the engine.
//!
//! Ignored by default: it builds the whole engine (wgpu included) and needs
//! a screen and a GPU (or a software renderer). Run it with
//! `cargo test -p star_script --test engine_game -- --ignored`.

// Test code: a failed step should stop the test with its explanation.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::path::Path;
use std::process::Command;

use common::{ENGINE_CRATES, binary, build, compile_with, package_directory};
use star_script::package::Package;
use star_script::{GameSettings, InputAction, MainScene};

/// The games, their scenes and what they print, in `games/`: the
/// interpreter's `window` test plays the same files and expects the same
/// output (rule 3.7 of `CLAUDE.md`).
const GAME: &str = include_str!("games/counting_game.star");
const NODE_GAME: &str = include_str!("games/node_game.star");
const PHYSICS_GAME: &str = include_str!("games/physics_game.star");
const PHYSICS_3D_GAME: &str = include_str!("games/physics_3d_game.star");
const COROUTINE_GAME: &str = include_str!("games/coroutine_game.star");
const FLOOR_SCENE: &str = include_str!("games/floor.scene");
const FLOOR_3D_SCENE: &str = include_str!("games/floor_3d.scene");
const FLEET_SCENE: &str = include_str!("games/fleet.scene");
const MISSPELLED_SCENE: &str = include_str!("games/misspelled.scene");

fn settings(bindings: &[&str]) -> GameSettings {
    GameSettings {
        actions: vec![InputAction {
            name: "jump".to_owned(),
            bindings: bindings.iter().map(|&binding| binding.to_owned()).collect(),
        }],
        main_scene: None,
    }
}

/// Runs the game `name` of the package in `directory`, from that folder,
/// and gives what it wrote, once it ended well.
fn play(directory: &Path, name: &str) -> (String, String) {
    let output = Command::new(binary(directory, name))
        .current_dir(directory)
        .output()
        .expect("the game runs");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "{name} failed:\n{stderr}");
    (String::from_utf8_lossy(&output.stdout).into_owned(), stderr)
}

#[test]
#[ignore = "builds the whole engine and opens a window"]
fn games_run_in_the_engine_until_they_quit() {
    let directory = package_directory("star_script_engine_game");
    let mut package = Package::new("engine_game", Path::new(ENGINE_CRATES)).expect("valid name");
    let valid = settings(&["Space", "MouseLeft", "Touch", "GamepadSouth"]);
    package
        .add_program("counting_game", compile_with("counting_game", GAME, &valid))
        .expect("valid name");
    let misspelled = settings(&["Spcae"]);
    package
        .add_program(
            "misspelled_game",
            compile_with("misspelled_game", GAME, &misspelled),
        )
        .expect("valid name");
    package
        .add_program("node_game", compile_with("node_game", NODE_GAME, &valid))
        .expect("valid name");
    package
        .add_program(
            "physics_game",
            compile_with("physics_game", PHYSICS_GAME, &valid),
        )
        .expect("valid name");
    package
        .add_program(
            "physics_3d_game",
            compile_with("physics_3d_game", PHYSICS_3D_GAME, &valid),
        )
        .expect("valid name");
    package
        .add_program(
            "coroutine_game",
            compile_with("coroutine_game", COROUTINE_GAME, &valid),
        )
        .expect("valid name");
    // Never quits by itself: played only with a scene it cannot load.
    let scene_only = GameSettings {
        main_scene: Some(MainScene {
            path: "levels/nowhere.scene".to_owned(),
            title: "Scene only".to_owned(),
        }),
        ..valid.clone()
    };
    package
        .add_program("scene_game", compile_with("scene_game", "", &scene_only))
        .expect("valid name");
    package
        .add_program(
            "scene_player",
            star_script::player_program().expect("valid player"),
        )
        .expect("valid name");
    package.write(&directory).expect("package written");

    build(&directory);
    let (stdout, _) = play(&directory, "counting_game");
    assert_eq!(stdout, include_str!("games/counting_game.out"));

    let levels = directory.join("assets/levels");
    std::fs::create_dir_all(&levels).expect("asset folder created");
    std::fs::write(levels.join("fleet.scene"), FLEET_SCENE).expect("scene written");
    std::fs::write(levels.join("misspelled.scene"), MISSPELLED_SCENE).expect("scene written");
    let (stdout, stderr) = play(&directory, "node_game");
    assert!(
        stderr.contains("node \"Drone\": there is no class \"Dron\""),
        "the misspelled class is reported:\n{stderr}"
    );
    assert_eq!(stdout, include_str!("games/node_game.out"));

    std::fs::write(levels.join("floor.scene"), FLOOR_SCENE).expect("scene written");
    let (stdout, _) = play(&directory, "physics_game");
    assert_eq!(stdout, include_str!("games/physics_game.out"));

    std::fs::write(levels.join("floor_3d.scene"), FLOOR_3D_SCENE).expect("scene written");
    let (stdout, _) = play(&directory, "physics_3d_game");
    assert_eq!(stdout, include_str!("games/physics_3d_game.out"));

    let (stdout, _) = play(&directory, "coroutine_game");
    // The blinker's coroutine ends with frame 3, when it leaves the tree.
    assert_eq!(stdout, include_str!("games/coroutine_game.out"));

    scene_only_games_need_their_scene(&directory);

    let output = Command::new(binary(&directory, "misspelled_game"))
        .output()
        .expect("the game runs");
    assert!(!output.status.success(), "a wrong binding stops the game");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown input binding \"Spcae\""),
        "the error names the binding"
    );
}

/// The scene-only game and the scene player stop, saying which scene they
/// could not load (a scene they load would play until closed).
fn scene_only_games_need_their_scene(directory: &Path) {
    let output = Command::new(binary(directory, "scene_game"))
        .current_dir(directory)
        .output()
        .expect("the game runs");
    assert!(
        !output.status.success(),
        "a missing main scene stops the game"
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("the main scene, \"levels/nowhere.scene\", could not be loaded"),
        "the error names the scene"
    );

    let without_arguments = Command::new(binary(directory, "scene_player"))
        .current_dir(directory)
        .output()
        .expect("the player runs");
    assert!(!without_arguments.status.success());
    assert!(
        String::from_utf8_lossy(&without_arguments.stderr)
            .contains("play a scene with `player <scene> <title>`"),
        "the player says how to run it"
    );
    let missing_scene = Command::new(binary(directory, "scene_player"))
        .args(["levels/elsewhere.scene", "Player"])
        .current_dir(directory)
        .output()
        .expect("the player runs");
    assert!(!missing_scene.status.success());
    assert!(
        String::from_utf8_lossy(&missing_scene.stderr)
            .contains("the main scene, \"levels/elsewhere.scene\", could not be loaded"),
        "the player names the scene it was given"
    );
}
