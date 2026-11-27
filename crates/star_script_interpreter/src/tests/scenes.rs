//! Scenes, read from a loader in memory instead of files: the runtime asks
//! the loader of its thread, as a game's does.

use star_script::{GameSettings, MainScene, check_source_with};
use star_script_runtime::scene_fields::SceneValue;
use star_script_runtime::scenes::{self, SceneMesh, SceneNode, SceneTransform3D};

use super::{play, play_program};
use crate::{Program, RunError};

/// A node of a scene with nothing but its name, its class and its parent.
fn scene_node(name: &str, class: Option<&str>, parent: Option<usize>) -> SceneNode {
    SceneNode {
        name: name.to_owned(),
        class: class.map(str::to_owned),
        tags: Vec::new(),
        visible: true,
        parent,
        transform: None,
        sprite: None,
        body: None,
        transform_3d: None,
        mesh: None,
        light: None,
        light_2d: None,
        camera: None,
        camera_3d: None,
        animator: None,
        bone: None,
        model: None,
        fields: Vec::new(),
    }
}

/// Lets the runtime of this thread read the scenes `fleet.scene` (a
/// `Drone` with its `export` fields set, and a camera), `drone.scene`
/// (a `Drone` with only its `speed`),
/// `misspelled.scene` (a class the program does not have), `wrong.scene`
/// (a value that does not fit its field) and `broken.scene` (a class
/// whose `init` fails), and `space.scene` (3D nodes).
fn load_scenes_from_memory() {
    scenes::set_loader(|path| {
        if path == "space.scene" {
            return Ok(space_scene());
        }
        let mut root = scene_node("Fleet", None, None);
        root.tags = vec!["fleet".to_owned()];
        let mut drone = scene_node("Wingman", Some("Drone"), Some(0));
        drone.tags = vec!["ship".to_owned()];
        let mut nodes = vec![root, drone];
        match path {
            "fleet.scene" => {
                nodes[1].fields = vec![
                    ("speed".to_owned(), SceneValue::Int(4)),
                    ("label".to_owned(), SceneValue::Text("wing".to_owned())),
                    ("tint".to_owned(), SceneValue::Color([0.25, 1.0, 1.0, 1.0])),
                ];
                nodes.push(scene_node("Lookout", Some("Camera2D"), Some(0)));
            }
            "drone.scene" => nodes[1].fields = vec![("speed".to_owned(), SceneValue::Int(4))],
            "misspelled.scene" => nodes[1].class = Some("Dron".to_owned()),
            "wrong.scene" => {
                nodes[1].fields = vec![("speed".to_owned(), SceneValue::Text("fast".to_owned()))];
            }
            "broken.scene" => nodes[1].class = Some("Broken".to_owned()),
            _ => return Err("missing".to_owned()),
        }
        Ok(nodes)
    });
}

/// A `Node3D` holding a cube one unit deep, and a camera.
fn space_scene() -> Vec<SceneNode> {
    let mut cube = scene_node("Cube", Some("Mesh"), Some(0));
    cube.transform_3d = Some(SceneTransform3D {
        x: 0.0,
        y: 0.0,
        z: 1.0,
        pitch: 0.0,
        yaw: 0.0,
        roll: 0.0,
        scale_x: 1.0,
        scale_y: 1.0,
        scale_z: 2.0,
    });
    cube.mesh = Some(SceneMesh {
        shape: "cube".to_owned(),
        width: 1.0,
        height: 1.0,
        depth: 1.0,
        radius: 0.5,
        color: [1.0, 0.5, 0.5, 1.0],
        metallic: 0.0,
        roughness: 0.75,
        texture: String::new(),
        pixelated: false,
    });
    vec![
        scene_node("Space", Some("Node3D"), None),
        cube,
        scene_node("Eye", Some("Camera3D"), Some(0)),
    ]
}

