//! Modules of the engine's node classes written from a description:
//! `Sprite`, `Camera2D`, `Light2D` and `Node3D`, which extend `Node`; and
//! `Mesh`, `Light` and `Camera3D`, which extend `Node3D`.
//!
//! They all have the same shape, so they are written from a description of
//! each class: the type of its fields (in `star_script_runtime`), the
//! arguments of its `init`, and its own methods.

use super::writer::Writer;

/// The path of the modules of the engine classes.
const CLASSES: &str = "crate::classes";

/// A method of an engine node class, as Rust: the name, the parameters
/// after the receiver, the arguments that pass them on and the return type
/// (with its arrow, or empty).
struct Method {
    name: &'static str,
    parameters: &'static str,
    arguments: &'static str,
    returns: &'static str,
}

/// The methods of `Node`, which every node class implements.
const NODE_METHODS: [Method; 18] = [
    Method {
        name: "start",
        parameters: "",
        arguments: "",
        returns: "",
    },
    Method {
        name: "update",
        parameters: "delta: f32",
        arguments: "delta",
        returns: "",
    },
    Method {
        name: "add_child",
        parameters: "child: crate::classes::Node::Ref",
        arguments: "child",
        returns: "",
    },
    Method {
        name: "remove",
        parameters: "",
        arguments: "",
        returns: "",
    },
    Method {
        name: "parent",
        parameters: "",
        arguments: "",
        returns: " -> ::std::option::Option<crate::classes::Node::Ref>",
    },
    Method {
        name: "children",
        parameters: "",
        arguments: "",
        returns: " -> ::star_script_runtime::List<crate::classes::Node::Ref>",
    },
    Method {
        name: "find_child",
        parameters: "name: ::std::string::String",
        arguments: "name",
        returns: " -> ::std::option::Option<crate::classes::Node::Ref>",
    },
    Method {
        name: "has_tag",
        parameters: "tag: ::std::string::String",
        arguments: "tag",
        returns: " -> bool",
    },
    Method {
        name: "find_by_name",
        parameters: "name: ::std::string::String",
        arguments: "name",
        returns: " -> ::std::option::Option<crate::classes::Node::Ref>",
    },
    Method {
        name: "find_by_tag",
        parameters: "tag: ::std::string::String",
        arguments: "tag",
        returns: " -> ::star_script_runtime::List<crate::classes::Node::Ref>",
    },
    Method {
        name: "global_x",
        parameters: "",
        arguments: "",
        returns: " -> f32",
    },
    Method {
        name: "global_y",
        parameters: "",
        arguments: "",
        returns: " -> f32",
    },
    Method {
        name: "global_rotation",
        parameters: "",
        arguments: "",
        returns: " -> f32",
    },
    Method {
        name: "set_global_position",
        parameters: "x: f32, y: f32",
        arguments: "x, y",
        returns: "",
    },
    Method {
        name: "set_global_rotation",
        parameters: "angle: f32",
        arguments: "angle",
        returns: "",
    },
    Method {
        name: "translate",
        parameters: "x: f32, y: f32",
        arguments: "x, y",
        returns: "",
    },
    Method {
        name: "translate_global",
        parameters: "x: f32, y: f32",
        arguments: "x, y",
        returns: "",
    },
    Method {
        name: "rotate",
        parameters: "angle: f32",
        arguments: "angle",
        returns: "",
    },
];

/// The methods `Node3D` adds.
const NODE_3D_METHODS: [Method; 5] = [
    Method {
        name: "look_at",
        parameters: "x: f32, y: f32, z: f32",
        arguments: "x, y, z",
        returns: "",
    },
    Method {
        name: "global_z",
        parameters: "",
        arguments: "",
        returns: " -> f32",
    },
    Method {
        name: "set_global_position_3d",
        parameters: "x: f32, y: f32, z: f32",
        arguments: "x, y, z",
        returns: "",
    },
    Method {
        name: "translate_3d",
        parameters: "x: f32, y: f32, z: f32",
        arguments: "x, y, z",
        returns: "",
    },
    Method {
        name: "translate_global_3d",
        parameters: "x: f32, y: f32, z: f32",
        arguments: "x, y, z",
        returns: "",
    },
];

/// `func global_z`: how deep the node is in the world.
const GLOBAL_Z_BODY: &str = "let node_ = crate::AnyObject::Node(::std::rc::Rc::clone(this_));
node_.map_or(0.0, |node_| ::star_script_runtime::nodes::global_position::<crate::classes::Host_>(&node_)[2])";

/// `func set_global_position_3d`: places the node at a point of the world.
const SET_GLOBAL_POSITION_3D_BODY: &str =
    "let node_ = crate::AnyObject::Node(::std::rc::Rc::clone(this_));
