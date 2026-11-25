//! The interpreter as a [`Host`] of `star_script_runtime` (decision 70):
//! what its nodes, colors and bodies are, so the runtime's fields and its
//! code of nodes work on them as on the compiled game's.
//!
//! The engine classes of later steps of Phase 14 (`Node3D`, `Camera3D`,
//! the game) are [`NotYet`] here: no value of them exists, so the runtime
//! is never handed one.

use std::rc::{Rc, Weak};

use star_script_runtime::bodies::BodyFields;
use star_script_runtime::color::ColorFields;
use star_script_runtime::game::GameFields;
use star_script_runtime::node_classes::{
    Camera2dFields, Camera3dFields, LightFields, MeshFields, SpriteFields,
};
use star_script_runtime::nodes::{Host, Node3dFields, NodeFields};
use star_script_runtime::scene_fields::SceneValue;
use star_script_runtime::scene_tree::NewNode;

use crate::tree::EngineClass;
use crate::value::Object;

/// The interpreter's objects, as the runtime sees them.
pub(crate) enum Interpreted {}

/// An engine class the interpreter cannot make objects of yet.
#[derive(Clone, Copy)]
pub(crate) enum NotYet {}

/// An object of `Node`, `Camera2D` or a class that extends one.
pub(crate) type NodeObject = Object<NodeParts>;

/// The runtime's fields of a node: those of `Node`, and those of its node
/// class.
pub(crate) struct NodeParts {
    pub(crate) node: NodeFields<Interpreted>,
    /// The fields of `Camera2D`, if it is one: kept apart, so a camera can
    /// be handed out with them.
    pub(crate) camera_2d: Option<Rc<Camera2dFields>>,
}

impl NodeParts {
    /// The fields of a new node of `class` called `name`.
    pub(crate) fn new(class: EngineClass, name: String) -> Self {
        Self {
            node: NodeFields::new(name),
            camera_2d: (class == EngineClass::Camera2D).then(|| Rc::new(Camera2dFields::new())),
        }
    }
}

/// A `Camera2D`: the node, and its fields of camera.
#[derive(Clone)]
pub(crate) struct CameraRef {
    node: Rc<NodeObject>,
    fields: Rc<Camera2dFields>,
}

/// Why the interpreter cannot make the nodes of scenes yet.
const NO_SCENES: &str = "the interpreter cannot read scenes yet";

impl Host for Interpreted {
    type Node = Rc<NodeObject>;
    type WeakNode = Weak<NodeObject>;
    type Node3D = NotYet;
    type Camera2D = CameraRef;
    type Camera3D = NotYet;
    type Body = Rc<BodyFields<Self>>;
    type Color = Rc<Object<ColorFields>>;
    type Game = NotYet;

    fn node(node: &Self::Node) -> &NodeFields<Self> {
        &node.engine.node
    }

    fn downgrade(node: &Self::Node) -> Self::WeakNode {
        Rc::downgrade(node)
    }

    fn upgrade(weak: &Self::WeakNode) -> Option<Self::Node> {
        weak.upgrade()
    }

    fn same(node: &Self::Node, other: &Self::Node) -> bool {
        Rc::ptr_eq(node, other)
    }

    fn as_node_3d(_node: &Self::Node) -> Option<Self::Node3D> {
        None
    }

    fn node_3d(node: &Self::Node3D) -> &Node3dFields {
        match *node {}
    }

    fn as_camera_2d(node: &Self::Node) -> Option<Self::Camera2D> {
        let fields = node.engine.camera_2d.as_ref()?;
        Some(CameraRef {
            node: Rc::clone(node),
            fields: Rc::clone(fields),
        })
    }

    fn camera_2d(camera: &Self::Camera2D) -> &Camera2dFields {
        &camera.fields
    }

    fn camera_2d_node(camera: &Self::Camera2D) -> Self::Node {
        Rc::clone(&camera.node)
    }

    fn as_camera_3d(_node: &Self::Node) -> Option<Self::Camera3D> {
        None
    }

    fn camera_3d(camera: &Self::Camera3D) -> &Camera3dFields {
        match *camera {}
    }

    fn camera_3d_node(camera: &Self::Camera3D) -> Self::Node {
        match *camera {}
    }

    fn with_sprite<R>(
        _node: &Self::Node,
        _read: impl FnOnce(&SpriteFields<Self>) -> R,
    ) -> Option<R> {
        None
    }

    fn with_mesh<R>(_node: &Self::Node, _read: impl FnOnce(&MeshFields<Self>) -> R) -> Option<R> {
        None
    }

    fn with_light<R>(_node: &Self::Node, _read: impl FnOnce(&LightFields<Self>) -> R) -> Option<R> {
        None
    }

    fn with_light_2d<R>(
        _node: &Self::Node,
        _read: impl FnOnce(&LightFields<Self>) -> R,
    ) -> Option<R> {
        None
    }

    fn body(body: &Self::Body) -> &BodyFields<Self> {
        body
    }

    fn color(r: f32, g: f32, b: f32, a: f32) -> Self::Color {
        Rc::new(Object::new(
            EngineClass::Color.class(),
            Vec::new(),
            ColorFields::new(r, g, b, a),
        ))
    }

    fn channels(color: &Self::Color) -> [f32; 4] {
        color.engine.channels()
    }

    fn create(_name: String, _class: NewNode<'_>) -> Result<Self::Node, String> {
        Err(NO_SCENES.to_owned())
    }

    fn set_exported(
        _class: &str,
        _node: &Self::Node,
        _values: &[(String, SceneValue)],
    ) -> Result<(), String> {
        Err(NO_SCENES.to_owned())
    }

    fn new_body(fields: BodyFields<Self>) -> Self::Body {
        Rc::new(fields)
    }

    // Nodes only run code of their own in a game's tree, and the
    // interpreter runs no game yet (Phase 14.6b).
    fn start(_node: &Self::Node) {}

    fn update(_node: &Self::Node, _delta: f32) {}

    fn game(game: &Self::Game) -> &GameFields<Self> {
        match *game {}
    }

    fn start_game(game: &Self::Game) {
        match *game {}
    }

    fn update_game(game: &Self::Game, _delta: f32) {
        match *game {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_cameras_are_seen_as_cameras() {
        let node = |class: EngineClass| {
            Rc::new(Object::new(
                class.class(),
                Vec::new(),
                NodeParts::new(class, "Seen".to_owned()),
            ))
        };
        let camera = node(EngineClass::Camera2D);

        let seen = Interpreted::as_camera_2d(&camera).expect("a camera");

        assert!(Rc::ptr_eq(&Interpreted::camera_2d_node(&seen), &camera));
        assert!(Interpreted::as_camera_2d(&node(EngineClass::Node)).is_none());
    }

    #[test]
    fn colors_made_by_the_runtime_are_engine_colors() {
        let color = Interpreted::color(0.1, 0.2, 0.3, 0.4);

        assert_eq!(color.class, EngineClass::Color.class());
        assert_eq!(Interpreted::channels(&color), [0.1, 0.2, 0.3, 0.4]);
    }
}
