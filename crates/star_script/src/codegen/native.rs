//! Modules of the engine classes that scripts can extend, and the code that
//! runs a game in the engine.
//!
//! Their methods have no StarScript source, so their modules are written
//! here, with the same shape as the modules of script classes.

use super::conversions::RUNTIME;
use super::native_nodes;
use super::writer::Writer;
use crate::InputAction;

/// The crate that connects StarScript's runtime with the engine.
const SCRIPT_ENGINE: &str = "::star_script_engine";

/// The module of the engine class `Game`.
pub(super) fn game_module(out: &mut Writer) {
    for line in GAME_MODULE.lines() {
        out.line(line);
    }
}

const GAME_MODULE: &str = "/// `class Game`, provided by the engine.
pub mod Game {
    /// A reference to an object of this class or of a subclass.
    pub type Ref = ::std::rc::Rc<dyn Object>;

    /// The fields of a game: the public ones, whether it asked to close, and
    /// the root of its tree of nodes.
    pub type Fields = ::star_script_runtime::game::GameFields<crate::classes::Host_>;

    /// Gives access to the fields of this class, in objects of any subclass.
    pub trait HasFields {
        fn fields(&self) -> &Fields;
    }

    /// What the engine calls: `start` once, `update` every frame.
    pub trait Object: crate::AnyObject + HasFields {
        fn start(self: ::std::rc::Rc<Self>);
        fn update(self: ::std::rc::Rc<Self>, delta: f32);
        fn quit(self: ::std::rc::Rc<Self>);
        fn add_child(self: ::std::rc::Rc<Self>, node: crate::classes::Node::Ref);
    }

    /// What games do unless they override it: nothing, until `quit`.
    pub struct Methods;

    impl Methods {
        /// `func start`
        pub fn start(this_: &Ref) {}

        /// `func update`
        pub fn update(this_: &Ref, delta: f32) {}

        /// `func quit`: closes the game after the current frame.
        pub fn quit(this_: &Ref) {
            HasFields::fields(&**this_).quit();
        }

        /// `func add_child`: puts `node` in the game's tree, at the top.
        pub fn add_child(this_: &Ref, node: crate::classes::Node::Ref) {
            HasFields::fields(&**this_).add_child(node);
        }
    }

    /// An object created as this class (not as a subclass).
    pub struct Instance {
        pub Game: Fields,
    }

    impl HasFields for Instance {
        fn fields(&self) -> &Fields { &self.Game }
    }

    impl crate::AnyObject for Instance {
        fn Game(self: ::std::rc::Rc<Self>) -> ::std::option::Option<Ref> { ::std::option::Option::Some(self) }
    }

    impl Object for Instance {
        fn start(self: ::std::rc::Rc<Self>) { Methods::start(&(self as Ref)) }
        fn update(self: ::std::rc::Rc<Self>, delta: f32) { Methods::update(&(self as Ref), delta) }
        fn quit(self: ::std::rc::Rc<Self>) { Methods::quit(&(self as Ref)) }
        fn add_child(self: ::std::rc::Rc<Self>, node: crate::classes::Node::Ref) { Methods::add_child(&(self as Ref), node) }
    }

    /// Prepares an object of this class: `make_` builds it from its fields.
    pub fn init_<O: Object + 'static>(make_: impl FnOnce(Fields) -> ::std::rc::Rc<O>) -> ::std::rc::Rc<O> {
        make_(Fields::new(crate::classes::Node::new_root_()))
    }

    /// Creates an object of this class.
    pub fn new() -> Ref {
        init_(|fields0_| ::std::rc::Rc::new(Instance { Game: fields0_ }))
    }
}";

/// How the generated classes are nodes for `star_script_runtime`, the
/// modules of the engine classes `Color`, `Body`, `Node`, `Scene` and
/// `Sound`, and those of the other node classes.
pub(super) fn node_modules(out: &mut Writer) {
    let modules = [
        HOST,
        COLOR_MODULE,
        BODY_MODULE,
        NODE_MODULE,
        SCENE_MODULE,
        SOUND_MODULE,
    ];
    for (index, module) in modules.iter().enumerate() {
        if index > 0 {
            out.line("");
        }
        for line in module.lines() {
            out.line(line);
        }
    }
    native_nodes::node_class_modules(out);
}

const COLOR_MODULE: &str = "/// `class Color`, provided by the engine.
pub mod Color {
    /// A reference to an object of this class.
    pub type Ref = ::std::rc::Rc<dyn Object>;

    /// The fields of a color: its channels, from `0.0` to `1.0`.
    pub type Fields = ::star_script_runtime::color::ColorFields;

