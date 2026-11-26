//! What the language provides without declaring it: native classes and the
//! methods of basic types and lists.

use super::{
    ClassId, ClassInfo, ClassOrigin, ClassTable, ConstructorInfo, Execution, FieldInfo,
    InspectorHints, MethodInfo, ParameterInfo, Type,
};
use crate::Span;

/// Names of the basic types; they cannot be used as class names.
pub const BASIC_TYPE_NAMES: [&str; 7] = [
    "short", "int", "bigint", "float", "double", "bool", "string",
];

/// The basic type called `name`.
#[must_use]
pub fn basic_type(name: &str) -> Option<Type> {
    match name {
        "short" => Some(Type::Short),
        "int" => Some(Type::Int),
        "bigint" => Some(Type::BigInt),
        "float" => Some(Type::Float),
        "double" => Some(Type::Double),
        "bool" => Some(Type::Bool),
        "string" => Some(Type::String),
        _ => None,
    }
}

/// Name of the engine class that games extend.
pub const GAME_CLASS: &str = "Game";

/// Name of the engine class that reads the game's input actions.
pub const INPUT_CLASS: &str = "Input";

/// Name of the engine class of the objects of a game's scene tree.
pub const NODE_CLASS: &str = "Node";

/// Name of the engine class of the nodes that show an image.
pub const SPRITE_CLASS: &str = "Sprite";

/// Name of the engine class of the nodes the game is seen from.
pub const CAMERA_CLASS: &str = "Camera2D";

/// Name of the engine class of the nodes placed in 3D.
pub const NODE_3D_CLASS: &str = "Node3D";

/// Name of the engine class of the 3D nodes that show a mesh.
pub const MESH_CLASS: &str = "Mesh";

/// Name of the engine class of the 3D nodes that shine a light.
pub const LIGHT_CLASS: &str = "Light";

/// Name of the engine class of the 2D nodes that light the sprites.
pub const LIGHT_2D_CLASS: &str = "Light2D";

/// Name of the engine class of the 3D nodes the game is seen from.
pub const CAMERA_3D_CLASS: &str = "Camera3D";

/// Name of the engine class that loads `.scene` files.
pub const SCENE_CLASS: &str = "Scene";

/// Name of the engine class of colors: of sprites and of the background.
pub const COLOR_CLASS: &str = "Color";

/// Name of the engine class of the sounds a game loads and plays.
pub const SOUND_CLASS: &str = "Sound";

/// Name of the engine class of the game's audio as a whole.
pub const AUDIO_CLASS: &str = "Audio";

/// Name of the engine class of what coroutines wait for.
pub const TIME_CLASS: &str = "Time";

/// Name of the engine class of the physics bodies of nodes.
pub const BODY_CLASS: &str = "Body";

/// The static methods of `Body` that make one, with the sizes of their
/// shape after the kind: of 2D, of 3D, and the capsule of both.
pub(super) const BODY_SHAPES: [(&str, &[&str]); 5] = [
    ("circle", &["radius"]),
    ("rectangle", &["width", "height"]),
    ("capsule", &["height", "radius"]),
    ("sphere", &["radius"]),
    ("box", &["width", "height", "depth"]),
];

/// The methods of `Body` that list the nodes it touches.
pub(super) const BODY_CONTACT_METHODS: [&str; 3] = ["touching", "just_touched", "just_separated"];

/// The methods of `Input` that take the name of an action.
pub(super) const INPUT_ACTION_METHODS: [(&str, Type); 4] = [
    ("is_down", Type::Bool),
    ("just_pressed", Type::Bool),
    ("just_released", Type::Bool),
    ("strength", Type::Float),
];

/// The method of `Input` that takes two opposite actions.
pub(super) const INPUT_AXIS_METHOD: &str = "axis";

/// The methods of `Input` that say where the player points, without arguments.
pub(super) const INPUT_POINTER_METHODS: [(&str, Type); 3] = [
    ("pointer_x", Type::Float),
    ("pointer_y", Type::Float),
    ("has_pointer", Type::Bool),
];

