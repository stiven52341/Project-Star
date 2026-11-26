//! The tree of StarScript's `Node`s: the fields every node has, and what
//! the methods of `Node` do with them.
//!
//! It is written once for every way of running StarScript. Each one says
//! what a node is with a [`Host`] — the generated Rust gives its `Rc`s of
//! classes, the interpreter its own objects — and these functions
//! do the rest. It needs no engine, so console programs use it too.

use std::cell::{Cell, Ref, RefCell};
use std::io::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::bodies::BodyFields;
use crate::game::GameFields;
use crate::node_classes::{
    AnimatorFields, Camera2dFields, Camera3dFields, LightFields, MeshFields, SpriteFields,
};
use crate::placement::{Place2d, Place3d};
use crate::scene_fields::SceneValue;
use crate::scene_tree::NewNode;
use crate::{Field, List};

/// What the objects of a way of running StarScript are: references to
/// its nodes, bodies, colors and game, how to reach their fields, and how
/// to run their scripts' code.
pub trait Host: Sized + 'static {
    /// A reference to a node: an object of `Node` or of a subclass.
    type Node: Clone;
    /// A reference to a node that does not keep it alive.
    type WeakNode;
    /// A reference to a node placed in 3D: a `Node3D` or a subclass.
    type Node3D;
    /// A reference to a `Camera2D` or a subclass.
    type Camera2D: Clone;
    /// A reference to a `Camera3D` or a subclass.
    type Camera3D: Clone;
    /// A reference to a body, which a node may have.
    type Body: Clone;
    /// A reference to a color.
    type Color: Clone;
    /// A reference to the game: a `Game` or a subclass.
    type Game;
    /// A node's list of tags: one list that scripts change and the runtime
    /// reads, so it is the way of running StarScript's own list of texts.
    type Tags: Clone;

    /// The fields of `Node` in `node`.
    fn node(node: &Self::Node) -> &NodeFields<Self>;
    /// A reference to `node` that does not keep it alive.
    fn downgrade(node: &Self::Node) -> Self::WeakNode;
    /// The node `weak` refers to, if it is still alive.
    fn upgrade(weak: &Self::WeakNode) -> Option<Self::Node>;
    /// Whether `node` and `other` are the same object.
    fn same(node: &Self::Node, other: &Self::Node) -> bool;
    /// `node` as a `Node3D`, if it is one.
    fn as_node_3d(node: &Self::Node) -> Option<Self::Node3D>;
    /// The fields of `Node3D` in `node`.
    fn node_3d(node: &Self::Node3D) -> &Node3dFields;
    /// `node` as a `Camera2D`, if it is one.
    fn as_camera_2d(node: &Self::Node) -> Option<Self::Camera2D>;
    /// The fields of `Camera2D` in `camera`.
    fn camera_2d(camera: &Self::Camera2D) -> &Camera2dFields;
    /// `camera` as a node.
    fn camera_2d_node(camera: &Self::Camera2D) -> Self::Node;
    /// `node` as a `Camera3D`, if it is one.
    fn as_camera_3d(node: &Self::Node) -> Option<Self::Camera3D>;
    /// The fields of `Camera3D` in `camera`.
    fn camera_3d(camera: &Self::Camera3D) -> &Camera3dFields;
    /// `camera` as a node.
    fn camera_3d_node(camera: &Self::Camera3D) -> Self::Node;
    /// Runs `read` on the fields of `Sprite` in `node`, if it is one.
    fn with_sprite<R>(node: &Self::Node, read: impl FnOnce(&SpriteFields<Self>) -> R) -> Option<R>;
    /// Runs `read` on the fields of `Mesh` in `node`, if it is one.
    fn with_mesh<R>(node: &Self::Node, read: impl FnOnce(&MeshFields<Self>) -> R) -> Option<R>;
    /// Runs `read` on the fields of `Light` in `node`, if it is one.
    fn with_light<R>(node: &Self::Node, read: impl FnOnce(&LightFields<Self>) -> R) -> Option<R>;
    /// Runs `read` on the fields of `Light2D` in `node`, if it is one.
    fn with_light_2d<R>(node: &Self::Node, read: impl FnOnce(&LightFields<Self>) -> R)
    -> Option<R>;
    /// Runs `read` on the fields of `Animator` in `node`, if it is one.
    fn with_animator<R>(node: &Self::Node, read: impl FnOnce(&AnimatorFields) -> R) -> Option<R>;
    /// A new list of tags holding `tags`.
    fn new_tags(tags: Vec<String>) -> Self::Tags;
    /// Whether `tags` holds `tag`.
    fn has_tag(tags: &Self::Tags, tag: &str) -> bool;
    /// The fields of the body `body`.
    fn body(body: &Self::Body) -> &BodyFields<Self>;
    /// A new color with the channels `r`, `g`, `b` and `a`.
    fn color(r: f32, g: f32, b: f32, a: f32) -> Self::Color;
    /// The channels of `color`, in the order `[r, g, b, a]`.
    fn channels(color: &Self::Color) -> [f32; 4];
    /// A new node of `class`, called `name`, as its `init` leaves it.
    ///
    /// # Errors
    ///
    /// Why a class of the program cannot be created for a scene: it does
    /// not exist, is not a node, or its `init` needs arguments.
    fn create(name: String, class: NewNode<'_>) -> Result<Self::Node, String>;
    /// Gives the `export` fields of `node`, an object of `class`, the
    /// values a scene writes.
    ///
    /// # Errors
    ///
    /// Why a value cannot be given: the class has no such `export` field,
    /// or the value does not fit it.
    fn set_exported(
        class: &str,
        node: &Self::Node,
        values: &[(String, SceneValue)],
    ) -> Result<(), String>;
    /// Gives the `export` field `field` of the class of `node` the value
    /// `value`, as an animation does.
    ///
    /// # Errors
    ///
    /// Why it cannot be given: the class of `node` has no such `export`
    /// field, or the value does not fit it.
    fn set_exported_field(node: &Self::Node, field: &str, value: &SceneValue)
    -> Result<(), String>;
    /// Runs the method `method` of `node`, which takes no arguments, as an
    /// animation does: its script's version.
    ///
    /// # Errors
    ///
    /// Why it cannot run: the class of `node` has no such method, or it
    /// takes arguments.
    fn call_method(node: &Self::Node, method: &str) -> Result<(), String>;
    /// A new body with the fields `fields`.
    fn new_body(fields: BodyFields<Self>) -> Self::Body;
    /// Runs the `start` of `node`: its script's, or `Node`'s.
    fn start(node: &Self::Node);
    /// Runs the `update` of `node`: its script's, or `Node`'s.
    fn update(node: &Self::Node, delta: f32);
    /// Runs the `animation_finished` of `node`, if it is a sprite or an
    /// animator: its script's, or the engine class's (nothing).
    fn animation_finished(node: &Self::Node, name: &str);
    /// The fields of `Game` in `game`.
    fn game(game: &Self::Game) -> &GameFields<Self>;
    /// Runs the `start` of `game`.
    fn start_game(game: &Self::Game);
    /// Runs the `update` of `game`.
    fn update_game(game: &Self::Game, delta: f32);
}

