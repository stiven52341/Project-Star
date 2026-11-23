//! Modules of the engine's 3D node classes: `Node3D`, which extends `Node`,
//! and `Mesh`, `Light` and `Camera3D`, which extend `Node3D`.
//!
//! They have the same shape as the modules of `Sprite` and `Camera`, so
//! they are written from a description of each class: its fields, the
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
const NODE_METHODS: [Method; 7] = [
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
];

/// The methods `Node3D` adds.
const NODE_3D_METHODS: [Method; 1] = [Method {
    name: "look_at",
    parameters: "x: f32, y: f32, z: f32",
    arguments: "x, y, z",
    returns: "",
}];

/// `func look_at`: turns the node so that its -z axis, where cameras and
/// lights look, points at a point of its parent's space. Its roll is kept.
const LOOK_AT_BODY: &str = "let place_ = crate::classes::Node::HasFields::fields(&**this_);
let depth_ = HasFields::fields(&**this_);
let (dx_, dy_, dz_) = (x - place_.x.get(), y - place_.y.get(), z - depth_.z.get());
let length_ = (dx_ * dx_ + dy_ * dy_ + dz_ * dz_).sqrt();
if length_ > 0.0 && length_.is_finite() {
    depth_.pitch.set((dy_ / length_).clamp(-1.0, 1.0).asin());
    depth_.yaw.set((-dx_).atan2(-dz_));
}";

/// A field of an engine class: its name, its Rust type and its first
/// value, which may be an argument of `init`.
type Field = (&'static str, &'static str, &'static str);

/// An engine node class placed in 3D.
struct NodeClass {
    name: &'static str,
    /// What the fields are, for the documentation of `Fields`.
    about: &'static str,
    /// The classes it extends, from `Node` down.
    ancestors: &'static [&'static str],
    fields: &'static [Field],
    /// The arguments of its `init` after the name, which give their own
    /// field its first value.
    parameters: &'static [(&'static str, &'static str)],
    methods: &'static [Method],
    /// The code of each of its methods, in the same order.
    bodies: &'static [&'static str],
}

const STRING: &str = "::std::string::String";
const COLOR: &str = "crate::classes::Color::Ref";
const WHITE: &str = "crate::classes::Color::new(1.0, 1.0, 1.0, 1.0)";

const NODE_3D: NodeClass = NodeClass {
    name: "Node3D",
    about: "its depth, its pitch and yaw (its roll is `rotation`) and its depth scale",
    ancestors: &["Node"],
    fields: &[
        ("z", "f32", "0.0"),
        ("pitch", "f32", "0.0"),
        ("yaw", "f32", "0.0"),
        ("scale_z", "f32", "1.0"),
    ],
    parameters: &[],
    methods: &NODE_3D_METHODS,
    bodies: &[LOOK_AT_BODY],
};

const MESH: NodeClass = NodeClass {
    name: "Mesh",
    about: "the shape it shows, its sizes and its surface",
    ancestors: &["Node", "Node3D"],
    fields: &[
        ("shape", STRING, "shape"),
        ("width", "f32", "1.0"),
        ("height", "f32", "1.0"),
        ("depth", "f32", "1.0"),
        ("radius", "f32", "0.5"),
        ("color", COLOR, WHITE),
        ("metallic", "f32", "0.0"),
        ("roughness", "f32", "0.5"),
        ("texture", STRING, "::std::string::String::new()"),
        ("pixelated", "bool", "false"),
    ],
    parameters: &[("shape", STRING)],
    methods: &[],
    bodies: &[],
};

const LIGHT: NodeClass = NodeClass {
    name: "Light",
    about: "the kind of light it shines, its color, how bright it is and how far it reaches",
    ancestors: &["Node", "Node3D"],
    fields: &[
        ("kind", STRING, "kind"),
        ("color", COLOR, WHITE),
        ("intensity", "f32", "1.0"),
        ("range", "f32", "10.0"),
    ],
    parameters: &[("kind", STRING)],
    methods: &[],
    bodies: &[],
};

const CAMERA_3D: NodeClass = NodeClass {
    name: "Camera3D",
    about: "how wide it sees, in radians",
    ancestors: &["Node", "Node3D"],
    fields: &[("fov", "f32", "::std::f32::consts::FRAC_PI_3")],
    parameters: &[],
    methods: &[],
    bodies: &[],
};

/// The modules of `Node3D`, `Mesh`, `Light` and `Camera3D`.
pub(super) fn node_3d_modules(out: &mut Writer) {
    for class in [NODE_3D, MESH, LIGHT, CAMERA_3D] {
        out.line("");
        module(out, &class);
    }
}

/// The methods each class of `ancestors` declares.
fn methods_of(class: &str) -> &'static [Method] {
    match class {
        "Node" => &NODE_METHODS,
        "Node3D" => &NODE_3D_METHODS,
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
    out.open("pub struct Fields {");
    for (field, ty, _) in class.fields {
        out.line(&format!("pub {field}: ::star_script_runtime::Field<{ty}>,"));
    }
    out.close("}");
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
    out.open("let fields_ = Fields {");
    for (field, _, value) in class.fields {
        out.line(&format!(
            "{field}: ::star_script_runtime::Field::new({value}),"
        ));
    }
    out.close("};");
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