if let ::std::option::Option::Some(node_) = node_ {
    ::star_script_runtime::nodes::set_global_position_3d::<crate::classes::Host_>(&node_, [x, y, z]);
}";

/// `func translate_3d`: moves the node in its parent's space.
const TRANSLATE_3D_BODY: &str = "let node_ = crate::AnyObject::Node(::std::rc::Rc::clone(this_));
if let ::std::option::Option::Some(node_) = node_ {
    ::star_script_runtime::nodes::translate_3d::<crate::classes::Host_>(&node_, x, y, z);
}";

/// `func translate_global_3d`: moves the node in the world.
const TRANSLATE_GLOBAL_3D_BODY: &str =
    "let node_ = crate::AnyObject::Node(::std::rc::Rc::clone(this_));
if let ::std::option::Option::Some(node_) = node_ {
    ::star_script_runtime::nodes::translate_global_3d::<crate::classes::Host_>(&node_, x, y, z);
}";

/// `func look_at`: turns the node to look at a point of its parent's space.
const LOOK_AT_BODY: &str = "let node_ = crate::AnyObject::Node(::std::rc::Rc::clone(this_));
if let ::std::option::Option::Some(node_) = node_ {
    ::star_script_runtime::nodes::look_at::<crate::classes::Host_>(&node_, x, y, z);
}";

/// The methods of `Sprite`: playing the clips of its `.frames` file.
const SPRITE_METHODS: [Method; 4] = [
    Method {
        name: "play",
        parameters: "name: ::std::string::String",
        arguments: "name",
        returns: "",
    },
    Method {
        name: "stop",
        parameters: "",
        arguments: "",
        returns: "",
    },
    Method {
        name: "is_playing",
        parameters: "",
        arguments: "",
        returns: " -> bool",
    },
    Method {
        name: "animation",
        parameters: "",
        arguments: "",
        returns: " -> ::std::string::String",
    },
];

/// `func play`: plays a clip of the sprite's `frames`.
const PLAY_BODY: &str = "let node_ = crate::AnyObject::Node(::std::rc::Rc::clone(this_));
if let ::std::option::Option::Some(node_) = node_ {
    ::star_script_runtime::animations::play::<crate::classes::Host_>(&node_, &name);
}";

/// `func stop`: stops the sprite's clip on its frame.
const STOP_BODY: &str = "let node_ = crate::AnyObject::Node(::std::rc::Rc::clone(this_));
if let ::std::option::Option::Some(node_) = node_ {
    ::star_script_runtime::animations::stop::<crate::classes::Host_>(&node_);
}";

/// `func is_playing`: whether the sprite's clip plays.
const IS_PLAYING_BODY: &str = "let node_ = crate::AnyObject::Node(::std::rc::Rc::clone(this_));
node_.is_some_and(|node_| ::star_script_runtime::animations::is_playing::<crate::classes::Host_>(&node_))";

/// `func animation`: the clip the sprite plays, or played last.
const ANIMATION_BODY: &str = "let node_ = crate::AnyObject::Node(::std::rc::Rc::clone(this_));
node_.map(|node_| ::star_script_runtime::animations::animation::<crate::classes::Host_>(&node_)).unwrap_or_default()";

/// An engine node class written from a description. Its own fields are a
/// type of `star_script_runtime`, shared with every way of running
/// StarScript.
struct NodeClass {
    name: &'static str,
    /// What the fields are, for the documentation of `Fields`.
    about: &'static str,
    /// The classes it extends, from `Node` down.
    ancestors: &'static [&'static str],
    /// The type of its own fields.
    fields: &'static str,
    /// The Rust expression that builds its own fields, from the arguments
    /// of its `init`.
    new_fields: &'static str,
    /// The arguments of its `init` after the name, which give their own
    /// fields their first values.
    parameters: &'static [(&'static str, &'static str)],
    methods: &'static [Method],
    /// The code of each of its methods, in the same order.
    bodies: &'static [&'static str],
}

const STRING: &str = "::std::string::String";

const SPRITE: NodeClass = NodeClass {
    name: "Sprite",
    about: "the image it shows and how",
    ancestors: &["Node"],
    fields: "::star_script_runtime::node_classes::SpriteFields<crate::classes::Host_>",
    new_fields: "Fields::new(texture)",
    parameters: &[("texture", STRING)],
    methods: &SPRITE_METHODS,
    bodies: &[PLAY_BODY, STOP_BODY, IS_PLAYING_BODY, ANIMATION_BODY],
};

const CAMERA_2D: NodeClass = NodeClass {
    name: "Camera2D",
    about: "how much of the world it shows, and whether it becomes the game's camera when it enters the game's tree",
    ancestors: &["Node"],
    fields: "::star_script_runtime::node_classes::Camera2dFields",
    new_fields: "Fields::new()",
    parameters: &[],
    methods: &[],
    bodies: &[],
};

