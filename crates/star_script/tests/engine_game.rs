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
    // The test's game has no sounds: it stays null, with a warning.
    var beep = Sound.load(\"sounds/beep.wav\")

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
        beep?.play()
        Audio.set_volume(Audio.volume())
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
    export var speed = 1.0
    export var label = \"none\"
    export var tint = Color(1.0, 1.0, 1.0, 1.0)

    init() {
        super.init(\"Drone\")
    }

    override func start() {
        Debug.print(\"drone start \" + name + \" \" + x.to_string() + \" \" + speed.to_string() + \" \" + label + \" \" + tint.r.to_string())
    }
}

class NodeGame extends Game {
    var frames = 0

    override func start() {
        background = Color(0.0, 0.0, 0.1, 1.0)
        ambient_2d = Color(0.2, 0.2, 0.3, 1.0)
        add_child(Light2D(\"Lamp\", \"point\"))
        const fleet = Scene.load(\"levels/fleet.scene\")
        if fleet != null {
            add_child(fleet)
            Debug.print(fleet.name + \" \" + fleet.children().length().to_string())
            Debug.print(Scene.find_by_tag(\"ship\").length().to_string() + \" \" + (Scene.find_by_name(\"Wingman\") != null).to_string())
            const leader = fleet.find_child(\"Leader\")
            if leader is Sprite {
                Debug.print(leader.texture + \" \" + leader.x.to_string() + \" \" + leader.color.g.to_string())
            }
            const torch = fleet.find_child(\"Torch\")
            if torch is Light2D {
                Debug.print(torch.kind + \" \" + torch.range.to_string() + \" \" + torch.angle.to_string())
            }
        }
        Debug.print((Scene.load(\"levels/missing.scene\") == null).to_string())
        Debug.print((Scene.load(\"levels/misspelled.scene\") == null).to_string())
        const ship = Ship()
        add_child(ship)
        // Not `current`: the scene's camera is the one the game is seen from.
        const eye = Camera2D(\"Eye\")
        ship.add_child(eye)
    }

    override func update(delta: float) {
        frames += 1
        Debug.print(\"game frame \" + frames.to_string())
        const view = camera
        if view != null {
            Debug.print(\"camera \" + view.name + \" \" + view.view_height.to_string())
        }
        if frames == 3 {
            quit()
        }
    }
}
";

/// A ball that falls on a floor loaded from a scene, and closes the game
/// when it lands.
const PHYSICS_GAME: &str = "
class Ball extends Node {
    init() {
        super.init(\"Ball\")
        body = Body.circle(\"dynamic\", 0.5)
    }
}

class PhysicsGame extends Game {
    var frames = 0
    var ball = Ball()

    override func start() {
        const floor = Scene.load(\"levels/floor.scene\")
        if floor != null {
            add_child(floor)
            const solid = floor.body
            if solid != null {
                Debug.print(solid.kind + \" \" + solid.shape + \" \" + solid.width.to_string())
            }
        }
        add_child(ball)
    }

    override func update(delta: float) {
        frames += 1
        const solid = ball.body
        if solid != null {
            for other in solid.just_touched() {
                Debug.print(\"landed on \" + other.name + \" \" + (ball.y < 0.0).to_string())
                quit()
            }
        }
        if frames == 600 {
            Debug.print(\"never landed\")
            quit()
        }
    }
}
";

/// A ball that falls in 3D onto a floor of a scene, seen from a 3D camera
/// and lit by the sun.
const PHYSICS_3D_GAME: &str = "
class Ball extends Mesh {
    init() {
        super.init(\"Ball\", \"sphere\")
        color = Color(0.5, 0.7, 1.0, 1.0)
        body = Body.sphere(\"dynamic\", 0.5)
    }
}

class Physics3dGame extends Game {
    var frames = 0
    var ball = Ball()

    override func start() {
        const floor = Scene.load(\"levels/floor_3d.scene\")
        if floor != null {
            add_child(floor)
            const solid = floor.body
            if solid != null {
                Debug.print(solid.shape + \" \" + solid.depth.to_string() + \" \" + (floor is Mesh).to_string())
            }
            const spot = floor.find_child(\"Spot\")
            if spot is Light {
                Debug.print(spot.kind + \" \" + spot.angle.to_string() + \" \" + spot.blend.to_string())
            }
        }
        const eye = Camera3D(\"Eye\")
        eye.y = 2.0
        eye.z = 8.0
        eye.look_at(0.0, 0.0, 0.0)
        camera_3d = eye
        add_child(eye)
        const sun = Light(\"Sun\", \"directional\")
        sun.pitch = -1.0
        add_child(sun)
        ambient = Color(0.2, 0.2, 0.2, 1.0)
        add_child(ball)
    }

    override func update(delta: float) {
        frames += 1
        const solid = ball.body
        if solid != null {
            for other in solid.just_touched() {
                Debug.print(\"landed on \" + other.name + \" \" + (ball.y < 0.0).to_string())
                quit()
            }
        }
        if frames == 600 {
            Debug.print(\"never landed\")
            quit()
        }
    }
}
";

/// A node whose coroutine prints every frame until it leaves the tree, and
/// a coroutine of the game that waits for a frame.
const COROUTINE_GAME: &str = "
class Blinker extends Node {
    init() {
        super.init(\"Blinker\")
    }

    async func blink() {
        while true {
            Debug.print(\"blink\")
            await Time.next_frame()
        }
    }

    override func start() {
        blink()
    }
}

class CoroutineGame extends Game {
    var frames = 0
    var blinker = Blinker()

    override func start() {
        add_child(blinker)
        later()
    }

    async func later() {
        await Time.next_frame()
        Debug.print(\"later\")
    }

    override func update(delta: float) {
        frames += 1
        Debug.print(\"frame \" + frames.to_string())
        if frames == 3 {
            blinker.remove()
        }
        if frames == 5 {
            quit()
        }
    }
}
";

/// The floor of the 3D physics game: a mesh with a body.
const FLOOR_3D_SCENE: &str = r#"
format = 1

[[node]]
name = "Floor"

[node.transform_3d]
position = [0.0, -2.0, 0.0]

[node.mesh]
shape = "cube"
size = [10.0, 1.0, 10.0]

[node.body_3d]
kind = "static"
shape = "box"
size = [10.0, 1.0, 10.0]

[[node]]
name = "Spot"
parent = 0

[node.transform_3d]
position = [0.0, 4.0, 0.0]
rotation = [-1.5, 0.0, 0.0]

[node.light]
kind = "spot"
range = 8.0
angle = 0.5
"#;

/// The floor of the physics game.
const FLOOR_SCENE: &str = r#"
format = 1

[[node]]
name = "Floor"
position = [0.0, -2.0]

[node.body]
kind = "static"
shape = "rectangle"
size = [10.0, 1.0]
"#;

/// The scene the node game loads, from the `assets` folder of its working
/// directory.
const FLEET_SCENE: &str = r#"
format = 1

[[node]]
name = "Fleet"

[[node]]
name = "Leader"
parent = 0
tags = ["ship"]
position = [1.5, 0.0]

[node.sprite]
texture = "sprites/leader.png"
size = [1.0, 1.0]
color = [1.0, 0.5, 0.5, 1.0]

[[node]]
name = "Wingman"
class = "Drone"
parent = 0
tags = ["ship", "drone"]
position = [-2.0, 0.0]

[node.fields]
speed = 4
label = "wing"
tint = [0.25, 1.0, 1.0, 1.0]

[[node]]
name = "Lookout"
parent = 0

[node.camera]
view_height = 6.0
current = true

[[node]]
name = "Torch"
parent = 0
rotation = 1.5

[node.light_2d]
kind = "spot"
range = 4.0
angle = 0.5
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
    package.write(&directory).expect("package written");

    build(&directory);
    let (stdout, _) = play(&directory, "counting_game");
    assert_eq!(stdout, "start\nframe 1 true\nframe 2 true\nframe 3 true\n");

    let levels = directory.join("assets/levels");
    std::fs::create_dir_all(&levels).expect("asset folder created");
    std::fs::write(levels.join("fleet.scene"), FLEET_SCENE).expect("scene written");
    std::fs::write(levels.join("misspelled.scene"), MISSPELLED_SCENE).expect("scene written");
    let (stdout, stderr) = play(&directory, "node_game");
    assert!(
        stderr.contains("node \"Drone\": there is no class \"Dron\""),
        "the misspelled class is reported:\n{stderr}"
    );
    assert_eq!(
        stdout,
        "Fleet 4\n2 true\nsprites/leader.png 1.5 0.5\nspot 4.0 0.5\ntrue\ntrue\n\
         game frame 1\nship start\nship x 1.0\ndrone start Wingman -2.0 4.0 wing 0.25\n\
         game frame 2\ncamera Lookout 6.0\nship x 2.0\n\
         game frame 3\ncamera Lookout 6.0\nship x 3.0\n"
    );

    std::fs::write(levels.join("floor.scene"), FLOOR_SCENE).expect("scene written");
    let (stdout, _) = play(&directory, "physics_game");
    assert_eq!(stdout, "static rectangle 10.0\nlanded on Floor true\n");

    std::fs::write(levels.join("floor_3d.scene"), FLOOR_3D_SCENE).expect("scene written");
    let (stdout, _) = play(&directory, "physics_3d_game");
    assert_eq!(
        stdout,
        "box 10.0 true\nspot 0.5 0.2\nlanded on Floor true\n"
    );

    let (stdout, _) = play(&directory, "coroutine_game");
    // The blinker's coroutine ends with frame 3, when it leaves the tree.
    assert_eq!(
        stdout,
        "frame 1\nblink\nframe 2\nlater\nblink\nframe 3\nframe 4\nframe 5\n"
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