/// Adds the classes implemented by the engine.
pub(super) fn add_native_classes(classes: &mut ClassTable) {
    let mut debug = ClassInfo::new("Debug".to_owned(), ClassOrigin::Native);
    debug.methods.push(native_method(
        "print",
        &[("message", Type::String)],
        Type::Void,
        true,
    ));
    classes.add(debug);

    // The game's input actions, declared in the project settings, and where
    // the player points.
    let mut input = ClassInfo::new(INPUT_CLASS.to_owned(), ClassOrigin::Native);
    for (name, return_type) in INPUT_ACTION_METHODS {
        input.methods.push(native_method(
            name,
            &[("action", Type::String)],
            return_type,
            true,
        ));
    }
    input.methods.push(native_method(
        INPUT_AXIS_METHOD,
        &[("negative", Type::String), ("positive", Type::String)],
        Type::Float,
        true,
    ));
    for (name, return_type) in INPUT_POINTER_METHODS {
        input
            .methods
            .push(native_method(name, &[], return_type, true));
    }
    classes.add(input);

    let color = add_color_class(classes);
    let body = add_body_class(classes);
    let node = add_node_class(classes, body);
    add_body_methods(classes, body, node);
    add_sprite_class(classes, node, color);
    let camera = add_camera_class(classes, node);
    add_light_2d_class(classes, node, color);
    let node_3d = add_node_3d_class(classes, node);
    add_mesh_class(classes, node_3d, color);
    add_light_class(classes, node_3d, color);
    let camera_3d = add_camera_3d_class(classes, node_3d);

    // Builds the nodes of a `.scene` file, or gives `null` if it cannot.
    let mut scene = ClassInfo::new(SCENE_CLASS.to_owned(), ClassOrigin::Native);
    scene.methods.push(native_method(
        "load",
        &[("path", Type::String)],
        Type::Class(node).nullable(),
        true,
    ));
    // Find the nodes of the game's tree, from any script.
    scene.methods.push(native_method(
        "find_by_name",
        &[("name", Type::String)],
        Type::Class(node).nullable(),
        true,
    ));
    scene.methods.push(native_method(
        "find_by_tag",
        &[("tag", Type::String)],
        Type::list(Type::Class(node)),
        true,
    ));
    classes.add(scene);

    add_sound_class(classes);
    // The volume every sound is scaled by.
    let mut audio = ClassInfo::new(AUDIO_CLASS.to_owned(), ClassOrigin::Native);
    audio.methods = vec![
        native_method("volume", &[], Type::Float, true),
        native_method("set_volume", &[("volume", Type::Float)], Type::Void, true),
    ];
    classes.add(audio);

    // What coroutines wait for (seconds of the game, or the next frame),
    // and the game's clock.
    let mut time = ClassInfo::new(TIME_CLASS.to_owned(), ClassOrigin::Native);
    time.methods = vec![
        native_async_method("wait", &[("seconds", Type::Float)]),
        native_async_method("next_frame", &[]),
        native_method("now", &[], Type::Float, true),
        native_method("delta", &[], Type::Float, true),
    ];
    classes.add(time);

    // A game: the engine calls `start` once and `update` every frame, until
    // `quit` is called or the window is closed. Its nodes hang from it.
    let mut game = ClassInfo::new(GAME_CLASS.to_owned(), ClassOrigin::Native);
    game.fields = vec![
        native_field("background", Type::Class(color)),
        native_field("camera", Type::Class(camera).nullable()),
        native_field("camera_3d", Type::Class(camera_3d).nullable()),
        native_field("ambient", Type::Class(color)),
        native_field("ambient_2d", Type::Class(color)),
    ];
    game.methods
        .push(native_method("start", &[], Type::Void, false));
    game.methods.push(native_method(
        "update",
        &[("delta", Type::Float)],
        Type::Void,
        false,
    ));
    game.methods
        .push(native_method("quit", &[], Type::Void, false));
    game.methods.push(native_method(
        "add_child",
        &[("node", Type::Class(node))],
        Type::Void,
        false,
    ));
    classes.add(game);
}