/// The classes the scenes name, and a game that loads them.
const FLEET_GAME: &str = "class Drone extends Node {\n export var speed = 1.0\n export var label = \"none\"\n export var tint = Color(1.0, 1.0, 1.0, 1.0)\n \
     init() {\n super.init(\"Drone\")\n Debug.print(\"drone init\")\n }\n \
     override func start() {\n Debug.print(\"drone start \" + name + \" \" + speed.to_string() + \" \" + label + \" \" + tint.r.to_string())\n }\n}\n\
     class Broken extends Node {\n init() {\n super.init(\"Broken\")\n Debug.print([1][2].to_string())\n }\n}\n\
     class Fleet extends Game {\n override func start() {\n \
     const fleet = Scene.load(\"fleet.scene\")\n if fleet != null {\n add_child(fleet)\n \
     Debug.print(fleet.name + \" \" + fleet.children().length().to_string() + \" \" + (fleet.find_child(\"Lookout\") is Camera2D).to_string())\n \
     Debug.print(Scene.find_by_tag(\"ship\").length().to_string())\n }\n \
     Debug.print((Scene.load(\"misspelled.scene\") == null).to_string())\n \
     Debug.print((Scene.load(\"wrong.scene\") == null).to_string())\n \
     Debug.print((Scene.load(\"missing.scene\") == null).to_string())\n }\n \
     override func update(delta: float) {\n quit()\n }\n}\n";

#[test]
fn scenes_create_the_program_s_classes_with_their_exported_values() {
    load_scenes_from_memory();

    let (printed, result) = play(FLEET_GAME, 10);

    // The refused scenes create their drone before the reason shows up,
    // as compiled.
    assert_eq!(
        printed,
        "drone init\nFleet 2 true\n1\ntrue\ndrone init\ntrue\ntrue\ndrone start Wingman 4.0 wing 0.25\n"
    );
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn errors_in_the_init_of_a_scene_s_class_stop_the_game() {
    load_scenes_from_memory();
    let source = FLEET_GAME.replace("\"fleet.scene\"", "\"broken.scene\"");

    let (printed, result) = play(&source, 10);

    assert_eq!(printed, "");
    assert!(matches!(result, Err(RunError::Fault(_))), "{result:?}");
}

/// A program without a `Game` class whose project plays the scene at
/// `path`.
fn main_scene_program(path: &str) -> Program {
    let settings = GameSettings {
        actions: Vec::new(),
        main_scene: Some(MainScene {
            path: path.to_owned(),
            title: "Fleet".to_owned(),
        }),
        ..Default::default()
    };
    let source = "class Drone extends Node {\n export var speed = 1.0\n init() {\n super.init(\"Drone\")\n }\n \
                  override func start() {\n Debug.print(\"drone \" + speed.to_string() + \" \" + (Scene.find_by_name(\"Fleet\") != null).to_string())\n }\n}\n";
    let checked = check_source_with(source, &settings).expect("a valid program");
    Program::lower(&checked).expect("a program the interpreter runs")
}

#[test]
fn programs_without_a_game_play_their_main_scene() {
    load_scenes_from_memory();

    let (printed, result) = play_program(&main_scene_program("drone.scene"), 2);

    assert_eq!(printed, "drone 4.0 true\n");
    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn main_scenes_that_cannot_be_loaded_stop_the_game() {
    load_scenes_from_memory();
    let program = main_scene_program("nowhere.scene");
    let (_, output) = super::buffer();

    let error = program.new_game(&output).err();

    assert_eq!(
        error.map(|error| error.to_string()),
        Some("the main scene, \"nowhere.scene\", could not be loaded".to_owned())
    );
}

#[test]
fn scenes_create_3d_nodes() {
    load_scenes_from_memory();
    let source = "class Space extends Game {\n override func start() {\n \
                  const space = Scene.load(\"space.scene\")\n const cube = space?.find_child(\"Cube\") as Mesh\n \
                  if cube != null {\n Debug.print(cube.shape + \" \" + cube.roughness.to_string() + \" \" + cube.z.to_string() + \" \" + cube.scale_z.to_string() + \" \" + cube.color.g.to_string())\n }\n \
                  Debug.print(((space?.find_child(\"Eye\") is Camera3D)).to_string() + \" \" + (space is Node3D).to_string())\n }\n \
                  override func update(delta: float) {\n quit()\n }\n}\n";

    let (printed, result) = play(source, 2);

    assert_eq!(printed, "cube 0.75 1.0 2.0 0.5\ntrue true\n");
    assert!(result.is_ok(), "{result:?}");
}