/// Gives every node its own number, which the engine knows it by.
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

/// The fields of a node: the public ones, and its place in the tree.
pub struct NodeFields<H: Host> {
    /// `name`
    pub name: Field<String>,
    /// `x`
    pub x: Field<f32>,
    /// `y`
    pub y: Field<f32>,
    /// `rotation`, in radians.
    pub rotation: Field<f32>,
    /// `scale_x`
    pub scale_x: Field<f32>,
    /// `scale_y`
    pub scale_y: Field<f32>,
    /// `visible`: whether it, and its children, are drawn.
    pub visible: Field<bool>,
    /// `body`
    pub body: Field<Option<H::Body>>,
    /// `tags`
    pub tags: Field<H::Tags>,
    id: u64,
    parent: RefCell<Option<H::WeakNode>>,
    children: RefCell<Vec<H::Node>>,
    started: Cell<bool>,
}

impl<H: Host> NodeFields<H> {
    /// The fields of a new node called `name`, at the origin, out of any
    /// tree, with a number no other node has.
    #[must_use]
    pub fn new(name: String) -> Self {
        Self {
            name: Field::new(name),
            x: Field::new(0.0),
            y: Field::new(0.0),
            rotation: Field::new(0.0),
            scale_x: Field::new(1.0),
            scale_y: Field::new(1.0),
            visible: Field::new(true),
            body: Field::new(None),
            tags: Field::new(H::new_tags(Vec::new())),
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            parent: RefCell::new(None),
            children: RefCell::new(Vec::new()),
            started: Cell::new(false),
        }
    }