/// `Color`: red, green, blue and opacity, from `0.0` to `1.0`, as image
/// editors show them (sRGB).
fn add_color_class(classes: &mut ClassTable) -> ClassId {
    let mut color = ClassInfo::new(COLOR_CLASS.to_owned(), ClassOrigin::Native);
    let channels = [
        ("r", Type::Float),
        ("g", Type::Float),
        ("b", Type::Float),
        ("a", Type::Float),
    ];
    color.fields = channels
        .iter()
        .map(|(name, ty)| native_field(name, ty.clone()))
        .collect();
    color.constructor = Some(native_constructor(&channels));
    classes.add(color)
}

/// `Node`: an object of the scene tree, placed relative to its parent. The
/// engine calls `start` once and `update` every frame while it is in the tree.
fn add_node_class(classes: &mut ClassTable, body: ClassId) -> ClassId {
    let mut node = ClassInfo::new(NODE_CLASS.to_owned(), ClassOrigin::Native);
    node.fields = vec![
        native_field("name", Type::String),
        native_field("x", Type::Float),
        native_field("y", Type::Float),
        native_field("rotation", Type::Float),
        native_field("scale_x", Type::Float),
        native_field("scale_y", Type::Float),
        native_field("body", Type::Class(body).nullable()),
        native_field("tags", Type::list(Type::String)),
    ];
    node.constructor = Some(native_constructor(&[("name", Type::String)]));
    let id = classes.add(node);
    let node_type = Type::Class(id);
    let methods = [
        native_method("start", &[], Type::Void, false),
        native_method("update", &[("delta", Type::Float)], Type::Void, false),
        native_method(
            "add_child",
            &[("child", node_type.clone())],
            Type::Void,
            false,
        ),
        native_method("remove", &[], Type::Void, false),
        native_method("parent", &[], node_type.clone().nullable(), false),
        native_method("children", &[], Type::list(node_type.clone()), false),
        native_method(
            "find_child",
            &[("name", Type::String)],
            node_type.clone().nullable(),
            false,
        ),
        native_method("has_tag", &[("tag", Type::String)], Type::Bool, false),
        native_method(
            "find_by_name",
            &[("name", Type::String)],
            node_type.clone().nullable(),
            false,
        ),
        native_method(
            "find_by_tag",
            &[("tag", Type::String)],
            Type::list(node_type),
            false,
        ),
        native_method("global_x", &[], Type::Float, false),
        native_method("global_y", &[], Type::Float, false),
        native_method("global_rotation", &[], Type::Float, false),
        native_method(
            "set_global_position",
            &[("x", Type::Float), ("y", Type::Float)],
            Type::Void,
            false,
        ),
        native_method(
            "set_global_rotation",
            &[("angle", Type::Float)],
            Type::Void,
            false,
        ),
        native_method(
            "translate",
            &[("x", Type::Float), ("y", Type::Float)],
            Type::Void,
            false,
        ),
        native_method(
            "translate_global",
            &[("x", Type::Float), ("y", Type::Float)],
            Type::Void,
            false,
        ),
        native_method("rotate", &[("angle", Type::Float)], Type::Void, false),
    ];
    classes.class_mut(id).methods = methods.into();
    id
}

/// `Body`: the physics body of a node, made with `Body.circle`,
/// `Body.rectangle` or `Body.capsule` for 2D nodes, and `Body.sphere`,
/// `Body.box` or `Body.capsule` for 3D nodes (there is no `init`). Its kind
/// and shape are words, and it keeps the sizes of every shape.
fn add_body_class(classes: &mut ClassTable) -> ClassId {
    let mut body = ClassInfo::new(BODY_CLASS.to_owned(), ClassOrigin::Native);
    let floats = [
        "radius",
        "width",
        "height",
        "depth",
        "gravity_scale",
        "friction",
        "bounciness",
        "density",
        "velocity_x",
        "velocity_y",
        "velocity_z",
    ];
    body.fields = [("kind", Type::String), ("shape", Type::String)]
        .into_iter()
        .chain(floats.into_iter().map(|name| (name, Type::Float)))
        .chain([("fixed_rotation", Type::Bool), ("sensor", Type::Bool)])
        .map(|(name, ty)| native_field(name, ty))
        .collect();
    body.constructor = Some(ConstructorInfo {
        parameters: Vec::new(),
        is_private: true,
        span: Span::new(0, 0),
    });
    classes.add(body)
}

