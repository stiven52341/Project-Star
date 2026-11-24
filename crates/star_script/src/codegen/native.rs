//! Modules of the engine classes that scripts can extend, and the code that
//! runs a game in the engine.
//!
//! Their methods have no StarScript source, so their modules are written
//! here, with the same shape as the modules of script classes.

use super::conversions::RUNTIME;
use super::native_3d;
use super::writer::Writer;
use crate::InputAction;

/// Hidden field of `Game` that `quit()` sets. It ends in a single `_`, so no
/// StarScript field can have the same Rust name.
const QUIT_FIELD: &str = "quit_requested_";

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
    pub struct Fields {
        pub background: ::star_script_runtime::Field<crate::classes::Color::Ref>,
        pub camera: ::star_script_runtime::Field<::std::option::Option<crate::classes::Camera2D::Ref>>,
        pub camera_3d: ::star_script_runtime::Field<::std::option::Option<crate::classes::Camera3D::Ref>>,
        pub ambient: ::star_script_runtime::Field<crate::classes::Color::Ref>,
        pub ambient_2d: ::star_script_runtime::Field<crate::classes::Color::Ref>,
        pub quit_requested_: ::star_script_runtime::Field<bool>,
        pub root_: crate::classes::Node::Ref,
    }

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
            HasFields::fields(&**this_).quit_requested_.set(true);
        }

        /// `func add_child`: puts `node` in the game's tree, at the top.
        pub fn add_child(this_: &Ref, node: crate::classes::Node::Ref) {
            crate::classes::Node::Methods::add_child(&HasFields::fields(&**this_).root_, node);
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

    /// Makes `node` the game's `camera` or `camera_3d` if it is a camera
    /// marked `main`: called as it enters the game's tree, before its
    /// `start`.
    pub fn see_from_if_main_(this_: &Ref, node: &crate::classes::Node::Ref) {
        let fields_ = HasFields::fields(&**this_);
        if let ::std::option::Option::Some(camera_) = crate::AnyObject::Camera2D(::std::rc::Rc::clone(node)) {
            if crate::classes::Camera2D::HasFields::fields(&*camera_).main.get() {
                fields_.camera.set(::std::option::Option::Some(camera_));
            }
        } else if let ::std::option::Option::Some(camera_) = crate::AnyObject::Camera3D(::std::rc::Rc::clone(node)) {
            if crate::classes::Camera3D::HasFields::fields(&*camera_).main.get() {
                fields_.camera_3d.set(::std::option::Option::Some(camera_));
            }
        }
    }

    /// Prepares an object of this class: `make_` builds it from its fields.
    pub fn init_<O: Object + 'static>(make_: impl FnOnce(Fields) -> ::std::rc::Rc<O>) -> ::std::rc::Rc<O> {
        make_(Fields {
            background: ::star_script_runtime::Field::new(crate::classes::Color::new(0.0, 0.0, 0.0, 1.0)),
            camera: ::star_script_runtime::Field::new(::std::option::Option::None),
            camera_3d: ::star_script_runtime::Field::new(::std::option::Option::None),
            ambient: ::star_script_runtime::Field::new(crate::classes::Color::new(0.0, 0.0, 0.0, 1.0)),
            ambient_2d: ::star_script_runtime::Field::new(crate::classes::Color::new(1.0, 1.0, 1.0, 1.0)),
            quit_requested_: ::star_script_runtime::Field::new(false),
            root_: crate::classes::Node::new_root_(),
        })
    }

    /// Creates an object of this class.
    pub fn new() -> Ref {
        init_(|fields0_| ::std::rc::Rc::new(Instance { Game: fields0_ }))
    }
}";

/// The modules of the engine classes `Color`, `Body`, `Node`, `Sprite`,
/// `Camera2D`, `Scene` and `Sound`, and of the 3D nodes.
pub(super) fn node_modules(out: &mut Writer) {
    let modules = [
        COLOR_MODULE,
        BODY_MODULE,
        NODE_MODULE,
        SPRITE_MODULE,
        CAMERA_MODULE,
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
    native_3d::node_3d_modules(out);
}

const COLOR_MODULE: &str = "/// `class Color`, provided by the engine.
pub mod Color {
    /// A reference to an object of this class.
    pub type Ref = ::std::rc::Rc<dyn Object>;

    /// The fields of a color: its channels, from `0.0` to `1.0`.
    pub struct Fields {
        pub r: ::star_script_runtime::Field<f32>,
        pub g: ::star_script_runtime::Field<f32>,
        pub b: ::star_script_runtime::Field<f32>,
        pub a: ::star_script_runtime::Field<f32>,
    }

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
        ::std::rc::Rc::new(Instance {
            Color: Fields {
                r: ::star_script_runtime::Field::new(r),
                g: ::star_script_runtime::Field::new(g),
                b: ::star_script_runtime::Field::new(b),
                a: ::star_script_runtime::Field::new(a),
            },
        })
    }
}";

const BODY_MODULE: &str = "/// `class Body`, provided by the engine.
pub mod Body {
    /// A reference to an object of this class.
    pub type Ref = ::std::rc::Rc<dyn Object>;

    /// The nodes a body touches, without keeping them alive.
    pub type Contacts_ = ::std::cell::RefCell<::std::vec::Vec<::std::rc::Weak<dyn crate::classes::Node::Object>>>;

    /// The fields of a body: the public ones, the impulses given during the
    /// frame, and what it touched in the last one.
    pub struct Fields {
        pub kind: ::star_script_runtime::Field<::std::string::String>,
        pub shape: ::star_script_runtime::Field<::std::string::String>,
        pub radius: ::star_script_runtime::Field<f32>,
        pub width: ::star_script_runtime::Field<f32>,
        pub height: ::star_script_runtime::Field<f32>,
        pub depth: ::star_script_runtime::Field<f32>,
        pub gravity_scale: ::star_script_runtime::Field<f32>,
        pub friction: ::star_script_runtime::Field<f32>,
        pub bounciness: ::star_script_runtime::Field<f32>,
        pub density: ::star_script_runtime::Field<f32>,
        pub velocity_x: ::star_script_runtime::Field<f32>,
        pub velocity_y: ::star_script_runtime::Field<f32>,
        pub velocity_z: ::star_script_runtime::Field<f32>,
        pub fixed_rotation: ::star_script_runtime::Field<bool>,
        pub sensor: ::star_script_runtime::Field<bool>,
        pub impulse_: ::std::cell::Cell<(f32, f32, f32)>,
        pub touching_: Contacts_,
        pub just_touched_: Contacts_,
        pub just_separated_: Contacts_,
    }

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

    /// The methods of a body.
    pub struct Methods;

    impl Methods {
        /// `func apply_impulse`: pushes the body once the frame ends.
        pub fn apply_impulse(this_: &Ref, x: f32, y: f32) {
            Self::apply_impulse_3d(this_, x, y, 0.0);
        }

        /// `func apply_impulse_3d`: pushes the body once the frame ends.
        pub fn apply_impulse_3d(this_: &Ref, x: f32, y: f32, z: f32) {
            let impulse_ = &HasFields::fields(&**this_).impulse_;
            let (old_x_, old_y_, old_z_) = impulse_.get();
            impulse_.set((old_x_ + x, old_y_ + y, old_z_ + z));
        }

        /// `func touching`
        pub fn touching(this_: &Ref) -> ::star_script_runtime::List<crate::classes::Node::Ref> {
            nodes_(&HasFields::fields(&**this_).touching_)
        }

        /// `func just_touched`
        pub fn just_touched(this_: &Ref) -> ::star_script_runtime::List<crate::classes::Node::Ref> {
            nodes_(&HasFields::fields(&**this_).just_touched_)
        }

        /// `func just_separated`
        pub fn just_separated(this_: &Ref) -> ::star_script_runtime::List<crate::classes::Node::Ref> {
            nodes_(&HasFields::fields(&**this_).just_separated_)
        }
    }