    /// Gives access to the fields of this class.
    pub trait HasFields {
        fn fields(&self) -> &Fields;
    }

    /// A color has no methods.
    pub trait Object: crate::AnyObject + HasFields {}

    /// An object created as this class.
    pub struct Instance {
        pub Color: Fields,
    }

    impl HasFields for Instance {
        fn fields(&self) -> &Fields { &self.Color }
    }

    impl crate::AnyObject for Instance {
        fn Color(self: ::std::rc::Rc<Self>) -> ::std::option::Option<Ref> { ::std::option::Option::Some(self) }
    }

    impl Object for Instance {}

    /// Creates an object of this class.
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Ref {
        ::std::rc::Rc::new(Instance { Color: Fields::new(r, g, b, a) })
    }
}";

const BODY_MODULE: &str = "/// `class Body`, provided by the engine.
pub mod Body {
    /// A reference to an object of this class.
    pub type Ref = ::std::rc::Rc<dyn Object>;

    /// The fields of a body: the public ones, the impulses given during the
    /// frame, and what it touched in the last one.
    pub type Fields = ::star_script_runtime::bodies::BodyFields<crate::classes::Host_>;

    /// Gives access to the fields of this class.
    pub trait HasFields {
        fn fields(&self) -> &Fields;
    }

    /// What a body can do.
    pub trait Object: crate::AnyObject + HasFields {
        fn apply_impulse(self: ::std::rc::Rc<Self>, x: f32, y: f32);
        fn apply_impulse_3d(self: ::std::rc::Rc<Self>, x: f32, y: f32, z: f32);
        fn touching(self: ::std::rc::Rc<Self>) -> ::star_script_runtime::List<crate::classes::Node::Ref>;
        fn just_touched(self: ::std::rc::Rc<Self>) -> ::star_script_runtime::List<crate::classes::Node::Ref>;
        fn just_separated(self: ::std::rc::Rc<Self>) -> ::star_script_runtime::List<crate::classes::Node::Ref>;
    }

    /// The methods of a body: what `star_script_runtime::bodies` does with
    /// its fields.
    pub struct Methods;

    impl Methods {
        /// `func apply_impulse`
        pub fn apply_impulse(this_: &Ref, x: f32, y: f32) {
            HasFields::fields(&**this_).apply_impulse(x, y, 0.0);
        }

        /// `func apply_impulse_3d`
        pub fn apply_impulse_3d(this_: &Ref, x: f32, y: f32, z: f32) {
            HasFields::fields(&**this_).apply_impulse(x, y, z);
        }

        /// `func touching`
        pub fn touching(this_: &Ref) -> ::star_script_runtime::List<crate::classes::Node::Ref> {
            HasFields::fields(&**this_).contacts(::star_script_runtime::bodies::Contact::Touching)
        }

        /// `func just_touched`
        pub fn just_touched(this_: &Ref) -> ::star_script_runtime::List<crate::classes::Node::Ref> {
            HasFields::fields(&**this_).contacts(::star_script_runtime::bodies::Contact::JustTouched)
        }

        /// `func just_separated`
        pub fn just_separated(this_: &Ref) -> ::star_script_runtime::List<crate::classes::Node::Ref> {
            HasFields::fields(&**this_).contacts(::star_script_runtime::bodies::Contact::JustSeparated)
        }
    }

    /// An object created as this class.
    pub struct Instance {
        pub Body: Fields,
    }

    impl HasFields for Instance {
        fn fields(&self) -> &Fields { &self.Body }
    }

    impl crate::AnyObject for Instance {
        fn Body(self: ::std::rc::Rc<Self>) -> ::std::option::Option<Ref> { ::std::option::Option::Some(self) }
    }

    impl Object for Instance {
        fn apply_impulse(self: ::std::rc::Rc<Self>, x: f32, y: f32) { Methods::apply_impulse(&(self as Ref), x, y) }
        fn apply_impulse_3d(self: ::std::rc::Rc<Self>, x: f32, y: f32, z: f32) { Methods::apply_impulse_3d(&(self as Ref), x, y, z) }
        fn touching(self: ::std::rc::Rc<Self>) -> ::star_script_runtime::List<crate::classes::Node::Ref> { Methods::touching(&(self as Ref)) }
        fn just_touched(self: ::std::rc::Rc<Self>) -> ::star_script_runtime::List<crate::classes::Node::Ref> { Methods::just_touched(&(self as Ref)) }
        fn just_separated(self: ::std::rc::Rc<Self>) -> ::star_script_runtime::List<crate::classes::Node::Ref> { Methods::just_separated(&(self as Ref)) }
    }