/// The methods of `Body`, which name `Node`s.
fn add_body_methods(classes: &mut ClassTable, body: ClassId, node: ClassId) {
    let mut methods: Vec<MethodInfo> = BODY_SHAPES
        .iter()
        .map(|(name, sizes)| {
            let parameters: Vec<(&str, Type)> = [("kind", Type::String)]
                .into_iter()
                .chain(sizes.iter().map(|size| (*size, Type::Float)))
                .collect();
            native_method(name, &parameters, Type::Class(body), true)
        })
        .collect();
    methods.push(native_method(
        "apply_impulse",
        &[("x", Type::Float), ("y", Type::Float)],
        Type::Void,
        false,
    ));
    methods.push(native_method(
        "apply_impulse_3d",
        &[("x", Type::Float), ("y", Type::Float), ("z", Type::Float)],
        Type::Void,
        false,
    ));
    methods.extend(
        BODY_CONTACT_METHODS
            .iter()
            .map(|name| native_method(name, &[], Type::list(Type::Class(node)), false)),
    );
    classes.class_mut(body).methods = methods;
}

/// `Sprite`: a node that shows an image of the asset folder.
fn add_sprite_class(classes: &mut ClassTable, node: ClassId, color: ClassId) -> ClassId {
    let mut sprite = ClassInfo::new(SPRITE_CLASS.to_owned(), ClassOrigin::Native);
    sprite.base = Some(node);
    sprite.fields = vec![
        native_field("texture", Type::String),
        native_field("width", Type::Float),
        native_field("height", Type::Float),
        native_field("layer", Type::Int),
        native_field("pixelated", Type::Bool),
        native_field("color", Type::Class(color)),
    ];
    sprite.constructor = Some(native_constructor(&[
        ("name", Type::String),
        ("texture", Type::String),
    ]));
    classes.add(sprite)
}

/// `Camera2D`: a node the game is seen from, when it is the game's `camera`;
/// a `current` one becomes it when it enters the game's tree.
fn add_camera_class(classes: &mut ClassTable, node: ClassId) -> ClassId {
    let mut camera = ClassInfo::new(CAMERA_CLASS.to_owned(), ClassOrigin::Native);
    camera.base = Some(node);
    camera.fields = vec![
        native_field("view_height", Type::Float),
        native_field("main", Type::Bool),
    ];
    camera.constructor = Some(native_constructor(&[("name", Type::String)]));
    classes.add(camera)
}

/// `Node3D`: a node placed in 3D. Its `x`, `y`, `rotation` (the roll,
/// around z) and scales are those of `Node`; it adds the depth and the
/// other two angles.
fn add_node_3d_class(classes: &mut ClassTable, node: ClassId) -> ClassId {
    let mut node_3d = ClassInfo::new(NODE_3D_CLASS.to_owned(), ClassOrigin::Native);
    node_3d.base = Some(node);
    node_3d.fields = ["z", "pitch", "yaw", "scale_z"]
        .into_iter()
        .map(|name| native_field(name, Type::Float))
        .collect();
    node_3d.constructor = Some(native_constructor(&[("name", Type::String)]));
    let point = [("x", Type::Float), ("y", Type::Float), ("z", Type::Float)];
    node_3d.methods = vec![
        native_method("look_at", &point, Type::Void, false),
        native_method("global_z", &[], Type::Float, false),
        native_method("set_global_position_3d", &point, Type::Void, false),
        native_method("translate_3d", &point, Type::Void, false),
        native_method("translate_global_3d", &point, Type::Void, false),
    ];
    classes.add(node_3d)
}