    /// A new list with the nodes of `contacts_` that still exist.
    fn nodes_(contacts_: &Contacts_) -> ::star_script_runtime::List<crate::classes::Node::Ref> {
        ::star_script_runtime::List::from_vec(contacts_.borrow().iter().filter_map(::std::rc::Weak::upgrade).collect())
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

    /// A body of `kind` and `shape` at rest, with the engine's surface.
    pub fn new_(kind: ::std::string::String, shape: &str, radius: f32, width: f32, height: f32, depth: f32) -> Ref {
        let contacts_ = || ::std::cell::RefCell::new(::std::vec::Vec::new());
        ::std::rc::Rc::new(Instance {
            Body: Fields {
                kind: ::star_script_runtime::Field::new(kind),
                shape: ::star_script_runtime::Field::new(::std::string::String::from(shape)),
                radius: ::star_script_runtime::Field::new(radius),
                width: ::star_script_runtime::Field::new(width),
                height: ::star_script_runtime::Field::new(height),
                depth: ::star_script_runtime::Field::new(depth),
                gravity_scale: ::star_script_runtime::Field::new(1.0),
                friction: ::star_script_runtime::Field::new(0.5),
                bounciness: ::star_script_runtime::Field::new(0.0),
                density: ::star_script_runtime::Field::new(1.0),
                velocity_x: ::star_script_runtime::Field::new(0.0),
                velocity_y: ::star_script_runtime::Field::new(0.0),
                velocity_z: ::star_script_runtime::Field::new(0.0),
                fixed_rotation: ::star_script_runtime::Field::new(false),
                sensor: ::star_script_runtime::Field::new(false),
                impulse_: ::std::cell::Cell::new((0.0, 0.0, 0.0)),
                touching_: contacts_(),
                just_touched_: contacts_(),
                just_separated_: contacts_(),
            },
        })
    }

    /// `static func circle`
    pub fn circle(kind: ::std::string::String, radius: f32) -> Ref {
        new_(kind, \"circle\", radius, 1.0, 1.0, 1.0)
    }

    /// `static func rectangle`
    pub fn rectangle(kind: ::std::string::String, width: f32, height: f32) -> Ref {
        new_(kind, \"rectangle\", 0.5, width, height, 1.0)
    }

    /// `static func capsule`
    pub fn capsule(kind: ::std::string::String, height: f32, radius: f32) -> Ref {
        new_(kind, \"capsule\", radius, 1.0, height, 1.0)
    }

    /// `static func sphere`
    pub fn sphere(kind: ::std::string::String, radius: f32) -> Ref {
        new_(kind, \"sphere\", radius, 1.0, 1.0, 1.0)
    }

    /// `static func box`
    pub fn box_(kind: ::std::string::String, width: f32, height: f32, depth: f32) -> Ref {
        new_(kind, \"box\", 0.5, width, height, depth)
    }
}";

const NODE_MODULE: &str = "/// `class Node`, provided by the engine.
pub mod Node {
    /// A reference to an object of this class or of a subclass.
    pub type Ref = ::std::rc::Rc<dyn Object>;

    /// Gives every node its own number, which the engine knows it by.
    static NEXT_ID_: ::std::sync::atomic::AtomicU64 = ::std::sync::atomic::AtomicU64::new(1);

    /// The fields of a node: the public ones, and its place in the tree.
    pub struct Fields {
        pub name: ::star_script_runtime::Field<::std::string::String>,
        pub x: ::star_script_runtime::Field<f32>,
        pub y: ::star_script_runtime::Field<f32>,
        pub rotation: ::star_script_runtime::Field<f32>,
        pub scale_x: ::star_script_runtime::Field<f32>,
        pub scale_y: ::star_script_runtime::Field<f32>,
        pub body: ::star_script_runtime::Field<::std::option::Option<crate::classes::Body::Ref>>,
        pub tags: ::star_script_runtime::Field<::star_script_runtime::List<::std::string::String>>,
        pub id_: u64,
        pub parent_: ::std::cell::RefCell<::std::option::Option<::std::rc::Weak<dyn Object>>>,
        pub children_: ::std::cell::RefCell<::std::vec::Vec<Ref>>,
        pub started_: ::std::cell::Cell<bool>,
    }

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

    /// What nodes do unless a subclass overrides it.
    pub struct Methods;

    impl Methods {
        /// `func start`
        pub fn start(this_: &Ref) {}

        /// `func update`
        pub fn update(this_: &Ref, delta: f32) {}

        /// `func add_child`: moves `child`, with its own children, under this node.
        pub fn add_child(this_: &Ref, child: Ref) {
            if is_same_or_ancestor_(&child, this_) {
                ::std::eprintln!(\"StarScript warning: a node cannot be added under itself or one of its children\");
                return;
            }
            Self::remove(&child);
            *HasFields::fields(&*child).parent_.borrow_mut() = ::std::option::Option::Some(::std::rc::Rc::downgrade(this_));
            HasFields::fields(&**this_).children_.borrow_mut().push(child);
        }

        /// `func remove`: takes the node, with its children, out of the tree.
        pub fn remove(this_: &Ref) {
            let parent_ = HasFields::fields(&**this_).parent_.borrow_mut().take();
            if let ::std::option::Option::Some(parent_) = parent_.and_then(|parent_| parent_.upgrade()) {
                HasFields::fields(&*parent_).children_.borrow_mut().retain(|child_| !::std::rc::Rc::ptr_eq(child_, this_));
            }
        }

        /// `func parent`
        pub fn parent(this_: &Ref) -> ::std::option::Option<Ref> {
            HasFields::fields(&**this_).parent_.borrow().as_ref().and_then(::std::rc::Weak::upgrade)
        }

        /// `func children`: a new list, so changing it does not change the tree.
        pub fn children(this_: &Ref) -> ::star_script_runtime::List<Ref> {
            ::star_script_runtime::List::from_vec(HasFields::fields(&**this_).children_.borrow().clone())
        }

        /// `func find_child`
        pub fn find_child(this_: &Ref, name: ::std::string::String) -> ::std::option::Option<Ref> {
            HasFields::fields(&**this_).children_.borrow().iter()
                .find(|child_| HasFields::fields(&***child_).name.read(|child_name_| *child_name_ == name))
                .cloned()
        }

        /// `func has_tag`
        pub fn has_tag(this_: &Ref, tag: ::std::string::String) -> bool {
            HasFields::fields(&**this_).tags.read(|tags_| tags_.contains(&tag))
        }

        /// `func find_by_name`: the first descendant named `name`, parents
        /// before their children and children in order.
        pub fn find_by_name(this_: &Ref, name: ::std::string::String) -> ::std::option::Option<Ref> {
            descendants_(this_).into_iter()
                .find(|node_| HasFields::fields(&**node_).name.read(|node_name_| *node_name_ == name))
        }

        /// `func find_by_tag`: every descendant with the tag `tag`, parents
        /// before their children and children in order.
        pub fn find_by_tag(this_: &Ref, tag: ::std::string::String) -> ::star_script_runtime::List<Ref> {
            let tagged_ = descendants_(this_).into_iter()
                .filter(|node_| HasFields::fields(&**node_).tags.read(|tags_| tags_.contains(&tag)))
                .collect();
            ::star_script_runtime::List::from_vec(tagged_)
        }

        /// `func global_x`: where the node is across, in the world.
        pub fn global_x(this_: &Ref) -> f32 {
            global_position_(this_)[0]
        }

        /// `func global_y`: where the node is up, in the world.
        pub fn global_y(this_: &Ref) -> f32 {
            global_position_(this_)[1]
        }

        /// `func global_rotation`: how much the node is turned in the
        /// world: its `rotation` and those of its ancestors placed as it
        /// is (in 2D, or in 3D for a `Node3D`, where it is its roll).
        pub fn global_rotation(this_: &Ref) -> f32 {
            let in_3d_ = is_3d_(this_);
            ancestors_(this_).iter()
                .filter(|ancestor_| is_3d_(ancestor_) == in_3d_)
                .chain(::std::iter::once(this_))
                .map(|node_| HasFields::fields(&**node_).rotation.get())
                .sum()
        }

        /// `func set_global_position`: places the node at a point of the
        /// world; a `Node3D` keeps its depth in the world.
        pub fn set_global_position(this_: &Ref, x: f32, y: f32) {
            if is_3d_(this_) {
                let [_, _, depth_] = global_position_(this_);
                set_global_position_3d_(this_, [x, y, depth_]);
            } else if let ::std::option::Option::Some([local_x_, local_y_]) = parent_place_2d_(this_).local_point([x, y]) {
                let fields_ = HasFields::fields(&**this_);
                fields_.x.set(local_x_);
                fields_.y.set(local_y_);
            }
        }

        /// `func set_global_rotation`: turns the node to `angle` in the world.
        pub fn set_global_rotation(this_: &Ref, angle: f32) {
            let fields_ = HasFields::fields(&**this_);
            let inherited_ = Self::global_rotation(this_) - fields_.rotation.get();
            fields_.rotation.set(angle - inherited_);
        }

        /// `func translate`: moves the node in its parent's space.
        pub fn translate(this_: &Ref, x: f32, y: f32) {
            let fields_ = HasFields::fields(&**this_);
            fields_.x.set(fields_.x.get() + x);
            fields_.y.set(fields_.y.get() + y);
        }

        /// `func translate_global`: moves the node in the world.
        pub fn translate_global(this_: &Ref, x: f32, y: f32) {
            let [global_x_, global_y_, _] = global_position_(this_);
            Self::set_global_position(this_, global_x_ + x, global_y_ + y);
        }

        /// `func rotate`: turns the node a little more.
        pub fn rotate(this_: &Ref, angle: f32) {
            let fields_ = HasFields::fields(&**this_);
            fields_.rotation.set(fields_.rotation.get() + angle);
        }
    }

    /// The ancestors of `node`, from the root down to its parent.
    fn ancestors_(node: &Ref) -> ::std::vec::Vec<Ref> {
        let mut found_ = ::std::vec::Vec::new();
        let mut current_ = Methods::parent(node);
        while let ::std::option::Option::Some(ancestor_) = current_ {
            current_ = Methods::parent(&ancestor_);
            found_.push(ancestor_);
        }
        found_.reverse();
        found_
    }

    /// Whether `node` is placed in 3D: a `Node3D`.
    fn is_3d_(node: &Ref) -> bool {
        crate::AnyObject::Node3D(::std::rc::Rc::clone(node)).is_some()
    }

    /// Where the 2D space a 2D node is placed in is in the world: its
    /// parent's, through its ancestors placed in 2D, as the engine does.
    fn parent_place_2d_(node: &Ref) -> ::star_script_runtime::placement::Place2d {
        ancestors_(node).iter()
            .filter(|ancestor_| !is_3d_(ancestor_))
            .fold(::star_script_runtime::placement::Place2d::IDENTITY, |place_, ancestor_| place_.then(place_2d_(ancestor_)))
    }

    fn place_2d_(node: &Ref) -> ::star_script_runtime::placement::Place2d {
        let fields_ = HasFields::fields(&**node);
        ::star_script_runtime::placement::Place2d::of_node(fields_.x.get(), fields_.y.get(), fields_.rotation.get(), fields_.scale_x.get(), fields_.scale_y.get())
    }

    /// Where the 3D space a `Node3D` is placed in is in the world: its
    /// parent's, through its ancestors placed in 3D.
    fn parent_place_3d_(node: &Ref) -> ::star_script_runtime::placement::Place3d {
        ancestors_(node).iter()
            .filter_map(place_3d_)
            .fold(::star_script_runtime::placement::Place3d::IDENTITY, |place_, inner_| place_.then(inner_))
    }

    /// The place of `node` in its parent's 3D space, if it is a `Node3D`.
    fn place_3d_(node: &Ref) -> ::std::option::Option<::star_script_runtime::placement::Place3d> {
        let node_3d_ = crate::AnyObject::Node3D(::std::rc::Rc::clone(node))?;
        let fields_ = HasFields::fields(&**node);
        let depth_ = crate::classes::Node3D::HasFields::fields(&*node_3d_);
        ::std::option::Option::Some(::star_script_runtime::placement::Place3d::of_node(
            [fields_.x.get(), fields_.y.get(), depth_.z.get()],
            [depth_.pitch.get(), depth_.yaw.get(), fields_.rotation.get()],
            [fields_.scale_x.get(), fields_.scale_y.get(), depth_.scale_z.get()],
        ))
    }

    /// Where `node` is in the world: in 3D for a `Node3D`, else in 2D with
    /// a depth of 0.
    pub fn global_position_(node: &Ref) -> [f32; 3] {
        let fields_ = HasFields::fields(&**node);
        match crate::AnyObject::Node3D(::std::rc::Rc::clone(node)) {
            ::std::option::Option::Some(node_3d_) => {
                let depth_ = crate::classes::Node3D::HasFields::fields(&*node_3d_).z.get();
                parent_place_3d_(node).point([fields_.x.get(), fields_.y.get(), depth_])
            }
            ::std::option::Option::None => {
                let [x_, y_] = parent_place_2d_(node).point([fields_.x.get(), fields_.y.get()]);
                [x_, y_, 0.0]
            }
        }
    }

    /// Places the `Node3D` `node` at `point` of the world; it stays where
    /// it is if its parent is scaled to nothing.
    pub fn set_global_position_3d_(node: &Ref, point: [f32; 3]) {
        let ::std::option::Option::Some(node_3d_) = crate::AnyObject::Node3D(::std::rc::Rc::clone(node)) else {
            return;
        };
        if let ::std::option::Option::Some([x_, y_, z_]) = parent_place_3d_(node).local_point(point) {
            let fields_ = HasFields::fields(&**node);
            fields_.x.set(x_);
            fields_.y.set(y_);
            crate::classes::Node3D::HasFields::fields(&*node_3d_).z.set(z_);
        }
    }

    /// The descendants of `node`, not itself: each parent before its
    /// children, and children in their order.
    fn descendants_(node: &Ref) -> ::std::vec::Vec<Ref> {
        let mut found_ = ::std::vec::Vec::new();
        let mut pending_: ::std::vec::Vec<Ref> = HasFields::fields(&**node).children_.borrow().iter().rev().cloned().collect();
        while let ::std::option::Option::Some(next_) = pending_.pop() {
            pending_.extend(HasFields::fields(&*next_).children_.borrow().iter().rev().cloned());
            found_.push(next_);
        }
        found_
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
        let root_ = GAME_ROOT_.with(|game_root_| game_root_.borrow().as_ref().and_then(::std::rc::Weak::upgrade));
        match root_ {
            ::std::option::Option::Some(root_) => is_same_or_ancestor_(&root_, node),
            ::std::option::Option::None => true,
        }
    }

    /// Whether `node` is `other` or one of its ancestors.
    fn is_same_or_ancestor_(node: &Ref, other: &Ref) -> bool {
        let mut current_ = ::std::option::Option::Some(::std::rc::Rc::clone(other));
        while let ::std::option::Option::Some(candidate_) = current_ {
            if ::std::rc::Rc::ptr_eq(&candidate_, node) {
                return true;
            }
            current_ = Methods::parent(&candidate_);
        }
        false
    }

    /// Runs `start` (the first time) and `update` of every node of the tree
    /// of `root`, parents first. Nodes added during the frame start in the
    /// next one; nodes removed during it are skipped. `nodes_` is a reused
    /// list, left empty.
    pub fn process_tree_(game_: &crate::classes::Game::Ref, root: &Ref, delta: f32, nodes_: &mut ::std::vec::Vec<Ref>) {
        nodes_.push(::std::rc::Rc::clone(root));
        let mut next_ = 0;
        while let ::std::option::Option::Some(node_) = nodes_.get(next_).cloned() {
            nodes_.extend(HasFields::fields(&*node_).children_.borrow().iter().cloned());
            next_ += 1;
        }
        for node_ in nodes_.iter() {
            if !is_same_or_ancestor_(root, node_) {
                continue;
            }
            let fields_ = HasFields::fields(&**node_);
            if !fields_.started_.get() {
                fields_.started_.set(true);
                crate::classes::Game::see_from_if_main_(game_, node_);
                Object::start(::std::rc::Rc::clone(node_));
            }
            Object::update(::std::rc::Rc::clone(node_), delta);
        }
        nodes_.clear();
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
        make_(Fields {
            name: ::star_script_runtime::Field::new(name),
            x: ::star_script_runtime::Field::new(0.0),
            y: ::star_script_runtime::Field::new(0.0),
            rotation: ::star_script_runtime::Field::new(0.0),
            scale_x: ::star_script_runtime::Field::new(1.0),
            scale_y: ::star_script_runtime::Field::new(1.0),
            body: ::star_script_runtime::Field::new(::std::option::Option::None),
            tags: ::star_script_runtime::Field::new(::star_script_runtime::List::new()),
            id_: NEXT_ID_.fetch_add(1, ::std::sync::atomic::Ordering::Relaxed),
            parent_: ::std::cell::RefCell::new(::std::option::Option::None),
            children_: ::std::cell::RefCell::new(::std::vec::Vec::new()),
            started_: ::std::cell::Cell::new(false),
        })
    }

    /// Creates an object of this class.
    pub fn new(name: ::std::string::String) -> Ref {
        init_(name, |fields0_| ::std::rc::Rc::new(Instance { Node: fields0_ }))
    }
}";

const SPRITE_MODULE: &str = "/// `class Sprite`, provided by the engine.
pub mod Sprite {
    /// A reference to an object of this class or of a subclass.
    pub type Ref = ::std::rc::Rc<dyn Object>;

    /// The fields of a sprite: the image it shows and how.
    pub struct Fields {
        pub texture: ::star_script_runtime::Field<::std::string::String>,
        pub width: ::star_script_runtime::Field<f32>,
        pub height: ::star_script_runtime::Field<f32>,
        pub layer: ::star_script_runtime::Field<i32>,
        pub pixelated: ::star_script_runtime::Field<bool>,
        pub color: ::star_script_runtime::Field<crate::classes::Color::Ref>,
    }

    /// Gives access to the fields of this class, in objects of any subclass.
    pub trait HasFields {
        fn fields(&self) -> &Fields;
    }

    /// A sprite adds no methods to those of a node.
    pub trait Object: crate::classes::Node::Object + HasFields {}

    /// A sprite adds no methods to those of a node.
    pub struct Methods;

    /// An object created as this class (not as a subclass).
    pub struct Instance {
        pub Node: crate::classes::Node::Fields,
        pub Sprite: Fields,
    }

    impl crate::classes::Node::HasFields for Instance {
        fn fields(&self) -> &crate::classes::Node::Fields { &self.Node }
    }

    impl HasFields for Instance {
        fn fields(&self) -> &Fields { &self.Sprite }
    }

    impl crate::AnyObject for Instance {
        fn Node(self: ::std::rc::Rc<Self>) -> ::std::option::Option<crate::classes::Node::Ref> { ::std::option::Option::Some(self) }
        fn Sprite(self: ::std::rc::Rc<Self>) -> ::std::option::Option<Ref> { ::std::option::Option::Some(self) }
    }

    impl crate::classes::Node::Object for Instance {
        fn start(self: ::std::rc::Rc<Self>) { crate::classes::Node::Methods::start(&(self as crate::classes::Node::Ref)) }
        fn update(self: ::std::rc::Rc<Self>, delta: f32) { crate::classes::Node::Methods::update(&(self as crate::classes::Node::Ref), delta) }
        fn add_child(self: ::std::rc::Rc<Self>, child: crate::classes::Node::Ref) { crate::classes::Node::Methods::add_child(&(self as crate::classes::Node::Ref), child) }
        fn remove(self: ::std::rc::Rc<Self>) { crate::classes::Node::Methods::remove(&(self as crate::classes::Node::Ref)) }
        fn parent(self: ::std::rc::Rc<Self>) -> ::std::option::Option<crate::classes::Node::Ref> { crate::classes::Node::Methods::parent(&(self as crate::classes::Node::Ref)) }
        fn children(self: ::std::rc::Rc<Self>) -> ::star_script_runtime::List<crate::classes::Node::Ref> { crate::classes::Node::Methods::children(&(self as crate::classes::Node::Ref)) }
        fn find_child(self: ::std::rc::Rc<Self>, name: ::std::string::String) -> ::std::option::Option<crate::classes::Node::Ref> { crate::classes::Node::Methods::find_child(&(self as crate::classes::Node::Ref), name) }
        fn has_tag(self: ::std::rc::Rc<Self>, tag: ::std::string::String) -> bool { crate::classes::Node::Methods::has_tag(&(self as crate::classes::Node::Ref), tag) }
        fn find_by_name(self: ::std::rc::Rc<Self>, name: ::std::string::String) -> ::std::option::Option<crate::classes::Node::Ref> { crate::classes::Node::Methods::find_by_name(&(self as crate::classes::Node::Ref), name) }
        fn find_by_tag(self: ::std::rc::Rc<Self>, tag: ::std::string::String) -> ::star_script_runtime::List<crate::classes::Node::Ref> { crate::classes::Node::Methods::find_by_tag(&(self as crate::classes::Node::Ref), tag) }
        fn global_x(self: ::std::rc::Rc<Self>) -> f32 { crate::classes::Node::Methods::global_x(&(self as crate::classes::Node::Ref)) }
        fn global_y(self: ::std::rc::Rc<Self>) -> f32 { crate::classes::Node::Methods::global_y(&(self as crate::classes::Node::Ref)) }
        fn global_rotation(self: ::std::rc::Rc<Self>) -> f32 { crate::classes::Node::Methods::global_rotation(&(self as crate::classes::Node::Ref)) }
        fn set_global_position(self: ::std::rc::Rc<Self>, x: f32, y: f32) { crate::classes::Node::Methods::set_global_position(&(self as crate::classes::Node::Ref), x, y) }
        fn set_global_rotation(self: ::std::rc::Rc<Self>, angle: f32) { crate::classes::Node::Methods::set_global_rotation(&(self as crate::classes::Node::Ref), angle) }
        fn translate(self: ::std::rc::Rc<Self>, x: f32, y: f32) { crate::classes::Node::Methods::translate(&(self as crate::classes::Node::Ref), x, y) }
        fn translate_global(self: ::std::rc::Rc<Self>, x: f32, y: f32) { crate::classes::Node::Methods::translate_global(&(self as crate::classes::Node::Ref), x, y) }
        fn rotate(self: ::std::rc::Rc<Self>, angle: f32) { crate::classes::Node::Methods::rotate(&(self as crate::classes::Node::Ref), angle) }
    }

    impl Object for Instance {}

    /// Prepares an object of this class: `make_` builds it from the fields
    /// of a node and of a sprite.
    pub fn init_<O: Object + 'static>(
        name: ::std::string::String,
        texture: ::std::string::String,
        make_: impl FnOnce(crate::classes::Node::Fields, Fields) -> ::std::rc::Rc<O>,
    ) -> ::std::rc::Rc<O> {
        let fields_ = Fields {
            texture: ::star_script_runtime::Field::new(texture),
            width: ::star_script_runtime::Field::new(1.0),
            height: ::star_script_runtime::Field::new(1.0),
            layer: ::star_script_runtime::Field::new(0),
            pixelated: ::star_script_runtime::Field::new(false),
            color: ::star_script_runtime::Field::new(crate::classes::Color::new(1.0, 1.0, 1.0, 1.0)),
        };
        crate::classes::Node::init_(name, move |base0_| make_(base0_, fields_))
    }

    /// Creates an object of this class.
    pub fn new(name: ::std::string::String, texture: ::std::string::String) -> Ref {
        init_(name, texture, |fields0_, fields1_| ::std::rc::Rc::new(Instance { Node: fields0_, Sprite: fields1_ }))
    }
}";

const CAMERA_MODULE: &str = "/// `class Camera2D`, provided by the engine.
pub mod Camera2D {
    /// A reference to an object of this class or of a subclass.
    pub type Ref = ::std::rc::Rc<dyn Object>;

    /// The fields of a camera: how much of the world it shows, and whether
    /// it becomes the game's camera when it enters the game's tree.
    pub struct Fields {
        pub view_height: ::star_script_runtime::Field<f32>,
        pub main: ::star_script_runtime::Field<bool>,
    }

    /// Gives access to the fields of this class, in objects of any subclass.
    pub trait HasFields {
        fn fields(&self) -> &Fields;
    }

    /// A camera adds no methods to those of a node.
    pub trait Object: crate::classes::Node::Object + HasFields {}

    /// A camera adds no methods to those of a node.
    pub struct Methods;

    /// An object created as this class (not as a subclass).
    pub struct Instance {
        pub Node: crate::classes::Node::Fields,
        pub Camera2D: Fields,
    }

    impl crate::classes::Node::HasFields for Instance {
        fn fields(&self) -> &crate::classes::Node::Fields { &self.Node }
    }

    impl HasFields for Instance {
        fn fields(&self) -> &Fields { &self.Camera2D }
    }

    impl crate::AnyObject for Instance {
        fn Node(self: ::std::rc::Rc<Self>) -> ::std::option::Option<crate::classes::Node::Ref> { ::std::option::Option::Some(self) }
        fn Camera2D(self: ::std::rc::Rc<Self>) -> ::std::option::Option<Ref> { ::std::option::Option::Some(self) }
    }

    impl crate::classes::Node::Object for Instance {
        fn start(self: ::std::rc::Rc<Self>) { crate::classes::Node::Methods::start(&(self as crate::classes::Node::Ref)) }
        fn update(self: ::std::rc::Rc<Self>, delta: f32) { crate::classes::Node::Methods::update(&(self as crate::classes::Node::Ref), delta) }
        fn add_child(self: ::std::rc::Rc<Self>, child: crate::classes::Node::Ref) { crate::classes::Node::Methods::add_child(&(self as crate::classes::Node::Ref), child) }
        fn remove(self: ::std::rc::Rc<Self>) { crate::classes::Node::Methods::remove(&(self as crate::classes::Node::Ref)) }
        fn parent(self: ::std::rc::Rc<Self>) -> ::std::option::Option<crate::classes::Node::Ref> { crate::classes::Node::Methods::parent(&(self as crate::classes::Node::Ref)) }
        fn children(self: ::std::rc::Rc<Self>) -> ::star_script_runtime::List<crate::classes::Node::Ref> { crate::classes::Node::Methods::children(&(self as crate::classes::Node::Ref)) }
        fn find_child(self: ::std::rc::Rc<Self>, name: ::std::string::String) -> ::std::option::Option<crate::classes::Node::Ref> { crate::classes::Node::Methods::find_child(&(self as crate::classes::Node::Ref), name) }
        fn has_tag(self: ::std::rc::Rc<Self>, tag: ::std::string::String) -> bool { crate::classes::Node::Methods::has_tag(&(self as crate::classes::Node::Ref), tag) }
        fn find_by_name(self: ::std::rc::Rc<Self>, name: ::std::string::String) -> ::std::option::Option<crate::classes::Node::Ref> { crate::classes::Node::Methods::find_by_name(&(self as crate::classes::Node::Ref), name) }
        fn find_by_tag(self: ::std::rc::Rc<Self>, tag: ::std::string::String) -> ::star_script_runtime::List<crate::classes::Node::Ref> { crate::classes::Node::Methods::find_by_tag(&(self as crate::classes::Node::Ref), tag) }
        fn global_x(self: ::std::rc::Rc<Self>) -> f32 { crate::classes::Node::Methods::global_x(&(self as crate::classes::Node::Ref)) }
        fn global_y(self: ::std::rc::Rc<Self>) -> f32 { crate::classes::Node::Methods::global_y(&(self as crate::classes::Node::Ref)) }
        fn global_rotation(self: ::std::rc::Rc<Self>) -> f32 { crate::classes::Node::Methods::global_rotation(&(self as crate::classes::Node::Ref)) }
        fn set_global_position(self: ::std::rc::Rc<Self>, x: f32, y: f32) { crate::classes::Node::Methods::set_global_position(&(self as crate::classes::Node::Ref), x, y) }
        fn set_global_rotation(self: ::std::rc::Rc<Self>, angle: f32) { crate::classes::Node::Methods::set_global_rotation(&(self as crate::classes::Node::Ref), angle) }
        fn translate(self: ::std::rc::Rc<Self>, x: f32, y: f32) { crate::classes::Node::Methods::translate(&(self as crate::classes::Node::Ref), x, y) }
        fn translate_global(self: ::std::rc::Rc<Self>, x: f32, y: f32) { crate::classes::Node::Methods::translate_global(&(self as crate::classes::Node::Ref), x, y) }
        fn rotate(self: ::std::rc::Rc<Self>, angle: f32) { crate::classes::Node::Methods::rotate(&(self as crate::classes::Node::Ref), angle) }
    }

    impl Object for Instance {}

    /// Prepares an object of this class: `make_` builds it from the fields
    /// of a node and of a camera.
    pub fn init_<O: Object + 'static>(
        name: ::std::string::String,
        make_: impl FnOnce(crate::classes::Node::Fields, Fields) -> ::std::rc::Rc<O>,
    ) -> ::std::rc::Rc<O> {
        let fields_ = Fields {
            view_height: ::star_script_runtime::Field::new(2.0),
            main: ::star_script_runtime::Field::new(false),
        };
        crate::classes::Node::init_(name, move |base0_| make_(base0_, fields_))
    }

    /// Creates an object of this class.
    pub fn new(name: ::std::string::String) -> Ref {
        init_(name, |fields0_, fields1_| ::std::rc::Rc::new(Instance { Node: fields0_, Camera2D: fields1_ }))
    }
}";

const SCENE_MODULE: &str = "/// `class Scene`, provided by the engine.
pub mod Scene {
    /// `static func load`: a new tree with the nodes of the scene at `path`,
    /// out of the game until it is added; `None` if it cannot be loaded.
    pub fn load(path: ::std::string::String) -> ::std::option::Option<crate::classes::Node::Ref> {
        let nodes_ = ::star_script_runtime::scenes::load(&path)?;
        match build_(&nodes_) {
            ::std::result::Result::Ok(root_) => ::std::option::Option::Some(root_),
            ::std::result::Result::Err(reason_) => {
                ::star_script_runtime::scenes::reject(&path, &reason_);
                ::std::option::Option::None
            }
        }
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

    /// The objects of `nodes_`, each one added to its parent; the root first.
    fn build_(nodes_: &[::star_script_runtime::scenes::SceneNode]) -> ::std::result::Result<crate::classes::Node::Ref, ::std::string::String> {
        let mut built_: ::std::vec::Vec<crate::classes::Node::Ref> = ::std::vec::Vec::with_capacity(nodes_.len());
        for node_ in nodes_ {
            let object_ = build_node_(node_).map_err(|reason_| ::std::format!(\"node \\\"{}\\\": {reason_}\", node_.name))?;
            if let ::std::option::Option::Some(parent_) = node_.parent.and_then(|index_| built_.get(index_)) {
                crate::classes::Node::Methods::add_child(parent_, ::std::rc::Rc::clone(&object_));
            }
            built_.push(object_);
        }
        built_.into_iter().next().ok_or_else(|| ::std::string::String::from(\"it has no nodes\"))
    }

    /// The object of one node: of its class, created as `init()` leaves it,
    /// with what the scene writes on top.
    fn build_node_(node_: &::star_script_runtime::scenes::SceneNode) -> ::std::result::Result<crate::classes::Node::Ref, ::std::string::String> {
        let name_ = node_.name.clone();
        let missing_ = |what_: &str| ::std::result::Result::Err(::std::format!(\"a `{what_}` needs a `[node.{}]`\", what_.to_lowercase()));
        let object_: crate::classes::Node::Ref = match node_.class.as_deref() {
            ::std::option::Option::None => default_object_(node_, name_),
            ::std::option::Option::Some(\"Node\") => crate::classes::Node::new(name_),
            ::std::option::Option::Some(\"Sprite\") => match &node_.sprite {
                ::std::option::Option::Some(image_) => crate::classes::Sprite::new(name_, image_.texture.clone()),
                ::std::option::Option::None => return missing_(\"Sprite\"),
            },
            ::std::option::Option::Some(\"Camera2D\") => crate::classes::Camera2D::new(name_),
            ::std::option::Option::Some(\"Light2D\") => match &node_.light_2d {
                ::std::option::Option::Some(light_) => crate::classes::Light2D::new(name_, light_.kind.clone()),
                ::std::option::Option::None => return ::std::result::Result::Err(::std::string::String::from(\"a `Light2D` needs a `[node.light_2d]`\")),
            },
            ::std::option::Option::Some(\"Node3D\") => crate::classes::Node3D::new(name_),
            ::std::option::Option::Some(\"Mesh\") => match &node_.mesh {
                ::std::option::Option::Some(mesh_) => crate::classes::Mesh::new(name_, mesh_.shape.clone()),
                ::std::option::Option::None => return missing_(\"Mesh\"),
            },
            ::std::option::Option::Some(\"Light\") => match &node_.light {
                ::std::option::Option::Some(light_) => crate::classes::Light::new(name_, light_.kind.clone()),
                ::std::option::Option::None => return missing_(\"Light\"),
            },
            ::std::option::Option::Some(\"Camera3D\") => crate::classes::Camera3D::new(name_),
            ::std::option::Option::Some(class_) => crate::new_scene_object_(class_)?,
        };
        let class_ = node_.class.as_deref().unwrap_or(\"Node\");
        let fields_ = crate::classes::Node::HasFields::fields(&*object_);
        fields_.name.set(node_.name.clone());
        if !node_.tags.is_empty() {
            fields_.tags.set(::star_script_runtime::List::from_vec(node_.tags.clone()));
        }
        if !node_.fields.is_empty() {
            crate::set_scene_fields_(class_, &object_, &node_.fields)?;
        }
        if let ::std::option::Option::Some(place_) = &node_.transform {
            fields_.x.set(place_.x);
            fields_.y.set(place_.y);
            fields_.rotation.set(place_.rotation);
            fields_.scale_x.set(place_.scale_x);
            fields_.scale_y.set(place_.scale_y);
        }
        if let ::std::option::Option::Some(place_) = &node_.transform_3d {
            let node_3d_ = crate::AnyObject::Node3D(::std::rc::Rc::clone(&object_))
                .ok_or_else(|| ::std::format!(\"it is placed in 3D, but `{class_}` is not a `Node3D`\"))?;
            fields_.x.set(place_.x);
            fields_.y.set(place_.y);
            fields_.rotation.set(place_.roll);
            fields_.scale_x.set(place_.scale_x);
            fields_.scale_y.set(place_.scale_y);
            let depth_ = crate::classes::Node3D::HasFields::fields(&*node_3d_);
            depth_.z.set(place_.z);
            depth_.pitch.set(place_.pitch);
            depth_.yaw.set(place_.yaw);
            depth_.scale_z.set(place_.scale_z);
        }
        if let ::std::option::Option::Some(image_) = &node_.sprite {
            let sprite_ = crate::AnyObject::Sprite(::std::rc::Rc::clone(&object_))
                .ok_or_else(|| ::std::format!(\"it shows an image, but `{class_}` is not a `Sprite`\"))?;
            let look_ = crate::classes::Sprite::HasFields::fields(&*sprite_);
            look_.texture.set(image_.texture.clone());
            look_.width.set(image_.width);
            look_.height.set(image_.height);
            look_.layer.set(image_.layer);
            look_.pixelated.set(image_.pixelated);
            let [r_, g_, b_, a_] = image_.color;
            look_.color.set(crate::classes::Color::new(r_, g_, b_, a_));
        }
        if let ::std::option::Option::Some(shown_) = &node_.mesh {
            let mesh_ = crate::AnyObject::Mesh(::std::rc::Rc::clone(&object_))
                .ok_or_else(|| ::std::format!(\"it shows a mesh, but `{class_}` is not a `Mesh`\"))?;
            let look_ = crate::classes::Mesh::HasFields::fields(&*mesh_);
            look_.shape.set(shown_.shape.clone());
            look_.width.set(shown_.width);
            look_.height.set(shown_.height);
            look_.depth.set(shown_.depth);
            look_.radius.set(shown_.radius);
            let [r_, g_, b_, a_] = shown_.color;
            look_.color.set(crate::classes::Color::new(r_, g_, b_, a_));
            look_.metallic.set(shown_.metallic);
            look_.roughness.set(shown_.roughness);
            look_.texture.set(shown_.texture.clone());
            look_.pixelated.set(shown_.pixelated);
        }
        if let ::std::option::Option::Some(shone_) = &node_.light {
            let light_ = crate::AnyObject::Light(::std::rc::Rc::clone(&object_))
                .ok_or_else(|| ::std::format!(\"it shines a light, but `{class_}` is not a `Light`\"))?;
            let look_ = crate::classes::Light::HasFields::fields(&*light_);
            look_.kind.set(shone_.kind.clone());
            let [r_, g_, b_, a_] = shone_.color;
            look_.color.set(crate::classes::Color::new(r_, g_, b_, a_));
            look_.intensity.set(shone_.intensity);
            look_.range.set(shone_.range);
            look_.angle.set(shone_.angle);
            look_.blend.set(shone_.blend);
        }
        if let ::std::option::Option::Some(shone_) = &node_.light_2d {
            let light_ = crate::AnyObject::Light2D(::std::rc::Rc::clone(&object_))
                .ok_or_else(|| ::std::format!(\"it lights the sprites, but `{class_}` is not a `Light2D`\"))?;
            let look_ = crate::classes::Light2D::HasFields::fields(&*light_);
            look_.kind.set(shone_.kind.clone());
            let [r_, g_, b_, a_] = shone_.color;
            look_.color.set(crate::classes::Color::new(r_, g_, b_, a_));
            look_.intensity.set(shone_.intensity);
            look_.range.set(shone_.range);
            look_.angle.set(shone_.angle);
            look_.blend.set(shone_.blend);
        }
        if let ::std::option::Option::Some(view_) = &node_.camera {
            let camera_ = crate::AnyObject::Camera2D(::std::rc::Rc::clone(&object_))
                .ok_or_else(|| ::std::format!(\"it is a 2D camera, but `{class_}` is not a `Camera2D`\"))?;
            let look_ = crate::classes::Camera2D::HasFields::fields(&*camera_);
            look_.view_height.set(view_.view_height);
            look_.main.set(view_.main);
        }
        if let ::std::option::Option::Some(view_) = &node_.camera_3d {
            let camera_ = crate::AnyObject::Camera3D(::std::rc::Rc::clone(&object_))
                .ok_or_else(|| ::std::format!(\"it is a 3D camera, but `{class_}` is not a `Camera3D`\"))?;
            let look_ = crate::classes::Camera3D::HasFields::fields(&*camera_);
            look_.fov.set(view_.fov);
            look_.main.set(view_.main);
        }
        if let ::std::option::Option::Some(solid_) = &node_.body {
            let body_ = crate::classes::Body::new_(solid_.kind.clone(), &solid_.shape, solid_.radius, solid_.width, solid_.height, solid_.depth);
            let surface_ = crate::classes::Body::HasFields::fields(&*body_);
            surface_.gravity_scale.set(solid_.gravity_scale);
            surface_.fixed_rotation.set(solid_.fixed_rotation);
            surface_.friction.set(solid_.friction);
            surface_.bounciness.set(solid_.bounciness);
            surface_.density.set(solid_.density);
            surface_.sensor.set(solid_.sensor);
            fields_.body.set(::std::option::Option::Some(body_));
        }
        ::std::result::Result::Ok(object_)
    }

    /// The object of a node whose class the scene does not name: of the
    /// engine class that can show what it has.
    fn default_object_(node_: &::star_script_runtime::scenes::SceneNode, name_: ::std::string::String) -> crate::classes::Node::Ref {
        if let ::std::option::Option::Some(image_) = &node_.sprite {
            crate::classes::Sprite::new(name_, image_.texture.clone())
        } else if let ::std::option::Option::Some(mesh_) = &node_.mesh {
            crate::classes::Mesh::new(name_, mesh_.shape.clone())
        } else if let ::std::option::Option::Some(light_) = &node_.light {
            crate::classes::Light::new(name_, light_.kind.clone())
        } else if let ::std::option::Option::Some(light_) = &node_.light_2d {
            crate::classes::Light2D::new(name_, light_.kind.clone())
        } else if node_.camera_3d.is_some() {
            crate::classes::Camera3D::new(name_)
        } else if node_.camera.is_some() {
            crate::classes::Camera2D::new(name_)
        } else if node_.transform_3d.is_some() {
            crate::classes::Node3D::new(name_)
        } else {
            crate::classes::Node::new(name_)
        }
    }
}";

const SOUND_MODULE: &str = "/// `class Sound`, provided by the engine.
pub mod Sound {
    /// A reference to an object of this class.
    pub type Ref = ::std::rc::Rc<dyn Object>;

    /// The fields of a sound: how it plays, which sound of the runtime it
    /// is, and the playings it started that may still sound.
    pub struct Fields {
        pub volume: ::star_script_runtime::Field<f32>,
        pub looping: ::star_script_runtime::Field<bool>,
        pub sound_: ::star_script_runtime::audio::SoundId,
        pub voices_: ::std::cell::RefCell<::std::vec::Vec<::star_script_runtime::audio::VoiceId>>,
    }

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

    /// The methods of a sound.
    pub struct Methods;

    impl Methods {
        /// `func play`: starts the sound once more, at its `volume`.
        pub fn play(this_: &Ref) {
            let fields_ = HasFields::fields(&**this_);
            let mut voices_ = fields_.voices_.borrow_mut();
            voices_.retain(|voice_| ::star_script_runtime::audio::is_playing(*voice_));
            if let ::std::option::Option::Some(voice_) = ::star_script_runtime::audio::play(fields_.sound_, fields_.volume.get(), fields_.looping.get()) {
                voices_.push(voice_);
            }
        }

        /// `func stop`: stops every playing this sound started.
        pub fn stop(this_: &Ref) {
            for voice_ in HasFields::fields(&**this_).voices_.borrow_mut().drain(..) {
                ::star_script_runtime::audio::stop(voice_);
            }
        }

        /// `func is_playing`: whether a playing it started still sounds.
        pub fn is_playing(this_: &Ref) -> bool {
            let mut voices_ = HasFields::fields(&**this_).voices_.borrow_mut();
            voices_.retain(|voice_| ::star_script_runtime::audio::is_playing(*voice_));
            !voices_.is_empty()
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

    /// `static func load`: the sound at `path`, at full volume and not
    /// looping; `None` if it cannot be loaded. Sounds of the same file
    /// share its samples.
    pub fn load(path: ::std::string::String) -> ::std::option::Option<Ref> {
        let sound_ = ::star_script_runtime::audio::load(&path)?;
        ::std::option::Option::Some(::std::rc::Rc::new(Instance {
            Sound: Fields {
                volume: ::star_script_runtime::Field::new(1.0),
                looping: ::star_script_runtime::Field::new(false),
                sound_,
                voices_: ::std::cell::RefCell::new(::std::vec::Vec::new()),
            },
        }))
    }
}";

/// `EngineGame_`: connects the StarScript game with the engine's game loop.
fn engine_game_loop() -> String {
    format!(
        "
/// Connects the StarScript game with the engine's game loop.
struct EngineGame_ {{
    /// Where `Sound`s play: silent until the engine gives the game its audio.
    audio: ::std::rc::Rc<::std::cell::RefCell<::star_engine::star_audio::Audio>>,
    game: crate::classes::Game::Ref,
    /// Reused list of the nodes whose code runs in a frame.
    nodes: ::std::vec::Vec<crate::classes::Node::Ref>,
    /// Reused list of the nodes still to mirror, with their parent's number.
    pending: ::std::vec::Vec<(crate::classes::Node::Ref, ::std::option::Option<u64>)>,
    /// Reused map of the nodes of the tree by their number, to tell bodies
    /// what they touch.
    known: ::std::collections::HashMap<u64, crate::classes::Node::Ref>,
    /// The node of the engine's world that mirrors each node of the game.
    mirror: ::star_engine::script_nodes::ScriptNodes,
}}

impl ::star_engine::game::Game for EngineGame_ {{
    fn start(
        &mut self,
        context: &mut ::star_engine::game::GameContext<'_>,
    ) -> ::star_engine::star_core::EngineResult<()> {{
        use ::star_engine::EngineWorld as _;
        *self.audio.borrow_mut() = context.world().audio();
        crate::classes::Game::Object::start(::std::rc::Rc::clone(&self.game));
        ::std::result::Result::Ok(())
    }}

    fn update(&mut self, world: &mut ::star_engine::star_ecs::World) {{
        use ::star_engine::EngineWorld as _;
        if let ::std::option::Option::Some(input) = world.input() {{
            share_actions_(&input);
            share_pointer_(&input, &self.view_(world), world.window_size());
        }}
        let delta_ = world.time().delta_seconds();
        let root_ = ::std::rc::Rc::clone(&crate::classes::Game::HasFields::fields(&*self.game).root_);
        sync_bodies_(world, &mut self.mirror, &root_, &mut self.known, &mut self.nodes);
        {RUNTIME}::tasks::frame_started(delta_);
        crate::classes::Game::Object::update(::std::rc::Rc::clone(&self.game), delta_);
        crate::classes::Node::process_tree_(&self.game, &root_, delta_, &mut self.nodes);
        {RUNTIME}::tasks::resume();
        mirror_nodes_(world, &mut self.mirror, &root_, &mut self.pending);
    }}

    fn draw(&self, world: &::star_engine::star_ecs::World, frame: &mut ::star_engine::star_render::Frame) {{
        let fields_ = crate::classes::Game::HasFields::fields(&*self.game);
        frame.set_clear_color(fields_.background.read(|color_| engine_color_(&**color_)));
        let ambient_ = fields_.ambient.read(|color_| engine_color_(&**color_));
        frame.set_ambient_light(::star_engine::star_render::AmbientLight::new(ambient_, 1.0));
        let ambient_2d_ = fields_.ambient_2d.read(|color_| engine_color_(&**color_));
        frame.set_ambient_light_2d(::star_engine::star_render::AmbientLight::new(ambient_2d_, 1.0));
        frame.set_camera(self.view_(world));
    }}

    fn is_finished(&self) -> bool {{
        crate::classes::Game::HasFields::fields(&*self.game).{QUIT_FIELD}.get()
    }}
}}

impl EngineGame_ {{
    /// The engine's camera that shows the game from its `camera_3d`, or
    /// else from its `camera`.
    fn view_(&self, world: &::star_engine::star_ecs::World) -> ::star_engine::star_render::Camera {{
        let fields_ = crate::classes::Game::HasFields::fields(&*self.game);
        if let ::std::option::Option::Some(camera_) = fields_.camera_3d.get() {{
            return self.mirror.camera_3d(world, &script_camera_3d_(&*camera_));
        }}
        fields_.camera.read(|camera_| match camera_ {{
            ::std::option::Option::Some(camera_) => self.mirror.camera(world, &script_camera_(&**camera_)),
            ::std::option::Option::None => ::star_engine::star_render::Camera::default(),
        }})
    }}
}}"
    )
}

/// `fn main` that runs the game `class_path` in a window of the engine, with
/// the input `actions` of the project.
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
        .with_title({title:?})
        .with_actions(actions_);
    let folder_ = ::std::clone::Clone::clone(app_.asset_folder());
    let audio_ = ::std::rc::Rc::new(::std::cell::RefCell::new(::star_engine::star_audio::Audio::silent()));
    {RUNTIME}::audio::set_backend(EngineAudio_ {{
        folder: ::std::clone::Clone::clone(&folder_),
        output: ::std::rc::Rc::clone(&audio_),
        sounds: ::std::vec::Vec::new(),
    }});
    {RUNTIME}::scenes::set_loader(move |path_| read_scene_(&folder_, path_));
    // Created after the loaders are set, so its `init` can load scenes and
    // sounds too.
    let game_ = EngineGame_ {{
        audio: audio_,
        game: {new_game},
        nodes: ::std::vec::Vec::new(),
        pending: ::std::vec::Vec::new(),
        known: ::std::collections::HashMap::new(),
        mirror: ::star_engine::script_nodes::ScriptNodes::new(),
    }};
    app_.run_game(game_)
}}
"
    );
    let engine_game = engine_game_loop();
    let support = [SHARE_INPUT, MIRROR_NODES, READ_SCENE, ENGINE_AUDIO];
    for line in main.lines().chain(engine_game.lines()).chain(
        support
            .iter()
            .flat_map(|code| [""].into_iter().chain(code.lines())),
    ) {
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

const SHARE_INPUT: &str =
    "/// Copies where the player points, in world units, where `Input` reads it.
/// The camera is the one the last frame was drawn from: what the player saw.
fn share_pointer_(
    input_: &::star_engine::star_input::Input,
    view_: &::star_engine::star_render::Camera,
    window_: ::star_engine::star_platform::WindowSize,
) {
    let position_ = input_.pointer().and_then(|pixel_| view_.screen_to_world_2d(pixel_, window_));
    ::star_script_runtime::input::set_pointer(position_.map(|point_| (point_.x, point_.y)));
}

/// Copies the state of each action where `Input` reads it.
fn share_actions_(input_: &::star_engine::star_input::Input) {
    let actions_ = input_.actions();
    for (index_, name_) in ACTION_NAMES_.iter().enumerate() {
        let state_ = ::star_script_runtime::input::ActionState {
            is_down: actions_.is_down(name_),
            just_pressed: actions_.just_pressed(name_),
            just_released: actions_.just_released(name_),
            strength: input_.action_strength(name_),
        };
        ::star_script_runtime::input::set_action_state(index_, state_);
    }
}";

const MIRROR_NODES: &str = "/// Describes every node of the game's tree to the engine, parents first, so
/// the world has a node in step with each one.
fn mirror_nodes_(
    world_: &mut ::star_engine::star_ecs::World,
    mirror_: &mut ::star_engine::script_nodes::ScriptNodes,
    root_: &crate::classes::Node::Ref,
    pending_: &mut ::std::vec::Vec<(crate::classes::Node::Ref, ::std::option::Option<u64>)>,
) {
    mirror_.begin_frame();
    pending_.push((::std::rc::Rc::clone(root_), ::std::option::Option::None));
    while let ::std::option::Option::Some((node_, parent_)) = pending_.pop() {
        let fields_ = crate::classes::Node::HasFields::fields(&*node_);
        let transform_ = ::star_engine::star_scene::Transform2D {
            position: ::star_engine::star_math::Vec2::new(fields_.x.get(), fields_.y.get()),
            rotation: fields_.rotation.get(),
            scale: ::star_engine::star_math::Vec2::new(fields_.scale_x.get(), fields_.scale_y.get()),
        };
        let transform_3d_ = crate::AnyObject::Node3D(::std::rc::Rc::clone(&node_)).map(|node_3d_| {
            let depth_ = crate::classes::Node3D::HasFields::fields(&*node_3d_);
            let angles_ = ::star_engine::star_math::Vec3::new(depth_.pitch.get(), depth_.yaw.get(), fields_.rotation.get());
            ::star_engine::star_scene::Transform3D {
                position: ::star_engine::star_math::Vec3::new(fields_.x.get(), fields_.y.get(), depth_.z.get()),
                rotation: ::star_engine::star_math::Quat::from_euler(angles_),
                scale: ::star_engine::star_math::Vec3::new(fields_.scale_x.get(), fields_.scale_y.get(), depth_.scale_z.get()),
            }
        });
        fields_.name.read(|name_| {
            with_sprite_(&node_, |sprite_| {
                with_body_(fields_, |body_| {
                    with_mesh_(&node_, |mesh_| {
                        with_light_(&node_, |light_| {
                          with_light_2d_(&node_, |light_2d_| {
                            let described_ = ::star_engine::script_nodes::ScriptNode {
                                id: fields_.id_,
                                parent: parent_,
                                name: name_,
                                transform: transform_,
                                transform_3d: transform_3d_,
                                sprite: sprite_,
                                body: body_,
                                mesh: mesh_,
                                light: light_,
                                light_2d: light_2d_,
                            };
                            mirror_.mirror(world_, &described_);
                          });
                        });
                    });
                });
            });
        });
        for child_ in fields_.children_.borrow().iter().rev() {
            pending_.push((::std::rc::Rc::clone(child_), ::std::option::Option::Some(fields_.id_)));
        }
    }
    mirror_.finish_frame(world_);
}

/// Runs `then_` with what the engine needs to know about the image `node_`
/// shows, if it is a `Sprite`.
fn with_sprite_(
    node_: &crate::classes::Node::Ref,
    then_: impl FnOnce(::std::option::Option<::star_engine::script_nodes::ScriptSprite<'_>>),
) {
    let ::std::option::Option::Some(sprite_) = crate::AnyObject::Sprite(::std::rc::Rc::clone(node_)) else {
        return then_(::std::option::Option::None);
    };
    let look_ = crate::classes::Sprite::HasFields::fields(&*sprite_);
    look_.texture.read(|texture_| {
        then_(::std::option::Option::Some(::star_engine::script_nodes::ScriptSprite {
            texture: texture_,
            size: ::star_engine::star_math::Vec2::new(look_.width.get(), look_.height.get()),
            layer: look_.layer.get(),
            pixelated: look_.pixelated.get(),
            color: look_.color.read(|color_| engine_color_(&**color_)),
        }));
    });
}

/// Runs `then_` with what the engine needs to know about the node's body,
/// if it has one, and forgets the impulses given to it.
fn with_body_(
    fields_: &crate::classes::Node::Fields,
    then_: impl FnOnce(::std::option::Option<::star_engine::script_nodes::ScriptBody<'_>>),
) {
    let ::std::option::Option::Some(body_) = fields_.body.get() else {
        return then_(::std::option::Option::None);
    };
    let body_ = crate::classes::Body::HasFields::fields(&*body_);
    let (impulse_x_, impulse_y_, impulse_z_) = body_.impulse_.replace((0.0, 0.0, 0.0));
    body_.kind.read(|kind_| {
        body_.shape.read(|shape_| {
            then_(::std::option::Option::Some(::star_engine::script_nodes::ScriptBody {
                kind: kind_,
                shape: shape_,
                radius: body_.radius.get(),
                width: body_.width.get(),
                height: body_.height.get(),
                depth: body_.depth.get(),
                gravity_scale: body_.gravity_scale.get(),
                fixed_rotation: body_.fixed_rotation.get(),
                friction: body_.friction.get(),
                bounciness: body_.bounciness.get(),
                density: body_.density.get(),
                sensor: body_.sensor.get(),
                velocity: ::star_engine::star_math::Vec3::new(body_.velocity_x.get(), body_.velocity_y.get(), body_.velocity_z.get()),
                impulse: ::star_engine::star_math::Vec3::new(impulse_x_, impulse_y_, impulse_z_),
            }));
        });
    });
}

/// Runs `then_` with what the engine needs to know about the mesh `node_`
/// shows, if it is a `Mesh`.
fn with_mesh_(
    node_: &crate::classes::Node::Ref,
    then_: impl FnOnce(::std::option::Option<::star_engine::script_nodes::ScriptMesh<'_>>),
) {
    let ::std::option::Option::Some(mesh_) = crate::AnyObject::Mesh(::std::rc::Rc::clone(node_)) else {
        return then_(::std::option::Option::None);
    };
    let look_ = crate::classes::Mesh::HasFields::fields(&*mesh_);
    look_.shape.read(|shape_| {
        look_.texture.read(|texture_| {
            then_(::std::option::Option::Some(::star_engine::script_nodes::ScriptMesh {
                shape: shape_,
                width: look_.width.get(),
                height: look_.height.get(),
                depth: look_.depth.get(),
                radius: look_.radius.get(),
                color: look_.color.read(|color_| engine_color_(&**color_)),
                metallic: look_.metallic.get(),
                roughness: look_.roughness.get(),
                texture: texture_,
                pixelated: look_.pixelated.get(),
            }));
        });
    });
}

/// Runs `then_` with what the engine needs to know about the light `node_`
/// shines, if it is a `Light`.
fn with_light_(
    node_: &crate::classes::Node::Ref,
    then_: impl FnOnce(::std::option::Option<::star_engine::script_nodes::ScriptLight<'_>>),
) {
    let ::std::option::Option::Some(light_) = crate::AnyObject::Light(::std::rc::Rc::clone(node_)) else {
        return then_(::std::option::Option::None);
    };
    let look_ = crate::classes::Light::HasFields::fields(&*light_);
    look_.kind.read(|kind_| {
        then_(::std::option::Option::Some(::star_engine::script_nodes::ScriptLight {
            kind: kind_,
            color: look_.color.read(|color_| engine_color_(&**color_)),
            intensity: look_.intensity.get(),
            range: look_.range.get(),
            angle: look_.angle.get(),
            blend: look_.blend.get(),
        }));
    });
}

/// Runs `then_` with what the engine needs to know about the light `node_`
/// shines on the sprites, if it is a `Light2D`.
fn with_light_2d_(
    node_: &crate::classes::Node::Ref,
    then_: impl FnOnce(::std::option::Option<::star_engine::script_nodes::ScriptLight<'_>>),
) {
    let ::std::option::Option::Some(light_) = crate::AnyObject::Light2D(::std::rc::Rc::clone(node_)) else {
        return then_(::std::option::Option::None);
    };
    let look_ = crate::classes::Light2D::HasFields::fields(&*light_);
    look_.kind.read(|kind_| {
        then_(::std::option::Option::Some(::star_engine::script_nodes::ScriptLight {
            kind: kind_,
            color: look_.color.read(|color_| engine_color_(&**color_)),
            intensity: look_.intensity.get(),
            range: look_.range.get(),
            angle: look_.angle.get(),
            blend: look_.blend.get(),
        }));
    });
}

/// Tells the nodes with a body of the game's tree what the simulation did
/// in the last frame: where they are, how fast they move and what they
/// touch. `known_` and `queue_` are reused, left empty.
fn sync_bodies_(
    world_: &::star_engine::star_ecs::World,
    mirror_: &mut ::star_engine::script_nodes::ScriptNodes,
    root_: &crate::classes::Node::Ref,
    known_: &mut ::std::collections::HashMap<u64, crate::classes::Node::Ref>,
    queue_: &mut ::std::vec::Vec<crate::classes::Node::Ref>,
) {
    mirror_.read_bodies(world_);
    let mut next_ = ::std::option::Option::Some(::std::rc::Rc::clone(root_));
    while let ::std::option::Option::Some(node_) = next_ {
        let fields_ = crate::classes::Node::HasFields::fields(&*node_);
        queue_.extend(fields_.children_.borrow().iter().cloned());
        known_.insert(fields_.id_, ::std::rc::Rc::clone(&node_));
        next_ = queue_.pop();
    }
    for (id_, node_) in known_.iter() {
        let fields_ = crate::classes::Node::HasFields::fields(&**node_);
        let ::std::option::Option::Some(body_) = fields_.body.get() else {
            continue;
        };
        let ::std::option::Option::Some(state_) = mirror_.body_state(world_, *id_) else {
            continue;
        };
        fields_.x.set(state_.position.x);
        fields_.y.set(state_.position.y);
        fields_.rotation.set(state_.angles.z);
        if let (true, ::std::option::Option::Some(node_3d_)) = (state_.is_3d, crate::AnyObject::Node3D(::std::rc::Rc::clone(node_))) {
            let depth_ = crate::classes::Node3D::HasFields::fields(&*node_3d_);
            depth_.z.set(state_.position.z);
            depth_.pitch.set(state_.angles.x);
            depth_.yaw.set(state_.angles.y);
        }
        let body_ = crate::classes::Body::HasFields::fields(&*body_);
        body_.velocity_x.set(state_.velocity.x);
        body_.velocity_y.set(state_.velocity.y);
        body_.velocity_z.set(state_.velocity.z);
        let contacts_ = [
            (&body_.touching_, state_.touching),
            (&body_.just_touched_, state_.just_touched),
            (&body_.just_separated_, state_.just_separated),
        ];
        for (list_, ids_) in contacts_ {
            let mut list_ = list_.borrow_mut();
            list_.clear();
            list_.extend(ids_.iter().filter_map(|other_| known_.get(other_)).map(::std::rc::Rc::downgrade));
        }
    }
    known_.clear();
}

/// The engine's value of a script `Color`.
fn engine_color_(color_: &dyn crate::classes::Color::Object) -> ::star_engine::star_render::Color {
    let channels_ = crate::classes::Color::HasFields::fields(color_);
    ::star_engine::star_render::Color::rgba(channels_.r.get(), channels_.g.get(), channels_.b.get(), channels_.a.get())
}

/// What the engine needs to know about the game's camera.
fn script_camera_(camera_: &dyn crate::classes::Camera2D::Object) -> ::star_engine::script_nodes::ScriptCamera {
    let node_ = crate::classes::Node::HasFields::fields(camera_);
    ::star_engine::script_nodes::ScriptCamera {
        id: node_.id_,
        position: ::star_engine::star_math::Vec2::new(node_.x.get(), node_.y.get()),
        view_height: crate::classes::Camera2D::HasFields::fields(camera_).view_height.get(),
    }
}

/// What the engine needs to know about the game's 3D camera.
fn script_camera_3d_(camera_: &dyn crate::classes::Camera3D::Object) -> ::star_engine::script_nodes::ScriptCamera3D {
    let node_ = crate::classes::Node::HasFields::fields(camera_);
    let depth_ = crate::classes::Node3D::HasFields::fields(camera_);
    ::star_engine::script_nodes::ScriptCamera3D {
        id: node_.id_,
        position: ::star_engine::star_math::Vec3::new(node_.x.get(), node_.y.get(), depth_.z.get()),
        angles: ::star_engine::star_math::Vec3::new(depth_.pitch.get(), depth_.yaw.get(), node_.rotation.get()),
        field_of_view: crate::classes::Camera3D::HasFields::fields(camera_).fov.get(),
    }
}";

const READ_SCENE: &str = "/// Reads a scene of the game's asset folder for `Scene.load`. Parts of a
/// scene that scripts have no field for yet (a sprite's region and anchor,
/// the divisions of a sphere) are left out.
fn read_scene_(
    folder_: &::star_engine::star_assets::AssetFolder,
    path_: &str,
) -> ::std::result::Result<::std::vec::Vec<::star_script_runtime::scenes::SceneNode>, ::std::string::String> {
    let outline_ = ::star_engine::star_assets::AssetPath::new(path_)
        .and_then(|path_| folder_.read_text(&path_))
        .and_then(|text_| ::star_engine::star_scene::SceneOutline::from_text(&text_))
        .map_err(|error_| error_.to_string())?;
    let nodes_ = outline_.into_nodes().into_iter().map(|node_| {
        ::star_script_runtime::scenes::SceneNode {
            name: node_.name,
            class: node_.class,
            tags: node_.tags,
            parent: node_.parent,
            transform: node_.transform.map(|place_| ::star_script_runtime::scenes::SceneTransform {
                x: place_.position.x,
                y: place_.position.y,
                rotation: place_.rotation,
                scale_x: place_.scale.x,
                scale_y: place_.scale.y,
            }),
            sprite: node_.sprite.map(|image_| ::star_script_runtime::scenes::SceneSprite {
                texture: image_.texture.as_str().to_owned(),
                width: image_.size.x,
                height: image_.size.y,
                layer: image_.layer,
                pixelated: image_.filter == ::star_engine::star_render::TextureFilter::Pixelated,
                color: [image_.color.r, image_.color.g, image_.color.b, image_.color.a],
            }),
            body: match (node_.transform_3d.is_some(), node_.body_3d, node_.body) {
                (true, ::std::option::Option::Some(settings_), _) => ::std::option::Option::Some(scene_body_(&::star_engine::script_nodes::ScriptBody::from_settings_3d(&settings_))),
                (_, _, ::std::option::Option::Some(settings_)) => ::std::option::Option::Some(scene_body_(&::star_engine::script_nodes::ScriptBody::from_settings(&settings_))),
                _ => ::std::option::Option::None,
            },
            transform_3d: node_.transform_3d.map(|place_| {
                let angles_ = place_.rotation.to_euler();
                ::star_script_runtime::scenes::SceneTransform3D {
                    x: place_.position.x,
                    y: place_.position.y,
                    z: place_.position.z,
                    pitch: angles_.x,
                    yaw: angles_.y,
                    roll: angles_.z,
                    scale_x: place_.scale.x,
                    scale_y: place_.scale.y,
                    scale_z: place_.scale.z,
                }
            }),
            mesh: node_.mesh.as_ref().map(|outline_| {
                let mesh_ = ::star_engine::script_nodes::ScriptMesh::from_outline(outline_);
                ::star_script_runtime::scenes::SceneMesh {
                    shape: ::std::string::String::from(mesh_.shape),
                    width: mesh_.width,
                    height: mesh_.height,
                    depth: mesh_.depth,
                    radius: mesh_.radius,
                    color: [mesh_.color.r, mesh_.color.g, mesh_.color.b, mesh_.color.a],
                    metallic: mesh_.metallic,
                    roughness: mesh_.roughness,
                    texture: ::std::string::String::from(mesh_.texture),
                    pixelated: mesh_.pixelated,
                }
            }),
            fields: node_.fields.into_iter().map(|(name_, value_)| (name_, match value_ {
                ::star_engine::star_scene::FieldValue::Bool(truth_) => ::star_script_runtime::scene_fields::SceneValue::Bool(truth_),
                ::star_engine::star_scene::FieldValue::Int(number_) => ::star_script_runtime::scene_fields::SceneValue::Int(number_),
                ::star_engine::star_scene::FieldValue::Float(number_) => ::star_script_runtime::scene_fields::SceneValue::Float(number_),
                ::star_engine::star_scene::FieldValue::Text(text_) => ::star_script_runtime::scene_fields::SceneValue::Text(text_),
                ::star_engine::star_scene::FieldValue::Color(color_) => ::star_script_runtime::scene_fields::SceneValue::Color(color_),
            })).collect(),
            camera: node_.camera.map(|view_| ::star_script_runtime::scenes::SceneCamera {
                view_height: view_.view_height,
                main: view_.main,
            }),
            camera_3d: node_.camera_3d.map(|view_| ::star_script_runtime::scenes::SceneCamera3D {
                fov: view_.fov,
                main: view_.main,
            }),
            light_2d: node_.light_2d.map(|shone_| {
                let light_ = ::star_engine::script_nodes::ScriptLight::from_light_2d(&shone_);
                ::star_script_runtime::scenes::SceneLight {
                    kind: ::std::string::String::from(light_.kind),
                    color: [light_.color.r, light_.color.g, light_.color.b, light_.color.a],
                    intensity: light_.intensity,
                    range: light_.range,
                    angle: light_.angle,
                    blend: light_.blend,
                }
            }),
            light: node_.light.map(|shone_| {
                let light_ = ::star_engine::script_nodes::ScriptLight::from_light(&shone_);
                ::star_script_runtime::scenes::SceneLight {
                    kind: ::std::string::String::from(light_.kind),
                    color: [light_.color.r, light_.color.g, light_.color.b, light_.color.a],
                    intensity: light_.intensity,
                    range: light_.range,
                    angle: light_.angle,
                    blend: light_.blend,
                }
            }),
        }
    });
    ::std::result::Result::Ok(nodes_.collect())
}

/// The body of a scene node, with the words of StarScript's `Body`.
fn scene_body_(body_: &::star_engine::script_nodes::ScriptBody<'_>) -> ::star_script_runtime::scenes::SceneBody {
    ::star_script_runtime::scenes::SceneBody {
        kind: ::std::string::String::from(body_.kind),
        shape: ::std::string::String::from(body_.shape),
        radius: body_.radius,
        width: body_.width,
        height: body_.height,
        depth: body_.depth,
        gravity_scale: body_.gravity_scale,
        fixed_rotation: body_.fixed_rotation,
        friction: body_.friction,
        bounciness: body_.bounciness,
        density: body_.density,
        sensor: body_.sensor,
    }
}";

const ENGINE_AUDIO: &str = "/// Loads and plays the sounds of StarScript's `Sound` with the engine.
struct EngineAudio_ {
    folder: ::star_engine::star_assets::AssetFolder,
    output: ::std::rc::Rc<::std::cell::RefCell<::star_engine::star_audio::Audio>>,
    /// Every sound loaded, by its number.
    sounds: ::std::vec::Vec<::std::sync::Arc<::star_engine::star_core::Sound>>,
}

impl ::star_script_runtime::audio::AudioBackend for EngineAudio_ {
    fn load(&mut self, path_: &str) -> ::std::result::Result<::star_script_runtime::audio::SoundId, ::std::string::String> {
        let sound_ = ::star_engine::star_assets::AssetPath::new(path_)
            .and_then(|path_| self.folder.load_sound(&path_))
            .map_err(|error_| error_.to_string())?;
        self.sounds.push(::std::sync::Arc::new(sound_));
        ::std::result::Result::Ok(::star_script_runtime::audio::SoundId(self.sounds.len() - 1))
    }

    fn play(&mut self, sound_: ::star_script_runtime::audio::SoundId, volume_: f32, looping_: bool) -> ::std::result::Result<::star_script_runtime::audio::VoiceId, ::std::string::String> {
        let sound_ = self.sounds.get(sound_.0).ok_or_else(|| ::std::string::String::from(\"the sound was never loaded\"))?;
        let settings_ = ::star_engine::star_audio::PlaySettings { volume: volume_, looping: looping_ };
        self.output.borrow().play(::std::sync::Arc::clone(sound_), settings_)
            .map(|voice_| ::star_script_runtime::audio::VoiceId(voice_.to_bits()))
            .map_err(|error_| error_.to_string())
    }

    fn stop(&mut self, voice_: ::star_script_runtime::audio::VoiceId) {
        self.output.borrow().stop(::star_engine::star_audio::VoiceId::from_bits(voice_.0));
    }

    fn is_playing(&self, voice_: ::star_script_runtime::audio::VoiceId) -> bool {
        self.output.borrow().is_playing(::star_engine::star_audio::VoiceId::from_bits(voice_.0))
    }

    fn volume(&self) -> f32 {
        self.output.borrow().volume()
    }

    fn set_volume(&mut self, volume_: f32) {
        self.output.borrow().set_volume(volume_);
    }
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