    /// A body with the fields `fields`.
    pub fn of_(fields: Fields) -> Ref {
        ::std::rc::Rc::new(Instance { Body: fields })
    }

    /// `static func circle`
    pub fn circle(kind: ::std::string::String, radius: f32) -> Ref {
        of_(Fields::circle(kind, radius))
    }

    /// `static func rectangle`
    pub fn rectangle(kind: ::std::string::String, width: f32, height: f32) -> Ref {
        of_(Fields::rectangle(kind, width, height))
    }

    /// `static func capsule`
    pub fn capsule(kind: ::std::string::String, height: f32, radius: f32) -> Ref {
        of_(Fields::capsule(kind, height, radius))
    }

    /// `static func sphere`
    pub fn sphere(kind: ::std::string::String, radius: f32) -> Ref {
        of_(Fields::sphere(kind, radius))
    }

    /// `static func box`
    pub fn box_(kind: ::std::string::String, width: f32, height: f32, depth: f32) -> Ref {
        of_(Fields::cuboid(kind, width, height, depth))
    }
}";

const NODE_MODULE: &str = "/// `class Node`, provided by the engine.
pub mod Node {
    /// A reference to an object of this class or of a subclass.
    pub type Ref = ::std::rc::Rc<dyn Object>;

    /// The fields of a node: the public ones, and its place in the tree.
    pub type Fields = ::star_script_runtime::nodes::NodeFields<crate::classes::Host_>;

    /// Gives access to the fields of this class, in objects of any subclass.
    pub trait HasFields {
        fn fields(&self) -> &Fields;
    }

    /// What the engine calls and what subclasses may run their own version of.
    pub trait Object: crate::AnyObject + HasFields {
        fn start(self: ::std::rc::Rc<Self>);
        fn update(self: ::std::rc::Rc<Self>, delta: f32);
        fn add_child(self: ::std::rc::Rc<Self>, child: Ref);
        fn remove(self: ::std::rc::Rc<Self>);
        fn parent(self: ::std::rc::Rc<Self>) -> ::std::option::Option<Ref>;
        fn children(self: ::std::rc::Rc<Self>) -> ::star_script_runtime::List<Ref>;
        fn find_child(self: ::std::rc::Rc<Self>, name: ::std::string::String) -> ::std::option::Option<Ref>;
        fn has_tag(self: ::std::rc::Rc<Self>, tag: ::std::string::String) -> bool;
        fn find_by_name(self: ::std::rc::Rc<Self>, name: ::std::string::String) -> ::std::option::Option<Ref>;
        fn find_by_tag(self: ::std::rc::Rc<Self>, tag: ::std::string::String) -> ::star_script_runtime::List<Ref>;
        fn global_x(self: ::std::rc::Rc<Self>) -> f32;
        fn global_y(self: ::std::rc::Rc<Self>) -> f32;
        fn global_rotation(self: ::std::rc::Rc<Self>) -> f32;
        fn set_global_position(self: ::std::rc::Rc<Self>, x: f32, y: f32);
        fn set_global_rotation(self: ::std::rc::Rc<Self>, angle: f32);
        fn translate(self: ::std::rc::Rc<Self>, x: f32, y: f32);
        fn translate_global(self: ::std::rc::Rc<Self>, x: f32, y: f32);
        fn rotate(self: ::std::rc::Rc<Self>, angle: f32);
    }

    /// What nodes do unless a subclass overrides it: what
    /// `star_script_runtime::nodes` does with their fields.
    pub struct Methods;

    impl Methods {
        /// `func start`
        pub fn start(this_: &Ref) {}

        /// `func update`
        pub fn update(this_: &Ref, delta: f32) {}

        /// `func add_child`
        pub fn add_child(this_: &Ref, child: Ref) {
            ::star_script_runtime::nodes::add_child::<crate::classes::Host_>(this_, child);
        }

        /// `func remove`
        pub fn remove(this_: &Ref) {
            ::star_script_runtime::nodes::remove::<crate::classes::Host_>(this_);
        }

        /// `func parent`
        pub fn parent(this_: &Ref) -> ::std::option::Option<Ref> {
            ::star_script_runtime::nodes::parent::<crate::classes::Host_>(this_)
        }

        /// `func children`
        pub fn children(this_: &Ref) -> ::star_script_runtime::List<Ref> {
            ::star_script_runtime::nodes::children::<crate::classes::Host_>(this_)
        }

        /// `func find_child`
        pub fn find_child(this_: &Ref, name: ::std::string::String) -> ::std::option::Option<Ref> {
            ::star_script_runtime::nodes::find_child::<crate::classes::Host_>(this_, &name)
        }