/// `Mesh`: a 3D node that shows a basic shape with a PBR surface.
fn add_mesh_class(classes: &mut ClassTable, node_3d: ClassId, color: ClassId) -> ClassId {
    let mut mesh = ClassInfo::new(MESH_CLASS.to_owned(), ClassOrigin::Native);
    mesh.base = Some(node_3d);
    let floats = ["width", "height", "depth", "radius"]
        .into_iter()
        .map(|name| (name, Type::Float));
    mesh.fields = [("shape", Type::String)]
        .into_iter()
        .chain(floats)
        .chain([
            ("color", Type::Class(color)),
            ("metallic", Type::Float),
            ("roughness", Type::Float),
            ("texture", Type::String),
            ("pixelated", Type::Bool),
        ])
        .map(|(name, ty)| native_field(name, ty))
        .collect();
    mesh.constructor = Some(native_constructor(&[
        ("name", Type::String),
        ("shape", Type::String),
    ]));
    classes.add(mesh)
}

/// `Light`: a 3D node that lights the meshes.
fn add_light_class(classes: &mut ClassTable, node_3d: ClassId, color: ClassId) -> ClassId {
    let mut light = ClassInfo::new(LIGHT_CLASS.to_owned(), ClassOrigin::Native);
    light.base = Some(node_3d);
    light.fields = vec![
        native_field("kind", Type::String),
        native_field("color", Type::Class(color)),
        native_field("intensity", Type::Float),
        native_field("range", Type::Float),
        native_field("angle", Type::Float),
        native_field("blend", Type::Float),
    ];
    light.constructor = Some(native_constructor(&[
        ("name", Type::String),
        ("kind", Type::String),
    ]));
    classes.add(light)
}

/// `Light2D`: a 2D node that lights the sprites: a point light or a spot,
/// with the same fields as `Light`.
fn add_light_2d_class(classes: &mut ClassTable, node: ClassId, color: ClassId) -> ClassId {
    let mut light = ClassInfo::new(LIGHT_2D_CLASS.to_owned(), ClassOrigin::Native);
    light.base = Some(node);
    light.fields = vec![
        native_field("kind", Type::String),
        native_field("color", Type::Class(color)),
        native_field("intensity", Type::Float),
        native_field("range", Type::Float),
        native_field("angle", Type::Float),
        native_field("blend", Type::Float),
    ];
    light.constructor = Some(native_constructor(&[
        ("name", Type::String),
        ("kind", Type::String),
    ]));
    classes.add(light)
}

/// `Camera3D`: a 3D node the game is seen from, when it is the game's
/// `camera_3d`; a `current` one becomes it when it enters the game's tree.
fn add_camera_3d_class(classes: &mut ClassTable, node_3d: ClassId) -> ClassId {
    let mut camera = ClassInfo::new(CAMERA_3D_CLASS.to_owned(), ClassOrigin::Native);
    camera.base = Some(node_3d);
    camera.fields = vec![
        native_field("fov", Type::Float),
        native_field("main", Type::Bool),
    ];
    camera.constructor = Some(native_constructor(&[("name", Type::String)]));
    classes.add(camera)
}

/// `Sound`: a sound file of the asset folder, loaded with `Sound.load`
/// (there is no `init`). Each `play` starts it once more, at its `volume`,
/// looping if `looping`; `stop` stops every playing it started.
fn add_sound_class(classes: &mut ClassTable) -> ClassId {
    let mut sound = ClassInfo::new(SOUND_CLASS.to_owned(), ClassOrigin::Native);
    sound.fields = vec![
        native_field("volume", Type::Float),
        native_field("looping", Type::Bool),
    ];
    sound.constructor = Some(ConstructorInfo {
        parameters: Vec::new(),
        is_private: true,
        span: Span::new(0, 0),
    });
    let id = classes.add(sound);
    classes.class_mut(id).methods = vec![
        native_method(
            "load",
            &[("path", Type::String)],
            Type::Class(id).nullable(),
            true,
        ),
        native_method("play", &[], Type::Void, false),
        native_method("stop", &[], Type::Void, false),
        native_method("is_playing", &[], Type::Bool, false),
    ];
    id
}