const LIGHT_2D: NodeClass = NodeClass {
    name: "Light2D",
    about: "the kind of light it shines on the sprites, its color, how bright it is, how far it reaches and the cone of spots",
    ancestors: &["Node"],
    fields: "::star_script_runtime::node_classes::LightFields<crate::classes::Host_>",
    new_fields: "Fields::new_2d(kind)",
    parameters: &[("kind", STRING)],
    methods: &[],
    bodies: &[],
};

const NODE_3D: NodeClass = NodeClass {
    name: "Node3D",
    about: "its depth, its pitch and yaw (its roll is `rotation`) and its depth scale",
    ancestors: &["Node"],
    fields: "::star_script_runtime::nodes::Node3dFields",
    new_fields: "Fields::new()",
    parameters: &[],
    methods: &NODE_3D_METHODS,
    bodies: &[
        LOOK_AT_BODY,
        GLOBAL_Z_BODY,
        SET_GLOBAL_POSITION_3D_BODY,
        TRANSLATE_3D_BODY,
        TRANSLATE_GLOBAL_3D_BODY,
    ],
};

const MESH: NodeClass = NodeClass {
    name: "Mesh",
    about: "the shape it shows, its sizes and its surface",
    ancestors: &["Node", "Node3D"],
    fields: "::star_script_runtime::node_classes::MeshFields<crate::classes::Host_>",
    new_fields: "Fields::new(shape)",
    parameters: &[("shape", STRING)],
    methods: &[],
    bodies: &[],
};

const LIGHT: NodeClass = NodeClass {
    name: "Light",
    about: "the kind of light it shines, its color, how bright it is, how far it reaches and the cone of spots",
    ancestors: &["Node", "Node3D"],
    fields: "::star_script_runtime::node_classes::LightFields<crate::classes::Host_>",
    new_fields: "Fields::new_3d(kind)",
    parameters: &[("kind", STRING)],
    methods: &[],
    bodies: &[],
};

const CAMERA_3D: NodeClass = NodeClass {
    name: "Camera3D",
    about: "how wide it sees, in radians, and whether it becomes the game's 3D camera when it enters the game's tree",
    ancestors: &["Node", "Node3D"],
    fields: "::star_script_runtime::node_classes::Camera3dFields",
    new_fields: "Fields::new()",
    parameters: &[],
    methods: &[],
    bodies: &[],
};

/// The modules of `Sprite`, `Camera2D`, `Light2D`, `Node3D`, `Mesh`,
/// `Light` and `Camera3D`.
pub(super) fn node_class_modules(out: &mut Writer) {
    for class in [SPRITE, CAMERA_2D, LIGHT_2D, NODE_3D, MESH, LIGHT, CAMERA_3D] {
        out.line("");
        module(out, &class);
    }
}

/// The methods each class of `ancestors` declares.
fn methods_of(class: &str) -> &'static [Method] {
    match class {
        "Node" => &NODE_METHODS,
        "Node3D" => &NODE_3D_METHODS,
        "Sprite" => &SPRITE_METHODS,
        _ => &[],
    }
}

fn module(out: &mut Writer, class: &NodeClass) {
    let name = class.name;
    let base = class.ancestors.last().copied().unwrap_or("Node");
    out.line(&format!("/// `class {name}`, provided by the engine."));
    out.open(&format!("pub mod {name} {{"));
    out.line("/// A reference to an object of this class or of a subclass.");
    out.line("pub type Ref = ::std::rc::Rc<dyn Object>;");
    out.line("");
    out.line(&format!("/// The fields of a {name}: {}.", class.about));
    out.line(&format!("pub type Fields = {};", class.fields));
    out.line("");
    out.line("/// Gives access to the fields of this class, in objects of any subclass.");
    out.open("pub trait HasFields {");
    out.line("fn fields(&self) -> &Fields;");
    out.close("}");
    out.line("");
    out.line("/// What objects of this class can do, beyond what their base can.");
    out.open(&format!(
        "pub trait Object: {CLASSES}::{base}::Object + HasFields {{"
    ));
    for method in class.methods {
        out.line(&format!("{};", signature(method)));
    }
    out.close("}");
    out.line("");
    methods(out, class);
    out.line("");
    instance(out, class);
    out.line("");
    constructor(out, class);
    out.close("}");
}

/// `fn name(self: Rc<Self>, parameters…) -> Type`
fn signature(method: &Method) -> String {
    let separator = if method.parameters.is_empty() {
        ""
    } else {
        ", "
    };
    format!(
        "fn {}(self: ::std::rc::Rc<Self>{separator}{}){}",
        method.name, method.parameters, method.returns
    )
}