        /// `func has_tag`
        pub fn has_tag(this_: &Ref, tag: ::std::string::String) -> bool {
            ::star_script_runtime::nodes::has_tag::<crate::classes::Host_>(this_, &tag)
        }

        /// `func find_by_name`
        pub fn find_by_name(this_: &Ref, name: ::std::string::String) -> ::std::option::Option<Ref> {
            ::star_script_runtime::nodes::find_by_name::<crate::classes::Host_>(this_, &name)
        }

        /// `func find_by_tag`
        pub fn find_by_tag(this_: &Ref, tag: ::std::string::String) -> ::star_script_runtime::List<Ref> {
            ::star_script_runtime::nodes::find_by_tag::<crate::classes::Host_>(this_, &tag)
        }

        /// `func global_x`
        pub fn global_x(this_: &Ref) -> f32 {
            ::star_script_runtime::nodes::global_position::<crate::classes::Host_>(this_)[0]
        }

        /// `func global_y`
        pub fn global_y(this_: &Ref) -> f32 {
            ::star_script_runtime::nodes::global_position::<crate::classes::Host_>(this_)[1]
        }

        /// `func global_rotation`
        pub fn global_rotation(this_: &Ref) -> f32 {
            ::star_script_runtime::nodes::global_rotation::<crate::classes::Host_>(this_)
        }

        /// `func set_global_position`
        pub fn set_global_position(this_: &Ref, x: f32, y: f32) {
            ::star_script_runtime::nodes::set_global_position::<crate::classes::Host_>(this_, x, y);
        }

        /// `func set_global_rotation`
        pub fn set_global_rotation(this_: &Ref, angle: f32) {
            ::star_script_runtime::nodes::set_global_rotation::<crate::classes::Host_>(this_, angle);
        }

        /// `func translate`
        pub fn translate(this_: &Ref, x: f32, y: f32) {
            ::star_script_runtime::nodes::translate::<crate::classes::Host_>(this_, x, y);
        }

        /// `func translate_global`
        pub fn translate_global(this_: &Ref, x: f32, y: f32) {
            ::star_script_runtime::nodes::translate_global::<crate::classes::Host_>(this_, x, y);
        }

        /// `func rotate`
        pub fn rotate(this_: &Ref, angle: f32) {
            ::star_script_runtime::nodes::rotate::<crate::classes::Host_>(this_, angle);
        }
    }

    /// The root of the game's tree; `None` without a game.
    pub fn game_root_() -> ::std::option::Option<Ref> {
        GAME_ROOT_.with(|game_root_| game_root_.borrow().as_ref().and_then(::std::rc::Weak::upgrade))
    }

    thread_local! {
        /// The root of the game's tree, once the game exists.
        static GAME_ROOT_: ::std::cell::RefCell<::std::option::Option<::std::rc::Weak<dyn Object>>> = const { ::std::cell::RefCell::new(::std::option::Option::None) };
    }