/// A public field of an engine class; the engine gives it its first value.
fn native_field(name: &str, ty: Type) -> FieldInfo {
    FieldInfo {
        name: name.to_owned(),
        ty,
        is_private: false,
        is_exported: false,
        default: None,
        hints: InspectorHints::default(),
        has_initializer: true,
        span: Span::new(0, 0),
    }
}

/// The public `init` of an engine class.
fn native_constructor(parameters: &[(&str, Type)]) -> ConstructorInfo {
    ConstructorInfo {
        parameters: parameters_of(parameters),
        is_private: false,
        span: Span::new(0, 0),
    }
}

fn parameters_of(parameters: &[(&str, Type)]) -> Vec<ParameterInfo> {
    parameters
        .iter()
        .map(|(name, ty)| ParameterInfo {
            name: (*name).to_owned(),
            ty: ty.clone(),
        })
        .collect()
}

/// A method of an engine class.
fn native_method(
    name: &str,
    parameters: &[(&str, Type)],
    return_type: Type,
    is_static: bool,
) -> MethodInfo {
    MethodInfo {
        name: name.to_owned(),
        parameters: parameters_of(parameters),
        return_type,
        is_static,
        is_private: false,
        is_override: false,
        execution: Execution::Direct,
        span: None,
    }
}

/// A static method of an engine class that coroutines wait for, with
/// `await`; it gives nothing back.
fn native_async_method(name: &str, parameters: &[(&str, Type)]) -> MethodInfo {
    MethodInfo {
        execution: Execution::Async,
        ..native_method(name, parameters, Type::Void, true)
    }
}

/// A method of a basic type or list. The receiver type tells them apart:
/// `abs` of an `int` and of a `float` are both [`BuiltinMethod::Abs`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinMethod {
    /// `abs()`: value without sign.
    Abs,
    /// `min(other)`: the smaller value.
    Min,
    /// `max(other)`: the larger value.
    Max,
    /// `floor()`: rounds down.
    Floor,
    /// `ceil()`: rounds up.
    Ceil,
    /// `round()`: rounds to the nearest whole number.
    Round,
    /// `sqrt()`: square root.
    Sqrt,
    /// `to_string()`: the value as text.
    ToString,
    /// `length()`: number of characters or elements.
    Length,
    /// `is_empty()`: whether there are no characters or elements.
    IsEmpty,
    /// `contains(value)`: whether a text or element is present.
    Contains,
    /// `to_upper()`: text in capitals.
    ToUpper,
    /// `to_lower()`: text in lower case.
    ToLower,
    /// `push(value)`: adds an element at the end.
    Push,
    /// `pop()`: removes the last element, `null` if there is none.
    Pop,
    /// `clear()`: removes every element.
    Clear,
}

const BUILTIN_METHODS: [(&str, BuiltinMethod); 16] = [
    ("abs", BuiltinMethod::Abs),
    ("min", BuiltinMethod::Min),
    ("max", BuiltinMethod::Max),
    ("floor", BuiltinMethod::Floor),
    ("ceil", BuiltinMethod::Ceil),
    ("round", BuiltinMethod::Round),
    ("sqrt", BuiltinMethod::Sqrt),
    ("to_string", BuiltinMethod::ToString),
    ("length", BuiltinMethod::Length),
    ("is_empty", BuiltinMethod::IsEmpty),
    ("contains", BuiltinMethod::Contains),
    ("to_upper", BuiltinMethod::ToUpper),
    ("to_lower", BuiltinMethod::ToLower),
    ("push", BuiltinMethod::Push),
    ("pop", BuiltinMethod::Pop),
    ("clear", BuiltinMethod::Clear),
];

impl BuiltinMethod {
    /// How the method is called in StarScript.
    #[must_use]
    pub fn name(self) -> &'static str {
        BUILTIN_METHODS
            .iter()
            .find(|(_, method)| *method == self)
            .map_or("", |(name, _)| name)
    }
}