fn methods(out: &mut Writer, class: &NodeClass) {
    out.line("/// The code of the methods of this class.");
    out.line("pub struct Methods;");
    if class.methods.is_empty() {
        return;
    }
    out.line("");
    out.open("impl Methods {");
    for (method, body) in class.methods.iter().zip(class.bodies) {
        let separator = if method.parameters.is_empty() {
            ""
        } else {
            ", "
        };
        out.line(&format!("/// `func {}`", method.name));
        out.open(&format!(
            "pub fn {}(this_: &Ref{separator}{}){} {{",
            method.name, method.parameters, method.returns
        ));
        for line in body.lines() {
            out.line(line);
        }
        out.close("}");
    }
    out.close("}");
}

/// The objects created as this class, with the fields of each class of its
/// hierarchy, and the methods of each one.
fn instance(out: &mut Writer, class: &NodeClass) {
    let hierarchy: Vec<&str> = class
        .ancestors
        .iter()
        .copied()
        .chain([class.name])
        .collect();
    out.line("/// An object created as this class (not as a subclass).");
    out.open("pub struct Instance {");
    for member in &hierarchy {
        out.line(&format!("pub {member}: {CLASSES}::{member}::Fields,"));
    }
    out.close("}");
    for member in &hierarchy {
        out.line("");
        out.open(&format!(
            "impl {CLASSES}::{member}::HasFields for Instance {{"
        ));
        out.line(&format!(
            "fn fields(&self) -> &{CLASSES}::{member}::Fields {{ &self.{member} }}"
        ));
        out.close("}");
    }
    out.line("");
    out.open("impl crate::AnyObject for Instance {");
    for member in &hierarchy {
        out.line(&format!(
            "fn {member}(self: ::std::rc::Rc<Self>) -> ::std::option::Option<{CLASSES}::{member}::Ref> {{ ::std::option::Option::Some(self) }}"
        ));
    }
    out.close("}");
    for member in &hierarchy {
        let own = if *member == class.name {
            class.methods
        } else {
            methods_of(member)
        };
        out.line("");
        out.open(&format!("impl {CLASSES}::{member}::Object for Instance {{"));
        for method in own {
            let separator = if method.arguments.is_empty() {
                ""
            } else {
                ", "
            };
            out.line(&format!(
                "{} {{ {CLASSES}::{member}::Methods::{}(&(self as {CLASSES}::{member}::Ref){separator}{}) }}",
                signature(method),
                method.name,
                method.arguments
            ));
        }
        out.close("}");
    }
}

/// `init_`, which builds the object from the fields of each class of its
/// hierarchy, and `new`.
fn constructor(out: &mut Writer, class: &NodeClass) {
    let mut parameters = String::new();
    let mut arguments = String::new();
    for (name, ty) in class.parameters {
        for piece in [", ", name, ": ", ty] {
            parameters.push_str(piece);
        }
        arguments.push_str(", ");
        arguments.push_str(name);
    }
    let base = class.ancestors.last().copied().unwrap_or("Node");
    let field_types: Vec<String> = class
        .ancestors
        .iter()
        .map(|ancestor| format!("{CLASSES}::{ancestor}::Fields"))
        .chain(["Fields".to_owned()])
        .collect();
    let bases: Vec<String> = (0..class.ancestors.len())
        .map(|index| format!("base{index}_"))
        .collect();
    out.line("/// Prepares an object of this class: `make_` builds it from the fields");
    out.line("/// of each class of its hierarchy, from `Node` down.");
    out.open(&format!(
        "pub fn init_<O: Object + 'static>(name: ::std::string::String{parameters}, make_: impl FnOnce({}) -> ::std::rc::Rc<O>) -> ::std::rc::Rc<O> {{",
        field_types.join(", ")
    ));
    out.line(&format!("let fields_ = {};", class.new_fields));
    out.line(&format!(
        "{CLASSES}::{base}::init_(name, move |{}| make_({}, fields_))",
        bases.join(", "),
        bases.join(", ")
    ));
    out.close("}");
    out.line("");
    let pieces: Vec<String> = (0..=class.ancestors.len())
        .map(|index| format!("fields{index}_"))
        .collect();
    let members: Vec<String> = class
        .ancestors
        .iter()
        .chain([&class.name])
        .zip(&pieces)
        .map(|(member, piece)| format!("{member}: {piece}"))
        .collect();
    out.line("/// Creates an object of this class.");
    out.open(&format!(
        "pub fn new(name: ::std::string::String{parameters}) -> Ref {{"
    ));
    out.line(&format!(
        "init_(name{arguments}, |{}| ::std::rc::Rc::new(Instance {{ {} }}))",
        pieces.join(", "),
        members.join(", ")
    ));
    out.close("}");
}