    /// Creates the root of the game's tree, which `in_tree_` looks for.
    pub fn new_root_() -> Ref {
        let root_ = new(::std::string::String::from(\"Game\"));
        GAME_ROOT_.with(|game_root_| *game_root_.borrow_mut() = ::std::option::Option::Some(::std::rc::Rc::downgrade(&root_)));
        root_
    }

    /// Whether `node` is in the game's tree, so its coroutines go on.
    /// Without a game (a console program) there is no tree to leave.
    pub fn in_tree_(node: &Ref) -> bool {
        match game_root_() {
            ::std::option::Option::Some(root_) => ::star_script_runtime::nodes::is_same_or_ancestor::<crate::classes::Host_>(&root_, node),
            ::std::option::Option::None => true,
        }
    }

    /// An object created as this class (not as a subclass).
    pub struct Instance {
        pub Node: Fields,
    }

    impl HasFields for Instance {
        fn fields(&self) -> &Fields { &self.Node }
    }

    impl crate::AnyObject for Instance {
        fn Node(self: ::std::rc::Rc<Self>) -> ::std::option::Option<Ref> { ::std::option::Option::Some(self) }
    }

    impl Object for Instance {
        fn start(self: ::std::rc::Rc<Self>) { Methods::start(&(self as Ref)) }
        fn update(self: ::std::rc::Rc<Self>, delta: f32) { Methods::update(&(self as Ref), delta) }
        fn add_child(self: ::std::rc::Rc<Self>, child: Ref) { Methods::add_child(&(self as Ref), child) }
        fn remove(self: ::std::rc::Rc<Self>) { Methods::remove(&(self as Ref)) }
        fn parent(self: ::std::rc::Rc<Self>) -> ::std::option::Option<Ref> { Methods::parent(&(self as Ref)) }
        fn children(self: ::std::rc::Rc<Self>) -> ::star_script_runtime::List<Ref> { Methods::children(&(self as Ref)) }
        fn find_child(self: ::std::rc::Rc<Self>, name: ::std::string::String) -> ::std::option::Option<Ref> { Methods::find_child(&(self as Ref), name) }
        fn has_tag(self: ::std::rc::Rc<Self>, tag: ::std::string::String) -> bool { Methods::has_tag(&(self as Ref), tag) }
        fn find_by_name(self: ::std::rc::Rc<Self>, name: ::std::string::String) -> ::std::option::Option<Ref> { Methods::find_by_name(&(self as Ref), name) }
        fn find_by_tag(self: ::std::rc::Rc<Self>, tag: ::std::string::String) -> ::star_script_runtime::List<Ref> { Methods::find_by_tag(&(self as Ref), tag) }
        fn global_x(self: ::std::rc::Rc<Self>) -> f32 { Methods::global_x(&(self as Ref)) }
        fn global_y(self: ::std::rc::Rc<Self>) -> f32 { Methods::global_y(&(self as Ref)) }
        fn global_rotation(self: ::std::rc::Rc<Self>) -> f32 { Methods::global_rotation(&(self as Ref)) }
        fn set_global_position(self: ::std::rc::Rc<Self>, x: f32, y: f32) { Methods::set_global_position(&(self as Ref), x, y) }
        fn set_global_rotation(self: ::std::rc::Rc<Self>, angle: f32) { Methods::set_global_rotation(&(self as Ref), angle) }
        fn translate(self: ::std::rc::Rc<Self>, x: f32, y: f32) { Methods::translate(&(self as Ref), x, y) }
        fn translate_global(self: ::std::rc::Rc<Self>, x: f32, y: f32) { Methods::translate_global(&(self as Ref), x, y) }
        fn rotate(self: ::std::rc::Rc<Self>, angle: f32) { Methods::rotate(&(self as Ref), angle) }
    }

    /// Prepares an object of this class: `make_` builds it from its fields.
    pub fn init_<O: Object + 'static>(name: ::std::string::String, make_: impl FnOnce(Fields) -> ::std::rc::Rc<O>) -> ::std::rc::Rc<O> {
        make_(Fields::new(name))
    }

    /// Creates an object of this class.
    pub fn new(name: ::std::string::String) -> Ref {
        init_(name, |fields0_| ::std::rc::Rc::new(Instance { Node: fields0_ }))
    }
}";

/// How the generated classes are the nodes of `star_script_runtime::nodes`.
const HOST: &str = "/// The generated classes, as the nodes `star_script_runtime::nodes` works on.
pub struct Host_;

impl ::star_script_runtime::nodes::Host for Host_ {
    type Node = crate::classes::Node::Ref;
    type WeakNode = ::std::rc::Weak<dyn crate::classes::Node::Object>;
    type Node3D = crate::classes::Node3D::Ref;
    type Camera2D = crate::classes::Camera2D::Ref;
    type Camera3D = crate::classes::Camera3D::Ref;
    type Body = crate::classes::Body::Ref;
    type Color = crate::classes::Color::Ref;
    type Game = crate::classes::Game::Ref;
    type Tags = ::star_script_runtime::List<::std::string::String>;

    fn node(node: &Self::Node) -> &crate::classes::Node::Fields {
        crate::classes::Node::HasFields::fields(&**node)
    }

    fn downgrade(node: &Self::Node) -> Self::WeakNode {
        ::std::rc::Rc::downgrade(node)
    }

    fn upgrade(weak: &Self::WeakNode) -> ::std::option::Option<Self::Node> {
        weak.upgrade()
    }

    fn same(node: &Self::Node, other: &Self::Node) -> bool {
        ::std::rc::Rc::ptr_eq(node, other)
    }

    fn as_node_3d(node: &Self::Node) -> ::std::option::Option<Self::Node3D> {
        crate::AnyObject::Node3D(::std::rc::Rc::clone(node))
    }

    fn node_3d(node: &Self::Node3D) -> &::star_script_runtime::nodes::Node3dFields {
        crate::classes::Node3D::HasFields::fields(&**node)
    }

    fn as_camera_2d(node: &Self::Node) -> ::std::option::Option<Self::Camera2D> {
        crate::AnyObject::Camera2D(::std::rc::Rc::clone(node))
    }

