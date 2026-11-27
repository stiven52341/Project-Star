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
const MESH_GAME: &str = include_str!("games/mesh_game.star");
const SQUARE_GAME: &str = include_str!("games/square_game.star");
const COROUTINE_GAME: &str = include_str!("games/coroutine_game.star");
const ANIMATION_GAME: &str = include_str!("games/animation_game.star");
const HERO_FRAMES: &str = include_str!("games/hero.frames");
const ANIMATOR_GAME: &str = include_str!("games/animator_game.star");
const KNIGHT_SCENE: &str = include_str!("games/knight.scene");
const TWEEN_GAME: &str = include_str!("games/tween_game.star");
const STATES_GAME: &str = include_str!("games/states_game.star");
const MODEL_GAME: &str = include_str!("games/model_game.star");
const WAVE_MODEL: &[u8] = include_bytes!("games/wave.glb");
const GUARD_SCENE: &str = include_str!("games/guard.scene");
const GUARD_ANIMATIONS: &str = include_str!("games/guard.anim");
const GUARD_STATES: &str = include_str!("games/guard.states");
const KNIGHT_ANIMATIONS: &str = include_str!("games/knight.anim");
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
        ..Default::default()
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
    for (name, source) in [
        ("node_game", NODE_GAME),
        ("physics_game", PHYSICS_GAME),
        ("physics_3d_game", PHYSICS_3D_GAME),
        ("mesh_game", MESH_GAME),
        ("square_game", SQUARE_GAME),
        ("coroutine_game", COROUTINE_GAME),
        ("animation_game", ANIMATION_GAME),
        ("animator_game", ANIMATOR_GAME),
        ("tween_game", TWEEN_GAME),
        ("states_game", STATES_GAME),
        ("model_game", MODEL_GAME),
    ] {
        package
            .add_program(name, compile_with(name, source, &valid))
            .expect("valid name");
    }
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

    let (stdout, _) = play(&directory, "mesh_game");
    assert_eq!(stdout, include_str!("games/mesh_game.out"));

    let (stdout, _) = play(&directory, "square_game");
    assert_eq!(stdout, include_str!("games/square_game.out"));

    let (stdout, _) = play(&directory, "coroutine_game");
    // The blinker's coroutine ends with frame 3, when it leaves the tree.
    assert_eq!(stdout, include_str!("games/coroutine_game.out"));

    std::fs::write(levels.join("hero.frames"), HERO_FRAMES).expect("frames written");
    let (stdout, _) = play(&directory, "animation_game");
    assert_eq!(stdout, include_str!("games/animation_game.out"));

    std::fs::write(levels.join("knight.scene"), KNIGHT_SCENE).expect("scene written");
    std::fs::write(levels.join("knight.anim"), KNIGHT_ANIMATIONS).expect("animations written");
    let (stdout, _) = play(&directory, "animator_game");
    assert_eq!(stdout, include_str!("games/animator_game.out"));

    let (stdout, _) = play(&directory, "tween_game");
    assert_eq!(stdout, include_str!("games/tween_game.out"));

    state_machines_choose_the_animations(&directory, &levels);

    std::fs::write(levels.join("wave.glb"), WAVE_MODEL).expect("model written");
    let (stdout, _) = play(&directory, "model_game");
    assert_eq!(stdout, include_str!("games/model_game.out"));

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

/// Plays the game whose animator runs the state machine of
/// `guard.states`, with its scene and animations in `levels`.
fn state_machines_choose_the_animations(directory: &Path, levels: &Path) {
    std::fs::write(levels.join("guard.scene"), GUARD_SCENE).expect("scene written");
    std::fs::write(levels.join("guard.anim"), GUARD_ANIMATIONS).expect("animations written");
    std::fs::write(levels.join("guard.states"), GUARD_STATES).expect("states written");
    let (stdout, _) = play(directory, "states_game");
    assert_eq!(stdout, include_str!("games/states_game.out"));
}

/// The scene-only game stops, saying which scene it could not load (a
/// scene it loads would play until closed).
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
}
