//! The interpreter as a [`Host`] of `star_script_runtime` (decision 70):
//! what its nodes, colors, bodies and game are, so the runtime's fields,
//! its code of nodes and `star_script_engine`'s game loop work on them as
//! on the compiled game's. A game is played with
//! `star_script_engine::game::run::<Interpreted>` and a game from
//! [`crate::Program::new_game`].
//!
//! Its types are public only so the engine can name them; what they hold
//! stays the interpreter's. The engine classes of later steps of Phase 14
//! (`Node3D`, `Camera3D`, `Mesh` and `Light`) are [`NotYet`] or never
//! found here: no value of them exists, so the runtime is never handed
//! one.

use std::cell::RefCell;
use std::rc::{Rc, Weak};
use std::sync::Arc;

use star_script_runtime::audio::SoundFields;
use star_script_runtime::bodies::BodyFields;
use star_script_runtime::color::ColorFields;
use star_script_runtime::game::GameFields;
use star_script_runtime::node_classes::{
    Camera2dFields, Camera3dFields, LightFields, MeshFields, SpriteFields,
};
use star_script_runtime::nodes::{Host, Node3dFields, NodeFields};
use star_script_runtime::scene_fields::SceneValue;
use star_script_runtime::scene_tree::NewNode;

use crate::play;
use crate::run::{self, Hook};
use crate::tree::EngineClass;
use crate::value::{Elements, Value};

pub use crate::value::Object;

/// The interpreter's objects, as the runtime sees them.
pub enum Interpreted {}

/// An engine class the interpreter cannot make objects of yet.
#[derive(Clone, Copy)]
pub enum NotYet {}

/// An object of `Node`, `Camera2D` or a class that extends one.
pub type NodeObject = Object<NodeParts>;

/// The game: an object of the class that extends `Game`.
pub type GameObject = Object<GameFields<Interpreted>>;

/// A `Body`: an object with the runtime's fields of bodies (decision 72).
pub type BodyObject = Object<BodyFields<Interpreted>>;

/// A `Sound`: an object with the runtime's fields of sounds.
pub type SoundObject = Object<SoundFields>;

/// The runtime's fields of a node: those of `Node`, and those of its node
/// class.
pub struct NodeParts {
    pub(crate) node: NodeFields<Interpreted>,
    pub(crate) class: ClassParts,
}

/// The runtime's fields of the node class a node is, beyond `Node`'s.
pub(crate) enum ClassParts {
    /// A plain `Node`.
    Node,
    /// A `Camera2D`, whose fields are kept apart so a camera can be handed
    /// out with them.
    Camera2D(Rc<Camera2dFields>),
    Sprite(SpriteFields<Interpreted>),
    Light2D(LightFields<Interpreted>),
}

impl NodeParts {
    /// The fields of a new node called `name`, with those of its class.
    pub(crate) fn new(name: String, class: ClassParts) -> Self {
        Self {
            node: NodeFields::new(name),
            class,
        }
    }

    /// The fields of `Camera2D`, if it is one.
    pub(crate) const fn camera_2d(&self) -> Option<&Rc<Camera2dFields>> {
        match &self.class {
            ClassParts::Camera2D(fields) => Some(fields),
            _ => None,
        }
    }

    /// The fields of `Sprite`, if it is one.
    pub(crate) const fn sprite(&self) -> Option<&SpriteFields<Interpreted>> {
        match &self.class {
            ClassParts::Sprite(fields) => Some(fields),
            _ => None,
        }
    }

    /// The fields of `Light2D`, if it is one.
    pub(crate) const fn light_2d(&self) -> Option<&LightFields<Interpreted>> {
        match &self.class {
            ClassParts::Light2D(fields) => Some(fields),
            _ => None,
        }
    }
}

/// A node's `tags`: a list of the interpreter, so a `push` of a script is
/// seen by the runtime's `has_tag` and `find_by_tag`.
#[derive(Clone)]
pub struct Tags(pub(crate) Elements);

/// A `Camera2D`: the node, and its fields of camera.
#[derive(Clone)]
pub struct CameraRef {
    node: Rc<NodeObject>,
    fields: Rc<Camera2dFields>,
}

impl CameraRef {
    /// The camera as a node.
    pub(crate) const fn node(&self) -> &Rc<NodeObject> {
        &self.node
    }
}