    fn camera_2d(camera: &Self::Camera2D) -> &::star_script_runtime::node_classes::Camera2dFields {
        crate::classes::Camera2D::HasFields::fields(&**camera)
    }

    fn camera_2d_node(camera: &Self::Camera2D) -> Self::Node {
        let camera_: crate::classes::Camera2D::Ref = ::std::rc::Rc::clone(camera);
        camera_
    }

    fn as_camera_3d(node: &Self::Node) -> ::std::option::Option<Self::Camera3D> {
        crate::AnyObject::Camera3D(::std::rc::Rc::clone(node))
    }

    fn camera_3d(camera: &Self::Camera3D) -> &::star_script_runtime::node_classes::Camera3dFields {
        crate::classes::Camera3D::HasFields::fields(&**camera)
    }

    fn camera_3d_node(camera: &Self::Camera3D) -> Self::Node {
        let camera_: crate::classes::Camera3D::Ref = ::std::rc::Rc::clone(camera);
        camera_
    }

    fn with_sprite<R>(node: &Self::Node, read: impl FnOnce(&::star_script_runtime::node_classes::SpriteFields<Self>) -> R) -> ::std::option::Option<R> {
        crate::AnyObject::Sprite(::std::rc::Rc::clone(node)).map(|sprite_| read(crate::classes::Sprite::HasFields::fields(&*sprite_)))
    }

    fn with_mesh<R>(node: &Self::Node, read: impl FnOnce(&::star_script_runtime::node_classes::MeshFields<Self>) -> R) -> ::std::option::Option<R> {
        crate::AnyObject::Mesh(::std::rc::Rc::clone(node)).map(|mesh_| read(crate::classes::Mesh::HasFields::fields(&*mesh_)))
    }

    fn with_light<R>(node: &Self::Node, read: impl FnOnce(&::star_script_runtime::node_classes::LightFields<Self>) -> R) -> ::std::option::Option<R> {
        crate::AnyObject::Light(::std::rc::Rc::clone(node)).map(|light_| read(crate::classes::Light::HasFields::fields(&*light_)))
    }

    fn with_light_2d<R>(node: &Self::Node, read: impl FnOnce(&::star_script_runtime::node_classes::LightFields<Self>) -> R) -> ::std::option::Option<R> {
        crate::AnyObject::Light2D(::std::rc::Rc::clone(node)).map(|light_| read(crate::classes::Light2D::HasFields::fields(&*light_)))
    }

    fn with_animator<R>(node: &Self::Node, read: impl FnOnce(&::star_script_runtime::node_classes::AnimatorFields) -> R) -> ::std::option::Option<R> {
        crate::AnyObject::Animator(::std::rc::Rc::clone(node)).map(|animator_| read(crate::classes::Animator::HasFields::fields(&*animator_)))
    }

    fn new_tags(tags: ::std::vec::Vec<::std::string::String>) -> Self::Tags {
        ::star_script_runtime::List::from_vec(tags)
    }

    fn has_tag(tags: &Self::Tags, tag: &str) -> bool {
        tags.any(|own_| own_ == tag)
    }

    fn body(body: &Self::Body) -> &::star_script_runtime::bodies::BodyFields<Self> {
        crate::classes::Body::HasFields::fields(&**body)
    }

    fn color(r: f32, g: f32, b: f32, a: f32) -> Self::Color {
        crate::classes::Color::new(r, g, b, a)
    }

    fn channels(color: &Self::Color) -> [f32; 4] {
        crate::classes::Color::HasFields::fields(&**color).channels()
    }

    fn create(name: ::std::string::String, class: ::star_script_runtime::scene_tree::NewNode<'_>) -> ::std::result::Result<Self::Node, ::std::string::String> {
        use ::star_script_runtime::scene_tree::NewNode;
        let node_: crate::classes::Node::Ref = match class {
            NewNode::Node => crate::classes::Node::new(name),
            NewNode::Sprite { texture } => crate::classes::Sprite::new(name, ::std::string::String::from(texture)),
            NewNode::Camera2D => crate::classes::Camera2D::new(name),
            NewNode::Light2D { kind } => crate::classes::Light2D::new(name, ::std::string::String::from(kind)),
            NewNode::Node3D => crate::classes::Node3D::new(name),
            NewNode::Mesh { shape } => crate::classes::Mesh::new(name, ::std::string::String::from(shape)),
            NewNode::Light { kind } => crate::classes::Light::new(name, ::std::string::String::from(kind)),
            NewNode::Camera3D => crate::classes::Camera3D::new(name),
            NewNode::Animator => crate::classes::Animator::new(name),
            NewNode::Script(class_) => crate::new_scene_object_(class_)?,
        };
        ::std::result::Result::Ok(node_)
    }

