//! What the language provides without declaring it: native classes and the
//! methods of basic types and lists.

use super::{
    ClassId, ClassInfo, ClassOrigin, ClassTable, ConstructorInfo, FieldInfo, MethodInfo,
    ParameterInfo, Type,
};
use crate::Span;

/// Names of the basic types; they cannot be used as class names.
pub(super) const BASIC_TYPE_NAMES: [&str; 4] = ["int", "float", "bool", "string"];

/// The basic type called `name`.
pub(super) fn basic_type(name: &str) -> Option<Type> {
    match name {
        "int" => Some(Type::Int),
        "float" => Some(Type::Float),
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
pub const CAMERA_CLASS: &str = "Camera";

/// Name of the engine class that loads `.scene` files.
pub const SCENE_CLASS: &str = "Scene";

/// Name of the engine class of colors: of sprites and of the background.
pub const COLOR_CLASS: &str = "Color";

/// Name of the engine class of the sounds a game loads and plays.
pub const SOUND_CLASS: &str = "Sound";

/// Name of the engine class of the game's audio as a whole.
pub const AUDIO_CLASS: &str = "Audio";

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
    let node = add_node_class(classes);
    add_sprite_class(classes, node, color);
    let camera = add_camera_class(classes, node);

    // Builds the nodes of a `.scene` file, or gives `null` if it cannot.
    let mut scene = ClassInfo::new(SCENE_CLASS.to_owned(), ClassOrigin::Native);
    scene.methods.push(native_method(
        "load",
        &[("path", Type::String)],
        Type::Class(node).nullable(),
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

    // A game: the engine calls `start` once and `update` every frame, until
    // `quit` is called or the window is closed. Its nodes hang from it.
    let mut game = ClassInfo::new(GAME_CLASS.to_owned(), ClassOrigin::Native);
    game.fields = vec![
        native_field("background", Type::Class(color)),
        native_field("camera", Type::Class(camera).nullable()),
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
fn add_node_class(classes: &mut ClassTable) -> ClassId {
    let mut node = ClassInfo::new(NODE_CLASS.to_owned(), ClassOrigin::Native);
    node.fields = vec![
        native_field("name", Type::String),
        native_field("x", Type::Float),
        native_field("y", Type::Float),
        native_field("rotation", Type::Float),
        native_field("scale_x", Type::Float),
        native_field("scale_y", Type::Float),
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
            node_type.nullable(),
            false,
        ),
    ];
    classes.class_mut(id).methods = methods.into();
    id
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

/// `Camera`: a node the game is seen from, when it is the game's `camera`.
fn add_camera_class(classes: &mut ClassTable, node: ClassId) -> ClassId {
    let mut camera = ClassInfo::new(CAMERA_CLASS.to_owned(), ClassOrigin::Native);
    camera.base = Some(node);
    camera.fields = vec![native_field("view_height", Type::Float)];
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
        span: None,
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
pub(super) struct BuiltinSignature {
    pub(super) method: BuiltinMethod,
    pub(super) parameters: Vec<Type>,
    pub(super) return_type: Type,
}

/// The method `name` of a value of type `receiver`, if it has one.
pub(super) fn builtin_method(receiver: &Type, name: &str) -> Option<BuiltinSignature> {
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
        (Type::Int | Type::Float, Method::Abs) => (vec![], receiver.clone()),
        (Type::Int | Type::Float, Method::Min | Method::Max) => {
            (vec![receiver.clone()], receiver.clone())
        }
        (Type::Float, Method::Floor | Method::Ceil | Method::Round | Method::Sqrt) => {
            (vec![], Type::Float)
        }
        (Type::Int | Type::Float | Type::Bool | Type::String, Method::ToString)
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
    fn methods_only_exist_on_their_types() {
        assert!(builtin_method(&Type::Float, "sqrt").is_some());
        assert!(builtin_method(&Type::Int, "sqrt").is_none());
        assert!(builtin_method(&Type::Bool, "length").is_none());
        assert!(builtin_method(&Type::Int, "explode").is_none());
    }
}