    /// The node's own number.
    #[must_use]
    pub const fn id(&self) -> u64 {
        self.id
    }

    /// The node's children, in order. Nothing may change the tree while
    /// they are borrowed.
    #[must_use]
    pub fn children(&self) -> Ref<'_, Vec<H::Node>> {
        self.children.borrow()
    }

    /// Marks the node as started, and tells whether it was not yet: its
    /// `start` runs only then.
    pub fn start_once(&self) -> bool {
        !self.started.replace(true)
    }

    fn has_tag(&self, tag: &str) -> bool {
        self.tags.read(|tags| H::has_tag(tags, tag))
    }

    fn is_named(&self, name: &str) -> bool {
        self.name.read(|own| own == name)
    }
}

/// The fields `Node3D` adds: its depth, its pitch and yaw (its roll is
/// `rotation`) and its depth scale.
pub struct Node3dFields {
    /// `z`
    pub z: Field<f32>,
    /// `pitch`, in radians.
    pub pitch: Field<f32>,
    /// `yaw`, in radians.
    pub yaw: Field<f32>,
    /// `scale_z`
    pub scale_z: Field<f32>,
}

impl Node3dFields {
    /// At depth 0, not turned, not scaled.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            z: Field::new(0.0),
            pitch: Field::new(0.0),
            yaw: Field::new(0.0),
            scale_z: Field::new(1.0),
        }
    }
}

impl Default for Node3dFields {
    fn default() -> Self {
        Self::new()
    }
}

/// `func add_child`: moves `child`, with its own children, under `this`.
/// A node cannot go under itself or one of its children: it stays where it
/// is, with a warning.
pub fn add_child<H: Host>(this: &H::Node, child: H::Node) {
    if is_same_or_ancestor::<H>(&child, this) {
        warn("a node cannot be added under itself or one of its children");
        return;
    }
    remove::<H>(&child);
    *H::node(&child).parent.borrow_mut() = Some(H::downgrade(this));
    H::node(this).children.borrow_mut().push(child);
}

/// `func remove`: takes the node, with its children, out of the tree.
pub fn remove<H: Host>(this: &H::Node) {
    let parent = H::node(this).parent.borrow_mut().take();
    if let Some(parent) = parent.as_ref().and_then(H::upgrade) {
        H::node(&parent)
            .children
            .borrow_mut()
            .retain(|child| !H::same(child, this));
    }
}

/// `func parent`
#[must_use]
pub fn parent<H: Host>(this: &H::Node) -> Option<H::Node> {
    H::node(this).parent.borrow().as_ref().and_then(H::upgrade)
}

/// `func children`: a new list, so changing it does not change the tree.
#[must_use]
pub fn children<H: Host>(this: &H::Node) -> List<H::Node> {
    List::from_vec(H::node(this).children().clone())
}

/// `func find_child`: the first child called `name`.
#[must_use]
pub fn find_child<H: Host>(this: &H::Node, name: &str) -> Option<H::Node> {
    H::node(this)
        .children()
        .iter()
        .find(|child| H::node(child).is_named(name))
        .cloned()
}

/// `func has_tag`
#[must_use]
pub fn has_tag<H: Host>(this: &H::Node, tag: &str) -> bool {
    H::node(this).has_tag(tag)
}

/// `func find_by_name`: the first descendant called `name`, parents before
/// their children and children in order.
#[must_use]
pub fn find_by_name<H: Host>(this: &H::Node, name: &str) -> Option<H::Node> {
    descendants::<H>(this)
        .into_iter()
        .find(|node| H::node(node).is_named(name))
}

/// `func find_by_tag`: every descendant with the tag `tag`, parents before
/// their children and children in order.
#[must_use]
pub fn find_by_tag<H: Host>(this: &H::Node, tag: &str) -> List<H::Node> {
    let tagged = descendants::<H>(this)
        .into_iter()
        .filter(|node| H::node(node).has_tag(tag))
        .collect();
    List::from_vec(tagged)
}