    fn set_exported(class: &str, node: &Self::Node, values: &[(::std::string::String, ::star_script_runtime::scene_fields::SceneValue)]) -> ::std::result::Result<(), ::std::string::String> {
        crate::set_scene_fields_(class, node, values)
    }

    fn set_exported_field(node: &Self::Node, field: &str, value: &::star_script_runtime::scene_fields::SceneValue) -> ::std::result::Result<(), ::std::string::String> {
        crate::set_exported_field_(node, field, value)
    }

    fn new_body(fields: ::star_script_runtime::bodies::BodyFields<Self>) -> Self::Body {
        crate::classes::Body::of_(fields)
    }

    fn start(node: &Self::Node) {
        crate::classes::Node::Object::start(::std::rc::Rc::clone(node));
    }

    fn update(node: &Self::Node, delta: f32) {
        crate::classes::Node::Object::update(::std::rc::Rc::clone(node), delta);
    }

    fn animation_finished(node: &Self::Node, name: &str) {
        if let ::std::option::Option::Some(sprite_) = crate::AnyObject::Sprite(::std::rc::Rc::clone(node)) {
            crate::classes::Sprite::Object::animation_finished(sprite_, ::std::borrow::ToOwned::to_owned(name));
        } else if let ::std::option::Option::Some(animator_) = crate::AnyObject::Animator(::std::rc::Rc::clone(node)) {
            crate::classes::Animator::Object::animation_finished(animator_, ::std::borrow::ToOwned::to_owned(name));
        }
    }

    fn game(game: &Self::Game) -> &::star_script_runtime::game::GameFields<Self> {
        crate::classes::Game::HasFields::fields(&**game)
    }

    fn start_game(game: &Self::Game) {
        crate::classes::Game::Object::start(::std::rc::Rc::clone(game));
    }

    fn update_game(game: &Self::Game, delta: f32) {
        crate::classes::Game::Object::update(::std::rc::Rc::clone(game), delta);
    }
}";

const SCENE_MODULE: &str = "/// `class Scene`, provided by the engine.
pub mod Scene {
    /// `static func load`: a new tree with the nodes of the scene at `path`,
    /// out of the game until it is added; `None` if it cannot be loaded.
    pub fn load(path: ::std::string::String) -> ::std::option::Option<crate::classes::Node::Ref> {
        ::star_script_runtime::scene_tree::load::<crate::classes::Host_>(&path)
    }

    /// `static func find_by_name`: the first node of the game's tree named
    /// `name`; `None` without a game.
    pub fn find_by_name(name: ::std::string::String) -> ::std::option::Option<crate::classes::Node::Ref> {
        crate::classes::Node::game_root_().and_then(|root_| crate::classes::Node::Methods::find_by_name(&root_, name))
    }

    /// `static func find_by_tag`: every node of the game's tree with the
    /// tag `tag`; none without a game.
    pub fn find_by_tag(tag: ::std::string::String) -> ::star_script_runtime::List<crate::classes::Node::Ref> {
        match crate::classes::Node::game_root_() {
            ::std::option::Option::Some(root_) => crate::classes::Node::Methods::find_by_tag(&root_, tag),
            ::std::option::Option::None => ::star_script_runtime::List::new(),
        }
    }
}";

const SOUND_MODULE: &str = "/// `class Sound`, provided by the engine.
pub mod Sound {
    /// A reference to an object of this class.
    pub type Ref = ::std::rc::Rc<dyn Object>;

    /// The fields of a sound: how it plays, which sound of the runtime it
    /// is, and the playings it started that may still sound.
    pub type Fields = ::star_script_runtime::audio::SoundFields;

    /// Gives access to the fields of this class.
    pub trait HasFields {
        fn fields(&self) -> &Fields;
    }

    /// What a sound can do.
    pub trait Object: crate::AnyObject + HasFields {
        fn play(self: ::std::rc::Rc<Self>);
        fn stop(self: ::std::rc::Rc<Self>);
        fn is_playing(self: ::std::rc::Rc<Self>) -> bool;
    }

    /// The methods of a sound: what `star_script_runtime::audio` does with
    /// its fields.
    pub struct Methods;

    impl Methods {
        /// `func play`
        pub fn play(this_: &Ref) {
            HasFields::fields(&**this_).play();
        }

        /// `func stop`
        pub fn stop(this_: &Ref) {
            HasFields::fields(&**this_).stop();
        }

        /// `func is_playing`
        pub fn is_playing(this_: &Ref) -> bool {
            HasFields::fields(&**this_).is_playing()
        }
    }