impl Host for Interpreted {
    type Node = Rc<NodeObject>;
    type WeakNode = Weak<NodeObject>;
    type Node3D = NotYet;
    type Camera2D = CameraRef;
    type Camera3D = NotYet;
    type Body = Rc<BodyObject>;
    type Color = Rc<Object<ColorFields>>;
    type Game = Rc<GameObject>;
    type Tags = Tags;

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
        let fields = node.engine.camera_2d()?;
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

    fn with_sprite<R>(node: &Self::Node, read: impl FnOnce(&SpriteFields<Self>) -> R) -> Option<R> {
        node.engine.sprite().map(read)
    }

    fn with_mesh<R>(_node: &Self::Node, _read: impl FnOnce(&MeshFields<Self>) -> R) -> Option<R> {
        None
    }

    fn with_light<R>(_node: &Self::Node, _read: impl FnOnce(&LightFields<Self>) -> R) -> Option<R> {
        None
    }

    fn with_light_2d<R>(
        node: &Self::Node,
        read: impl FnOnce(&LightFields<Self>) -> R,
    ) -> Option<R> {
        node.engine.light_2d().map(read)
    }

    fn new_tags(tags: Vec<String>) -> Self::Tags {
        let elements = tags
            .into_iter()
            .map(|tag| Value::String(Arc::from(tag)))
            .collect();
        Tags(Rc::new(RefCell::new(elements)))
    }

    fn has_tag(tags: &Self::Tags, tag: &str) -> bool {
        // The interpreter never runs the runtime while it holds a list.
        tags.0.try_borrow().is_ok_and(|elements| {
            elements
                .iter()
                .any(|element| matches!(element, Value::String(own) if &**own == tag))
        })
    }

    fn body(body: &Self::Body) -> &BodyFields<Self> {
        &body.engine
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

    fn create(name: String, class: NewNode<'_>) -> Result<Self::Node, String> {
        run::create(name, class)
    }

    fn set_exported(
        class: &str,
        node: &Self::Node,
        values: &[(String, SceneValue)],
    ) -> Result<(), String> {
        run::set_exported(class, node, values)
    }

    fn new_body(fields: BodyFields<Self>) -> Self::Body {
        Rc::new(Object::new(EngineClass::Body.class(), Vec::new(), fields))
    }

    fn start(node: &Self::Node) {
        play::run_hook(Value::Node(Rc::clone(node)), Hook::Start);
    }

    fn update(node: &Self::Node, delta: f32) {
        play::run_hook(Value::Node(Rc::clone(node)), Hook::Update(delta));
    }

    fn game(game: &Self::Game) -> &GameFields<Self> {
        &game.engine
    }

    fn start_game(game: &Self::Game) {
        play::run_hook(Value::Game(Rc::clone(game)), Hook::Start);
    }

    fn update_game(game: &Self::Game, delta: f32) {
        play::run_hook(Value::Game(Rc::clone(game)), Hook::Update(delta));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_cameras_are_seen_as_cameras() {
        let node = |class: EngineClass, parts: ClassParts| {
            Rc::new(Object::new(
                class.class(),
                Vec::new(),
                NodeParts::new("Seen".to_owned(), parts),
            ))
        };
        let camera = node(
            EngineClass::Camera2D,
            ClassParts::Camera2D(Rc::new(Camera2dFields::new())),
        );

        let seen = Interpreted::as_camera_2d(&camera).expect("a camera");

        assert!(Rc::ptr_eq(&Interpreted::camera_2d_node(&seen), &camera));
        assert!(Interpreted::as_camera_2d(&node(EngineClass::Node, ClassParts::Node)).is_none());
    }

    #[test]
    fn tags_are_the_interpreter_s_lists() {
        let tags = Interpreted::new_tags(vec!["enemy".to_owned()]);

        tags.0.borrow_mut().push(Value::text("loud"));

        assert!(Interpreted::has_tag(&tags, "enemy"));
        assert!(Interpreted::has_tag(&tags, "loud"));
        assert!(!Interpreted::has_tag(&tags, "quiet"));
    }

    #[test]
    fn bodies_made_by_the_runtime_are_engine_bodies() {
        let body = Interpreted::new_body(BodyFields::circle("static".to_owned(), 2.0));

        assert_eq!(body.class, EngineClass::Body.class());
        assert!((Interpreted::body(&body).radius.get() - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn colors_made_by_the_runtime_are_engine_colors() {
        let color = Interpreted::color(0.1, 0.2, 0.3, 0.4);

        assert_eq!(color.class, EngineClass::Color.class());
        assert_eq!(Interpreted::channels(&color), [0.1, 0.2, 0.3, 0.4]);
    }
}
