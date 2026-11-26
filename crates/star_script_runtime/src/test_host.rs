//! A [`Host`] made of plain `Rc`s, as a way of running StarScript could
//! have, for the tests of what is generic over hosts.

use std::cell::RefCell;
use std::rc::{Rc, Weak};

use crate::List;
use crate::bodies::BodyFields;
use crate::game::GameFields;
use crate::node_classes::{
    AnimatorFields, Camera2dFields, Camera3dFields, LightFields, MeshFields, SpriteFields,
};
use crate::nodes::{self, Host, Node3dFields, NodeFields};
use crate::scene_fields::SceneValue;
use crate::scene_tree::NewNode;

pub(crate) struct TestHost;

/// A node of any class: the parts it does not have are `None`.
pub(crate) struct TestNode {
    pub(crate) node: NodeFields<TestHost>,
    /// Only used by nodes placed in 3D.
    pub(crate) node_3d: Node3dFields,
    pub(crate) is_3d: bool,
    pub(crate) camera: Option<Camera2dFields>,
    pub(crate) camera_3d: Option<Camera3dFields>,
    pub(crate) sprite: Option<SpriteFields<TestHost>>,
    pub(crate) animator: Option<AnimatorFields>,
    /// The methods the host ran on it, in order.
    pub(crate) calls: RefCell<Vec<String>>,
    /// A node its next `update` takes out of the tree.
    pub(crate) removes: RefCell<Option<Rc<TestNode>>>,
}

impl TestNode {
    /// A plain node placed in 2D.
    pub(crate) fn new(name: &str) -> Self {
        Self {
            node: NodeFields::new(name.to_owned()),
            node_3d: Node3dFields::new(),
            is_3d: false,
            camera: None,
            camera_3d: None,
            sprite: None,
            animator: None,
            calls: RefCell::new(Vec::new()),
            removes: RefCell::new(None),
        }
    }

    /// A `Node3D`.
    pub(crate) fn new_3d(name: &str) -> Self {
        Self {
            is_3d: true,
            ..Self::new(name)
        }
    }

    pub(crate) fn calls(&self) -> Vec<String> {
        self.calls.borrow().clone()
    }
}

/// A game, which records what the host ran on it.
pub(crate) struct TestGame {
    pub(crate) fields: GameFields<TestHost>,
    pub(crate) calls: RefCell<Vec<String>>,
}

impl Host for TestHost {
    type Node = Rc<TestNode>;
    type WeakNode = Weak<TestNode>;
    type Node3D = Rc<TestNode>;
    type Camera2D = Rc<TestNode>;
    type Camera3D = Rc<TestNode>;
    type Body = Rc<BodyFields<Self>>;
    type Color = [f32; 4];
    type Game = Rc<TestGame>;
    type Tags = List<String>;

    fn node(node: &Self::Node) -> &NodeFields<Self> {
        &node.node
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

    fn as_node_3d(node: &Self::Node) -> Option<Self::Node3D> {
        node.is_3d.then(|| Rc::clone(node))
    }

    fn node_3d(node: &Self::Node3D) -> &Node3dFields {
        &node.node_3d
    }

    fn as_camera_2d(node: &Self::Node) -> Option<Self::Camera2D> {
        node.camera.is_some().then(|| Rc::clone(node))
    }

    fn camera_2d(camera: &Self::Camera2D) -> &Camera2dFields {
        camera.camera.as_ref().expect("a camera")
    }

    fn camera_2d_node(camera: &Self::Camera2D) -> Self::Node {
        Rc::clone(camera)
    }

    fn as_camera_3d(node: &Self::Node) -> Option<Self::Camera3D> {
        node.camera_3d.is_some().then(|| Rc::clone(node))
    }

    fn camera_3d(camera: &Self::Camera3D) -> &Camera3dFields {
        camera.camera_3d.as_ref().expect("a 3D camera")
    }

    fn camera_3d_node(camera: &Self::Camera3D) -> Self::Node {
        Rc::clone(camera)
    }

    fn with_sprite<R>(node: &Self::Node, read: impl FnOnce(&SpriteFields<Self>) -> R) -> Option<R> {
        node.sprite.as_ref().map(read)
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

    fn with_animator<R>(node: &Self::Node, read: impl FnOnce(&AnimatorFields) -> R) -> Option<R> {
        node.animator.as_ref().map(read)
    }

    fn new_tags(tags: Vec<String>) -> Self::Tags {
        List::from_vec(tags)
    }

    fn has_tag(tags: &Self::Tags, tag: &str) -> bool {
        tags.any(|own| own == tag)
    }

    fn body(body: &Self::Body) -> &BodyFields<Self> {
        body
    }

    fn color(r: f32, g: f32, b: f32, a: f32) -> Self::Color {
        [r, g, b, a]
    }

    fn channels(color: &Self::Color) -> [f32; 4] {
        *color
    }

    fn create(name: String, class: NewNode<'_>) -> Result<Self::Node, String> {
        let mut node = TestNode::new(&name);
        match class {
            NewNode::Node | NewNode::Script("Enemy") => {}
            NewNode::Node3D => node.is_3d = true,
            NewNode::Camera2D => node.camera = Some(Camera2dFields::new()),
            NewNode::Camera3D => {
                node.is_3d = true;
                node.camera_3d = Some(Camera3dFields::new());
            }
            NewNode::Sprite { texture } => {
                node.sprite = Some(SpriteFields::new(texture.to_owned()));
            }
            NewNode::Animator => node.animator = Some(AnimatorFields::new()),
            other => return Err(format!("the test host cannot create {other:?}")),
        }
        Ok(Rc::new(node))
    }

    fn set_exported(
        class: &str,
        node: &Self::Node,
        values: &[(String, SceneValue)],
    ) -> Result<(), String> {
        for (field, value) in values {
            match (class, field.as_str(), value) {
                // Recorded, so the tests can see it was given.
                ("Enemy", "speed", SceneValue::Float(speed)) => {
                    node.calls.borrow_mut().push(format!("speed {speed}"));
                }
                _ => return Err(format!("`{class}` has no `export var {field}`")),
            }
        }
        Ok(())
    }

    /// The test nodes are named after the class they stand for.
    fn set_exported_field(
        node: &Self::Node,
        field: &str,
        value: &SceneValue,
    ) -> Result<(), String> {
        let class = node.node.name.get();
        Self::set_exported(&class, node, &[(field.to_owned(), value.clone())])
    }

    fn new_body(fields: BodyFields<Self>) -> Self::Body {
        Rc::new(fields)
    }

    fn start(node: &Self::Node) {
        node.calls.borrow_mut().push("start".to_owned());
    }

    fn update(node: &Self::Node, delta: f32) {
        node.calls.borrow_mut().push(format!("update {delta}"));
        if let Some(removed) = node.removes.take() {
            nodes::remove::<Self>(&removed);
        }
    }

    fn animation_finished(node: &Self::Node, name: &str) {
        node.calls.borrow_mut().push(format!("finished {name}"));
    }

    fn game(game: &Self::Game) -> &GameFields<Self> {
        &game.fields
    }

    fn start_game(game: &Self::Game) {
        game.calls.borrow_mut().push("start".to_owned());
    }

    fn update_game(game: &Self::Game, delta: f32) {
        game.calls.borrow_mut().push(format!("update {delta}"));
    }
}