    /// An object created as this class.
    pub struct Instance {
        pub Sound: Fields,
    }

    impl HasFields for Instance {
        fn fields(&self) -> &Fields { &self.Sound }
    }

    impl crate::AnyObject for Instance {
        fn Sound(self: ::std::rc::Rc<Self>) -> ::std::option::Option<Ref> { ::std::option::Option::Some(self) }
    }

    impl Object for Instance {
        fn play(self: ::std::rc::Rc<Self>) { Methods::play(&(self as Ref)) }
        fn stop(self: ::std::rc::Rc<Self>) { Methods::stop(&(self as Ref)) }
        fn is_playing(self: ::std::rc::Rc<Self>) -> bool { Methods::is_playing(&(self as Ref)) }
    }

    /// `static func load`
    pub fn load(path: ::std::string::String) -> ::std::option::Option<Ref> {
        let fields_ = Fields::load(&path)?;
        ::std::option::Option::Some(::std::rc::Rc::new(Instance { Sound: fields_ }))
    }
}";

/// `fn main` that runs the game `class_path` in a window of the engine, with
/// the input `actions` of the project.
/// `new_game` and `title` are Rust expressions, run inside `run_`: the
/// game and the title of its window.
pub(super) fn game_entry_point(
    out: &mut Writer,
    new_game: &str,
    title: &str,
    actions: &[InputAction],
) {
    action_declarations(out, actions);
    let main = format!(
        "
/// Starts the program: runs the game in a window of the engine.
pub fn main() -> ::std::process::ExitCode {{
    match run_() {{
        ::std::result::Result::Ok(()) => ::std::process::ExitCode::SUCCESS,
        ::std::result::Result::Err(error_) => {{
            ::std::eprintln!(\"{{error_}}\");
            ::std::process::ExitCode::FAILURE
        }}
    }}
}}

fn run_() -> ::star_engine::star_core::EngineResult<()> {{
    let actions_ = action_map_()?;
    {RUNTIME}::input::declare_actions(&ACTION_NAMES_);
    let app_ = ::star_engine::app::App::new()
        .with_title({title})
        .with_actions(actions_);
    {SCRIPT_ENGINE}::game::run::<crate::classes::Host_>(app_, &ACTION_NAMES_, || {{
        let game_: crate::classes::Game::Ref = {new_game};
        ::std::result::Result::Ok(game_)
    }})
}}
"
    );
    for line in main.lines() {
        out.line(line);
    }
}

/// Writes `main_scene_game_`: the game of a program without a `Game`
/// class, which only shows a scene.
pub(super) fn main_scene_game(out: &mut Writer) {
    for line in std::iter::once("").chain(MAIN_SCENE_GAME.lines()) {
        out.line(line);
    }
}

const MAIN_SCENE_GAME: &str = "/// A game with nothing but the scene at `path`: the main scene of a
/// project without a `Game` class.
fn main_scene_game_(path_: &str) -> ::star_engine::star_core::EngineResult<crate::classes::Game::Ref> {
    let game_ = crate::classes::Game::new();
    let scene_ = crate::classes::Scene::load(::std::string::String::from(path_)).ok_or_else(|| {
        ::star_engine::star_core::EngineError::Scene(::std::format!(\"the main scene, \\\"{path_}\\\", could not be loaded\"))
    })?;
    crate::classes::Game::Object::add_child(::std::rc::Rc::clone(&game_), scene_);
    ::std::result::Result::Ok(game_)
}";

/// The names of the actions and the function that builds their map. The
/// engine checks the bindings' names when the game starts.
fn action_declarations(out: &mut Writer, actions: &[InputAction]) {
    let names: Vec<String> = actions
        .iter()
        .map(|action| format!("{:?}", action.name))
        .collect();
    out.line("/// The input actions declared in the project.");
    out.line(&format!(
        "const ACTION_NAMES_: [&str; {}] = [{}];",
        actions.len(),
        names.join(", ")
    ));
    out.line("");
    out.open(
        "fn action_map_() -> ::star_engine::star_core::EngineResult<::star_engine::star_input::ActionMap> {",
    );
    out.line("let mut actions_ = ::star_engine::star_input::ActionMap::new();");
    for action in actions {
        let bindings: Vec<String> = action
            .bindings
            .iter()
            .map(|binding| format!("{binding:?}.parse()?"))
            .collect();
        out.line(&format!(
            "actions_.add({:?}, &[{}])?;",
            action.name,
            bindings.join(", ")
        ));
    }
    out.line("::std::result::Result::Ok(actions_)");
    out.close("}");
}
