//! Modules of the engine classes that scripts can extend, and the code that
//! runs a game in the engine.
//!
//! Their methods have no StarScript source, so their modules are written
//! here, with the same shape as the modules of script classes.

use super::conversions::RUNTIME;
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
        pub camera: ::star_script_runtime::Field<::std::option::Option<crate::classes::Camera::Ref>>,
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

    /// Prepares an object of this class: `make_` builds it from its fields.
    pub fn init_<O: Object + 'static>(make_: impl FnOnce(Fields) -> ::std::rc::Rc<O>) -> ::std::rc::Rc<O> {
        make_(Fields {
            background: ::star_script_runtime::Field::new(crate::classes::Color::new(0.0, 0.0, 0.0, 1.0)),
            camera: ::star_script_runtime::Field::new(::std::option::Option::None),
            quit_requested_: ::star_script_runtime::Field::new(false),
            root_: crate::classes::Node::new(::std::string::String::from(\"Game\")),
        })
    }

    /// Creates an object of this class.
    pub fn new() -> Ref {
        init_(|fields0_| ::std::rc::Rc::new(Instance { Game: fields0_ }))
    }
}";

/// The modules of the engine classes `Color`, `Node`, `Sprite`, `Camera`
/// and `Scene`.
pub(super) fn node_modules(out: &mut Writer) {
    let modules = [
        COLOR_MODULE,
        NODE_MODULE,
        SPRITE_MODULE,
        CAMERA_MODULE,
        SCENE_MODULE,
    ];
    for (index, module) in modules.iter().enumerate() {
        if index > 0 {
            out.line("");
        }
        for line in module.lines() {
            out.line(line);
        }
    }
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
    pub fn process_tree_(root: &Ref, delta: f32, nodes_: &mut ::std::vec::Vec<Ref>) {
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
        pub layer: ::star_script_runtime::Field<i64>,
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

const CAMERA_MODULE: &str = "/// `class Camera`, provided by the engine.
pub mod Camera {
    /// A reference to an object of this class or of a subclass.
    pub type Ref = ::std::rc::Rc<dyn Object>;

    /// The fields of a camera: how much of the world it shows.
    pub struct Fields {
        pub view_height: ::star_script_runtime::Field<f32>,
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
        pub Camera: Fields,
    }

    impl crate::classes::Node::HasFields for Instance {
        fn fields(&self) -> &crate::classes::Node::Fields { &self.Node }
    }

    impl HasFields for Instance {
        fn fields(&self) -> &Fields { &self.Camera }
    }

    impl crate::AnyObject for Instance {
        fn Node(self: ::std::rc::Rc<Self>) -> ::std::option::Option<crate::classes::Node::Ref> { ::std::option::Option::Some(self) }
        fn Camera(self: ::std::rc::Rc<Self>) -> ::std::option::Option<Ref> { ::std::option::Option::Some(self) }
    }

    impl crate::classes::Node::Object for Instance {
        fn start(self: ::std::rc::Rc<Self>) { crate::classes::Node::Methods::start(&(self as crate::classes::Node::Ref)) }
        fn update(self: ::std::rc::Rc<Self>, delta: f32) { crate::classes::Node::Methods::update(&(self as crate::classes::Node::Ref), delta) }
        fn add_child(self: ::std::rc::Rc<Self>, child: crate::classes::Node::Ref) { crate::classes::Node::Methods::add_child(&(self as crate::classes::Node::Ref), child) }
        fn remove(self: ::std::rc::Rc<Self>) { crate::classes::Node::Methods::remove(&(self as crate::classes::Node::Ref)) }
        fn parent(self: ::std::rc::Rc<Self>) -> ::std::option::Option<crate::classes::Node::Ref> { crate::classes::Node::Methods::parent(&(self as crate::classes::Node::Ref)) }
        fn children(self: ::std::rc::Rc<Self>) -> ::star_script_runtime::List<crate::classes::Node::Ref> { crate::classes::Node::Methods::children(&(self as crate::classes::Node::Ref)) }
        fn find_child(self: ::std::rc::Rc<Self>, name: ::std::string::String) -> ::std::option::Option<crate::classes::Node::Ref> { crate::classes::Node::Methods::find_child(&(self as crate::classes::Node::Ref), name) }
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
        };
        crate::classes::Node::init_(name, move |base0_| make_(base0_, fields_))
    }

    /// Creates an object of this class.
    pub fn new(name: ::std::string::String) -> Ref {
        init_(name, |fields0_, fields1_| ::std::rc::Rc::new(Instance { Node: fields0_, Camera: fields1_ }))
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
        let object_: crate::classes::Node::Ref = match (node_.class.as_deref(), &node_.sprite) {
            (::std::option::Option::None | ::std::option::Option::Some(\"Sprite\"), ::std::option::Option::Some(image_)) => {
                crate::classes::Sprite::new(name_, image_.texture.clone())
            }
            (::std::option::Option::None | ::std::option::Option::Some(\"Node\"), _) => crate::classes::Node::new(name_),
            (::std::option::Option::Some(\"Camera\"), _) => crate::classes::Camera::new(name_),
            (::std::option::Option::Some(\"Sprite\"), ::std::option::Option::None) => {
                return ::std::result::Result::Err(::std::string::String::from(\"a `Sprite` needs a `[node.sprite]` with its image\"));
            }
            (::std::option::Option::Some(class_), _) => crate::new_scene_object_(class_)?,
        };
        let fields_ = crate::classes::Node::HasFields::fields(&*object_);
        fields_.name.set(node_.name.clone());
        if let ::std::option::Option::Some(place_) = &node_.transform {
            fields_.x.set(place_.x);
            fields_.y.set(place_.y);
            fields_.rotation.set(place_.rotation);
            fields_.scale_x.set(place_.scale_x);
            fields_.scale_y.set(place_.scale_y);
        }
        if let ::std::option::Option::Some(image_) = &node_.sprite {
            let class_ = node_.class.as_deref().unwrap_or(\"Node\");
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
        ::std::result::Result::Ok(object_)
    }
}";

/// `fn main` that runs the game `class_path` in a window of the engine, with
/// the input `actions` of the project.
pub(super) fn game_entry_point(
    out: &mut Writer,
    class_path: &str,
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
    {RUNTIME}::scenes::set_loader(move |path_| read_scene_(&folder_, path_));
    // Created after the loader is set, so its `init` can load scenes too.
    let game_ = EngineGame_ {{
        game: {class_path}::new(),
        nodes: ::std::vec::Vec::new(),
        pending: ::std::vec::Vec::new(),
        mirror: ::star_engine::script_nodes::ScriptNodes::new(),
    }};
    app_.run_game(game_)
}}

/// Connects the StarScript game with the engine's game loop.
struct EngineGame_ {{
    game: crate::classes::Game::Ref,
    /// Reused list of the nodes whose code runs in a frame.
    nodes: ::std::vec::Vec<crate::classes::Node::Ref>,
    /// Reused list of the nodes still to mirror, with their parent's number.
    pending: ::std::vec::Vec<(crate::classes::Node::Ref, ::std::option::Option<u64>)>,
    /// The node of the engine's world that mirrors each node of the game.
    mirror: ::star_engine::script_nodes::ScriptNodes,
}}

impl ::star_engine::game::Game for EngineGame_ {{
    fn start(
        &mut self,
        _context: &mut ::star_engine::game::GameContext<'_>,
    ) -> ::star_engine::star_core::EngineResult<()> {{
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
        crate::classes::Game::Object::update(::std::rc::Rc::clone(&self.game), delta_);
        let root_ = ::std::rc::Rc::clone(&crate::classes::Game::HasFields::fields(&*self.game).root_);
        crate::classes::Node::process_tree_(&root_, delta_, &mut self.nodes);
        mirror_nodes_(world, &mut self.mirror, &root_, &mut self.pending);
    }}

    fn draw(&self, world: &::star_engine::star_ecs::World, frame: &mut ::star_engine::star_render::Frame) {{
        let fields_ = crate::classes::Game::HasFields::fields(&*self.game);
        frame.set_clear_color(fields_.background.read(|color_| engine_color_(&**color_)));
        frame.set_camera(self.view_(world));
    }}

    fn is_finished(&self) -> bool {{
        crate::classes::Game::HasFields::fields(&*self.game).{QUIT_FIELD}.get()
    }}
}}

impl EngineGame_ {{
    /// The engine's camera that shows the game from its `camera`.
    fn view_(&self, world: &::star_engine::star_ecs::World) -> ::star_engine::star_render::Camera {{
        crate::classes::Game::HasFields::fields(&*self.game).camera.read(|camera_| match camera_ {{
            ::std::option::Option::Some(camera_) => self.mirror.camera(world, &script_camera_(&**camera_)),
            ::std::option::Option::None => ::star_engine::star_render::Camera::default(),
        }})
    }}
}}

/// Copies where the player points, in world units, where `Input` reads it.
/// The camera is the one the last frame was drawn from: what the player saw.
fn share_pointer_(
    input_: &::star_engine::star_input::Input,
    view_: &::star_engine::star_render::Camera,
    window_: ::star_engine::star_platform::WindowSize,
) {{
    let position_ = input_.pointer().and_then(|pixel_| view_.screen_to_world_2d(pixel_, window_));
    {RUNTIME}::input::set_pointer(position_.map(|point_| (point_.x, point_.y)));
}}

/// Copies the state of each action where `Input` reads it.
fn share_actions_(input_: &::star_engine::star_input::Input) {{
    let actions_ = input_.actions();
    for (index_, name_) in ACTION_NAMES_.iter().enumerate() {{
        let state_ = {RUNTIME}::input::ActionState {{
            is_down: actions_.is_down(name_),
            just_pressed: actions_.just_pressed(name_),
            just_released: actions_.just_released(name_),
            strength: input_.action_strength(name_),
        }};
        {RUNTIME}::input::set_action_state(index_, state_);
    }}
}}"
    );
    let support = [MIRROR_NODES, READ_SCENE];
    for line in main.lines().chain(
        support
            .iter()
            .flat_map(|code| [""].into_iter().chain(code.lines())),
    ) {
        out.line(line);
    }
}

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
        let sprite_ = crate::AnyObject::Sprite(::std::rc::Rc::clone(&node_));
        fields_.name.read(|name_| {
            match &sprite_ {
                ::std::option::Option::Some(sprite_) => {
                    let look_ = crate::classes::Sprite::HasFields::fields(&**sprite_);
                    look_.texture.read(|texture_| {
                        let layer_ = look_.layer.get().clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32;
                        let shown_ = ::star_engine::script_nodes::ScriptSprite {
                            texture: texture_,
                            size: ::star_engine::star_math::Vec2::new(look_.width.get(), look_.height.get()),
                            layer: layer_,
                            pixelated: look_.pixelated.get(),
                            color: look_.color.read(|color_| engine_color_(&**color_)),
                        };
                        mirror_.mirror(world_, &script_node_(fields_, parent_, name_, transform_, ::std::option::Option::Some(shown_)));
                    });
                }
                ::std::option::Option::None => mirror_.mirror(world_, &script_node_(fields_, parent_, name_, transform_, ::std::option::Option::None)),
            }
        });
        for child_ in fields_.children_.borrow().iter().rev() {
            pending_.push((::std::rc::Rc::clone(child_), ::std::option::Option::Some(fields_.id_)));
        }
    }
    mirror_.finish_frame(world_);
}

/// The engine's value of a script `Color`.
fn engine_color_(color_: &dyn crate::classes::Color::Object) -> ::star_engine::star_render::Color {
    let channels_ = crate::classes::Color::HasFields::fields(color_);
    ::star_engine::star_render::Color::rgba(channels_.r.get(), channels_.g.get(), channels_.b.get(), channels_.a.get())
}

/// What the engine needs to know about the game's camera.
fn script_camera_(camera_: &dyn crate::classes::Camera::Object) -> ::star_engine::script_nodes::ScriptCamera {
    let node_ = crate::classes::Node::HasFields::fields(camera_);
    ::star_engine::script_nodes::ScriptCamera {
        id: node_.id_,
        position: ::star_engine::star_math::Vec2::new(node_.x.get(), node_.y.get()),
        view_height: crate::classes::Camera::HasFields::fields(camera_).view_height.get(),
    }
}

/// What the engine needs to know about one node.
fn script_node_<'a>(
    fields_: &crate::classes::Node::Fields,
    parent_: ::std::option::Option<u64>,
    name_: &'a str,
    transform_: ::star_engine::star_scene::Transform2D,
    sprite_: ::std::option::Option<::star_engine::script_nodes::ScriptSprite<'a>>,
) -> ::star_engine::script_nodes::ScriptNode<'a> {
    ::star_engine::script_nodes::ScriptNode {
        id: fields_.id_,
        parent: parent_,
        name: name_,
        transform: transform_,
        sprite: sprite_,
    }
}";

const READ_SCENE: &str = "/// Reads a scene of the game's asset folder for `Scene.load`. Parts of a
/// scene that scripts have no field for yet (a sprite's region and anchor)
/// are left out.
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
                layer: i64::from(image_.layer),
                pixelated: image_.filter == ::star_engine::star_render::TextureFilter::Pixelated,
                color: [image_.color.r, image_.color.g, image_.color.b, image_.color.a],
            }),
        }
    });
    ::std::result::Result::Ok(nodes_.collect())
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
