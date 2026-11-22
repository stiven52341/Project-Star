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
use star_script::{GameSettings, InputAction};

/// Counts its frames and closes itself after the third one. Nobody presses
/// `jump` while the test runs.
const GAME: &str = "
class CountingGame extends Game {
    var frames = 0

    override func start() {
        Debug.print(\"start\")
    }

    override func update(delta: float) {
        frames += 1
        Debug.print(\"frame \" + frames.to_string() + \" \" + (delta >= 0.0).to_string())
        if Input.is_down(\"jump\") || Input.just_pressed(\"jump\") || Input.strength(\"jump\") > 0.0 || Input.axis(\"jump\", \"jump\") != 0.0 {
            Debug.print(\"jump\")
        }
        // Where the mouse is depends on the desktop: only read it.
        if Input.has_pointer() && Input.pointer_x() + Input.pointer_y() > 1000000.0 {
            Debug.print(\"far away\")
        }
        if frames == 3 {
            quit()
        }
    }
}
";

/// A sprite node that runs its own code every frame, in the game's tree.
const NODE_GAME: &str = "
class Ship extends Sprite {
    init() {
        super.init(\"Ship\", \"sprites/ship.png\")
        color = Color(1.0, 0.5, 0.5, 1.0)
    }

    override func start() {
        Debug.print(\"ship start\")
    }

    override func update(delta: float) {
        x += 1.0
        Debug.print(\"ship x \" + x.to_string())
    }
}

class Drone extends Node {
    init() {
        super.init(\"Drone\")
    }

    override func start() {
        Debug.print(\"drone start \" + name + \" \" + x.to_string())
    }
}

class NodeGame extends Game {
    var frames = 0

    override func start() {
        background = Color(0.0, 0.0, 0.1, 1.0)
        const fleet = Scene.load(\"levels/fleet.scene\")
        if fleet != null {
            add_child(fleet)
            Debug.print(fleet.name + \" \" + fleet.children().length().to_string())
            const leader = fleet.find_child(\"Leader\")
            if leader is Sprite {
                Debug.print(leader.texture + \" \" + leader.x.to_string() + \" \" + leader.color.g.to_string())
            }
        }
        Debug.print((Scene.load(\"levels/missing.scene\") == null).to_string())
        Debug.print((Scene.load(\"levels/misspelled.scene\") == null).to_string())
        const ship = Ship()
        add_child(ship)
        const eye = Camera(\"Eye\")
        ship.add_child(eye)
        camera = eye
    }

    override func update(delta: float) {
        frames += 1
        Debug.print(\"game frame \" + frames.to_string())
        if frames == 3 {
            quit()
        }
    }
}
";

/// The scene the node game loads, from the `assets` folder of its working
/// directory.
const FLEET_SCENE: &str = r#"
format = 1

[[node]]
name = "Fleet"

[[node]]
name = "Leader"
parent = 0
position = [1.5, 0.0]

[node.sprite]
texture = "sprites/leader.png"
size = [1.0, 1.0]
color = [1.0, 0.5, 0.5, 1.0]

[[node]]
name = "Wingman"
class = "Drone"
parent = 0
position = [-2.0, 0.0]
"#;

/// A scene whose node names a class the game does not have.
const MISSPELLED_SCENE: &str = r#"
format = 1

[[node]]
name = "Drone"
class = "Dron"
"#;

fn settings(bindings: &[&str]) -> GameSettings {
    GameSettings {
        actions: vec![InputAction {
            name: "jump".to_owned(),
            bindings: bindings.iter().map(|&binding| binding.to_owned()).collect(),
        }],
    }
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
    package.write(&directory).expect("package written");

    build(&directory);
    let output = Command::new(binary(&directory, "counting_game"))
        .output()
        .expect("the game runs");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "the game failed:\n{stderr}");
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "start\nframe 1 true\nframe 2 true\nframe 3 true\n"
    );

    let levels = directory.join("assets/levels");
    std::fs::create_dir_all(&levels).expect("asset folder created");
    std::fs::write(levels.join("fleet.scene"), FLEET_SCENE).expect("scene written");
    std::fs::write(levels.join("misspelled.scene"), MISSPELLED_SCENE).expect("scene written");
    let output = Command::new(binary(&directory, "node_game"))
        .current_dir(&directory)
        .output()
        .expect("the game runs");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "the node game failed:\n{stderr}");
    assert!(
        stderr.contains("node \"Drone\": there is no class \"Dron\""),
        "the misspelled class is reported:\n{stderr}"
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "Fleet 2\nsprites/leader.png 1.5 0.5\ntrue\ntrue\n\
         game frame 1\nship start\nship x 1.0\ndrone start Wingman -2.0\ngame frame 2\nship x 2.0\n\
         game frame 3\nship x 3.0\n"
    );

    let output = Command::new(binary(&directory, "misspelled_game"))
        .output()
        .expect("the game runs");
    assert!(!output.status.success(), "a wrong binding stops the game");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("unknown input binding \"Spcae\""),
        "the error names the binding"
    );
}