/// The descendants of `node`, not itself: each parent before its
/// children, and children in their order.
#[must_use]
pub fn descendants<H: Host>(node: &H::Node) -> Vec<H::Node> {
    let mut found = Vec::new();
    let mut pending: Vec<H::Node> = H::node(node).children().iter().rev().cloned().collect();
    while let Some(next) = pending.pop() {
        pending.extend(H::node(&next).children().iter().rev().cloned());
        found.push(next);
    }
    found
}

/// Whether `node` is `other` or one of its ancestors.
#[must_use]
pub fn is_same_or_ancestor<H: Host>(node: &H::Node, other: &H::Node) -> bool {
    let mut current = Some(other.clone());
    while let Some(candidate) = current {
        if H::same(&candidate, node) {
            return true;
        }
        current = parent::<H>(&candidate);
    }
    false
}

/// `func translate`: moves the node in its parent's space.
pub fn translate<H: Host>(this: &H::Node, x: f32, y: f32) {
    let fields = H::node(this);
    fields.x.set(fields.x.get() + x);
    fields.y.set(fields.y.get() + y);
}

/// `func rotate`: turns the node a little more.
pub fn rotate<H: Host>(this: &H::Node, angle: f32) {
    let fields = H::node(this);
    fields.rotation.set(fields.rotation.get() + angle);
}

/// Where `node` is in the world: in 3D for a `Node3D`, else in 2D with a
/// depth of 0.
#[must_use]
pub fn global_position<H: Host>(node: &H::Node) -> [f32; 3] {
    let fields = H::node(node);
    if let Some(node_3d) = H::as_node_3d(node) {
        let depth = H::node_3d(&node_3d).z.get();
        return parent_place_3d::<H>(node).point([fields.x.get(), fields.y.get(), depth]);
    }
    let [x, y] = parent_place_2d::<H>(node).point([fields.x.get(), fields.y.get()]);
    [x, y, 0.0]
}

/// `func global_rotation`: how much the node is turned in the world: its
/// `rotation` and those of its ancestors placed as it is (in 2D, or in 3D
/// for a `Node3D`, where it is its roll).
#[must_use]
pub fn global_rotation<H: Host>(this: &H::Node) -> f32 {
    let in_3d = is_3d::<H>(this);
    let inherited: f32 = ancestors::<H>(this)
        .iter()
        .filter(|ancestor| is_3d::<H>(ancestor) == in_3d)
        .map(|ancestor| H::node(ancestor).rotation.get())
        .sum();
    inherited + H::node(this).rotation.get()
}

/// `func set_global_position`: places the node at a point of the world; a
/// `Node3D` keeps its depth in the world. A node whose parent is scaled to
/// nothing stays where it is.
pub fn set_global_position<H: Host>(this: &H::Node, x: f32, y: f32) {
    if is_3d::<H>(this) {
        let [_, _, depth] = global_position::<H>(this);
        set_global_position_3d::<H>(this, [x, y, depth]);
    } else if let Some([local_x, local_y]) = parent_place_2d::<H>(this).local_point([x, y]) {
        let fields = H::node(this);
        fields.x.set(local_x);
        fields.y.set(local_y);
    }
}

/// Places the `Node3D` `node` at `point` of the world; it stays where it is
/// if it is not a `Node3D` or its parent is scaled to nothing.
pub fn set_global_position_3d<H: Host>(node: &H::Node, point: [f32; 3]) {
    let Some(node_3d) = H::as_node_3d(node) else {
        return;
    };
    if let Some([x, y, z]) = parent_place_3d::<H>(node).local_point(point) {
        let fields = H::node(node);
        fields.x.set(x);
        fields.y.set(y);
        H::node_3d(&node_3d).z.set(z);
    }
}

/// `func set_global_rotation`: turns the node to `angle` in the world.
pub fn set_global_rotation<H: Host>(this: &H::Node, angle: f32) {
    let rotation = &H::node(this).rotation;
    let inherited = global_rotation::<H>(this) - rotation.get();
    rotation.set(angle - inherited);
}

