use std::rc::Rc;

use super::*;
use crate::List;
use crate::scene_fields::SceneValue;
use crate::scenes::{
    SceneAnimator, SceneBody, SceneCamera, SceneSprite, SceneTransform, SceneTransform3D,
};
use crate::test_host::TestHost;

fn node(name: &str, class: Option<&str>, parent: Option<usize>) -> SceneNode {
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
        fields: Vec::new(),
    }
}

fn sprite() -> SceneSprite {
    SceneSprite {
        texture: "ship.png".to_owned(),
        width: 2.0,
        height: 3.0,
        layer: 4,
        pixelated: true,
        color: [0.5, 0.5, 0.5, 1.0],
        columns: 4,
        rows: 2,
        frame: 5,
        frames: "ship.frames".to_owned(),
        animation: "fly".to_owned(),
    }
}

fn names(nodes: &List<Rc<crate::test_host::TestNode>>) -> Vec<String> {
    nodes
        .to_vec()
        .iter()
        .map(|node| node.node.name.get())
        .collect()
}

#[test]
fn nodes_are_added_to_their_parents_and_the_root_is_given() {
    let nodes = [
        node("Level", None, None),
        node("Floor", None, Some(0)),
        node("Lamp", None, Some(1)),
    ];

    let root = build::<TestHost>(&nodes).expect("built");

    assert_eq!(root.node.name.get(), "Level");
    assert_eq!(names(&nodes::children::<TestHost>(&root)), ["Floor"]);
    let floor = nodes::find_child::<TestHost>(&root, "Floor").expect("floor");
    assert_eq!(names(&nodes::children::<TestHost>(&floor)), ["Lamp"]);
}

#[test]
fn hidden_scene_nodes_are_built_hidden() {
    let ghost = SceneNode {
        visible: false,
        ..node("Ghost", None, Some(0))
    };
    let nodes = [node("Level", None, None), ghost];

    let root = build::<TestHost>(&nodes).expect("built");

    assert!(root.node.visible.get());
    let ghost = nodes::find_child::<TestHost>(&root, "Ghost").expect("ghost");
    assert!(!ghost.node.visible.get());
}

#[test]
fn animators_take_their_file_and_play_their_first_animation() {
    let animator = SceneNode {
        animator: Some(SceneAnimator {
            animations: "knight.anim".to_owned(),
            animation: "idle".to_owned(),
        }),
        ..node("Animator", None, Some(0))
    };
    let nodes = [node("Knight", None, None), animator];

    let root = build::<TestHost>(&nodes).expect("built");

    let animator = nodes::find_child::<TestHost>(&root, "Animator").expect("animator");
    let file = TestHost::with_animator(&animator, |fields| fields.animations.get());
    assert_eq!(file.as_deref(), Some("knight.anim"));
    assert_eq!(crate::tracks::animation::<TestHost>(&animator), "idle");
    assert!(crate::tracks::is_playing::<TestHost>(&animator));
}

#[test]
fn scenes_without_nodes_cannot_be_built() {
    assert_eq!(
        build::<TestHost>(&[]).err().as_deref(),
        Some("it has no nodes")
    );
}

#[test]
fn nodes_without_a_class_are_what_their_parts_show() {
    let mut ship = node("Ship", None, None);
    ship.sprite = Some(sprite());
    ship.tags = vec!["player".to_owned()];
    ship.transform = Some(SceneTransform {
        x: 1.0,
        y: 2.0,
        rotation: 0.5,
        scale_x: 1.0,
        scale_y: 1.0,
    });

    let built = build::<TestHost>(&[ship]).expect("built");

    let look = built.sprite.as_ref().expect("a sprite");
    assert_eq!(look.texture.get(), "ship.png");
    assert_eq!(look.layer.get(), 4);
    assert_eq!(look.color.get(), [0.5, 0.5, 0.5, 1.0]);
    let sheet = (look.columns.get(), look.rows.get(), look.frame.get());
    assert_eq!(sheet, (4, 2, 5));
    assert_eq!(look.frames.get(), "ship.frames");
    assert_eq!(crate::animations::animation::<TestHost>(&built), "fly");
    assert!(crate::animations::is_playing::<TestHost>(&built));
    assert!(nodes::has_tag::<TestHost>(&built, "player"));
    assert!((built.node.y.get() - 2.0).abs() < f32::EPSILON);
}

#[test]
fn engine_classes_need_their_parts() {
    let error = build::<TestHost>(&[node("Ship", Some("Sprite"), None)]).err();

    assert_eq!(
        error.as_deref(),
        Some("node \"Ship\": a `Sprite` needs a `[node.sprite]`")
    );
}

#[test]
fn parts_must_fit_the_class() {
    let mut lamp = node("Lamp", Some("Node"), None);
    lamp.camera = Some(SceneCamera {
        view_height: 4.0,
        main: true,
    });
    let mut flat = node("Flat", Some("Enemy"), None);
    flat.transform_3d = Some(SceneTransform3D {
        x: 0.0,
        y: 0.0,
        z: 0.0,
        pitch: 0.0,
        yaw: 0.0,
        roll: 0.0,
        scale_x: 1.0,
        scale_y: 1.0,
        scale_z: 1.0,
    });

    let camera = build::<TestHost>(&[lamp]).err();
    let deep = build::<TestHost>(&[flat]).err();

    assert_eq!(
        camera.as_deref(),
        Some("node \"Lamp\": it is a 2D camera, but `Node` is not a `Camera2D`")
    );
    assert_eq!(
        deep.as_deref(),
        Some("node \"Flat\": it is placed in 3D, but `Enemy` is not a `Node3D`")
    );
}

#[test]
fn cameras_take_their_settings() {
    let mut eye = node("Eye", None, None);
    eye.camera = Some(SceneCamera {
        view_height: 4.0,
        main: true,
    });

    let built = build::<TestHost>(&[eye]).expect("built");

    let camera = built.camera.as_ref().expect("a camera");
    assert!(camera.main.get());
    assert!((camera.view_height.get() - 4.0).abs() < f32::EPSILON);
}

#[test]
fn script_classes_get_their_exported_fields() {
    let mut enemy = node("Bat", Some("Enemy"), None);
    enemy.fields = vec![("speed".to_owned(), SceneValue::Float(7.0))];
    let mut wrong = node("Rat", Some("Enemy"), None);
    wrong.fields = vec![("size".to_owned(), SceneValue::Float(1.0))];

    let built = build::<TestHost>(&[enemy]).expect("built");
    let error = build::<TestHost>(&[wrong]).err();

    assert_eq!(built.calls(), ["speed 7"]);
    assert_eq!(
        error.as_deref(),
        Some("node \"Rat\": `Enemy` has no `export var size`")
    );
}

#[test]
fn bodies_take_the_scene_s_surface() {
    let mut ball = node("Ball", None, None);
    ball.body = Some(SceneBody {
        kind: "dynamic".to_owned(),
        shape: "circle".to_owned(),
        radius: 0.25,
        width: 1.0,
        height: 1.0,
        depth: 1.0,
        gravity_scale: 0.5,
        fixed_rotation: true,
        friction: 0.1,
        bounciness: 0.9,
        density: 2.0,
        sensor: false,
    });

    let built = build::<TestHost>(&[ball]).expect("built");

    let body = built.node.body.get().expect("a body");
    assert_eq!(body.shape.get(), "circle");
    assert!((body.bounciness.get() - 0.9).abs() < f32::EPSILON);
    assert!(body.fixed_rotation.get());
}