/// Parameter types and return type of a builtin method.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinSignature {
    /// Which method it is.
    pub method: BuiltinMethod,
    /// The types of its parameters, in order.
    pub parameters: Vec<Type>,
    /// What it gives.
    pub return_type: Type,
}

/// Every method a value of type `receiver` has, in a fixed order. Tools
/// use it to suggest them.
pub fn builtin_methods(receiver: &Type) -> impl Iterator<Item = BuiltinSignature> + '_ {
    BUILTIN_METHODS
        .iter()
        .filter_map(|(name, _)| builtin_method(receiver, name))
}

/// The method `name` of a value of type `receiver`, if it has one.
#[must_use]
pub fn builtin_method(receiver: &Type, name: &str) -> Option<BuiltinSignature> {
    let method = BUILTIN_METHODS
        .iter()
        .find(|(method_name, _)| *method_name == name)
        .map(|(_, method)| *method)?;
    let (parameters, return_type) = builtin_signature(receiver, method)?;
    Some(BuiltinSignature {
        method,
        parameters,
        return_type,
    })
}

fn builtin_signature(receiver: &Type, method: BuiltinMethod) -> Option<(Vec<Type>, Type)> {
    use BuiltinMethod as Method;

    let signature = match (receiver, method) {
        (number, Method::Abs) if number.is_numeric() => (vec![], receiver.clone()),
        (number, Method::Min | Method::Max) if number.is_numeric() => {
            (vec![receiver.clone()], receiver.clone())
        }
        (
            Type::Float | Type::Double,
            Method::Floor | Method::Ceil | Method::Round | Method::Sqrt,
        ) => (vec![], receiver.clone()),
        (value, Method::ToString) if value.is_numeric() => (vec![], Type::String),
        (Type::Bool | Type::String, Method::ToString)
        | (Type::String, Method::ToUpper | Method::ToLower) => (vec![], Type::String),
        (Type::String | Type::List(_), Method::Length) => (vec![], Type::Int),
        (Type::String | Type::List(_), Method::IsEmpty) => (vec![], Type::Bool),
        (Type::String, Method::Contains) => (vec![Type::String], Type::Bool),
        (Type::List(element), Method::Contains) => (vec![(**element).clone()], Type::Bool),
        (Type::List(element), Method::Push) => (vec![(**element).clone()], Type::Void),
        (Type::List(element), Method::Pop) => (vec![], element.clone().nullable()),
        (Type::List(_), Method::Clear) => (vec![], Type::Void),
        _ => return None,
    };
    Some(signature)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_builtin_has_a_name() {
        for (name, method) in BUILTIN_METHODS {
            assert_eq!(method.name(), name);
        }
    }

    #[test]
    fn list_methods_use_the_element_type() {
        let list = Type::list(Type::String);

        let push = builtin_method(&list, "push").expect("lists have push");
        let pop = builtin_method(&list, "pop").expect("lists have pop");

        assert_eq!(push.parameters, [Type::String]);
        assert_eq!(pop.return_type, Type::String.nullable());
    }

    #[test]
    fn every_method_of_a_type_is_listed() {
        let names: Vec<&str> = builtin_methods(&Type::Int)
            .map(|signature| signature.method.name())
            .collect();

        assert_eq!(names, ["abs", "min", "max", "to_string"]);
    }

    #[test]
    fn methods_only_exist_on_their_types() {
        assert!(builtin_method(&Type::Float, "sqrt").is_some());
        assert!(builtin_method(&Type::Int, "sqrt").is_none());
        assert!(builtin_method(&Type::Bool, "length").is_none());
        assert!(builtin_method(&Type::Int, "explode").is_none());
        assert!(builtin_method(&Type::Short, "abs").is_some());
        assert!(builtin_method(&Type::Double, "sqrt").is_some());
        assert!(builtin_method(&Type::BigInt, "to_string").is_some());
    }
}