/// `func translate_global`: moves the node in the world.
pub fn translate_global<H: Host>(this: &H::Node, x: f32, y: f32) {
    let [global_x, global_y, _] = global_position::<H>(this);
    set_global_position::<H>(this, global_x + x, global_y + y);
}

/// `func translate_3d`: moves the `Node3D` `this` in its parent's space;
/// other nodes do not move.
pub fn translate_3d<H: Host>(this: &H::Node, x: f32, y: f32, z: f32) {
    let Some(node_3d) = H::as_node_3d(this) else {
        return;
    };
    translate::<H>(this, x, y);
    let depth = &H::node_3d(&node_3d).z;
    depth.set(depth.get() + z);
}

/// `func translate_global_3d`: moves the `Node3D` `this` in the world.
pub fn translate_global_3d<H: Host>(this: &H::Node, x: f32, y: f32, z: f32) {
    let [global_x, global_y, global_z] = global_position::<H>(this);
    set_global_position_3d::<H>(this, [global_x + x, global_y + y, global_z + z]);
}

/// `func look_at`: turns the `Node3D` `this` so that its -z axis, where
/// cameras and lights look, points at a point of its parent's space. Its
/// roll is kept; looking at its own place does not turn it.
pub fn look_at<H: Host>(this: &H::Node, x: f32, y: f32, z: f32) {
    let Some(node_3d) = H::as_node_3d(this) else {
        return;
    };
    let (place, depth) = (H::node(this), H::node_3d(&node_3d));
    let (dx, dy, dz) = (x - place.x.get(), y - place.y.get(), z - depth.z.get());
    let length = (dx * dx + dy * dy + dz * dz).sqrt();
    if length > 0.0 && length.is_finite() {
        depth.pitch.set((dy / length).clamp(-1.0, 1.0).asin());
        depth.yaw.set((-dx).atan2(-dz));
    }
}

/// The ancestors of `node`, from the root down to its parent.
fn ancestors<H: Host>(node: &H::Node) -> Vec<H::Node> {
    let mut found = Vec::new();
    let mut current = parent::<H>(node);
    while let Some(ancestor) = current {
        current = parent::<H>(&ancestor);
        found.push(ancestor);
    }
    found.reverse();
    found
}

fn is_3d<H: Host>(node: &H::Node) -> bool {
    H::as_node_3d(node).is_some()
}

/// Where the 2D space a 2D node is placed in is in the world: its
/// parent's, through its ancestors placed in 2D, as the engine does.
fn parent_place_2d<H: Host>(node: &H::Node) -> Place2d {
    ancestors::<H>(node)
        .iter()
        .filter(|ancestor| !is_3d::<H>(ancestor))
        .fold(Place2d::IDENTITY, |place, ancestor| {
            place.then(place_2d::<H>(ancestor))
        })
}

fn place_2d<H: Host>(node: &H::Node) -> Place2d {
    let fields = H::node(node);
    Place2d::of_node(
        fields.x.get(),
        fields.y.get(),
        fields.rotation.get(),
        fields.scale_x.get(),
        fields.scale_y.get(),
    )
}

/// Where the 3D space a `Node3D` is placed in is in the world: its
/// parent's, through its ancestors placed in 3D.
fn parent_place_3d<H: Host>(node: &H::Node) -> Place3d {
    ancestors::<H>(node)
        .iter()
        .filter_map(place_3d::<H>)
        .fold(Place3d::IDENTITY, Place3d::then)
}

/// The place of `node` in its parent's 3D space, if it is a `Node3D`.
fn place_3d<H: Host>(node: &H::Node) -> Option<Place3d> {
    let node_3d = H::as_node_3d(node)?;
    let fields = H::node(node);
    let depth = H::node_3d(&node_3d);
    Some(Place3d::of_node(
        [fields.x.get(), fields.y.get(), depth.z.get()],
        [depth.pitch.get(), depth.yaw.get(), fields.rotation.get()],
        [
            fields.scale_x.get(),
            fields.scale_y.get(),
            depth.scale_z.get(),
        ],
    ))
}

fn warn(message: &str) {
    // Nothing can be reported if the error output itself fails.
    let _ = writeln!(std::io::stderr(), "StarScript warning: {message}");
}

#[cfg(test)]
mod tests;
