//! What the language provides without declaring it: native classes and the
//! methods of basic types and lists.

use super::{ClassInfo, ClassOrigin, ClassTable, MethodInfo, ParameterInfo, Type};

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

/// Adds the classes implemented by the engine.
pub(super) fn add_native_classes(classes: &mut ClassTable) {
    let mut debug = ClassInfo::new("Debug".to_owned(), ClassOrigin::Native);
    debug
        .methods
        .push(native_method("print", &[("message", Type::String)], true));
    classes.add(debug);

    // A game: the engine calls `start` once and `update` every frame, until
    // `quit` is called or the window is closed.
    let mut game = ClassInfo::new(GAME_CLASS.to_owned(), ClassOrigin::Native);
    game.methods.push(native_method("start", &[], false));
    game.methods
        .push(native_method("update", &[("delta", Type::Float)], false));
    game.methods.push(native_method("quit", &[], false));
    classes.add(game);
}

/// A method of an engine class, without a result.
fn native_method(name: &str, parameters: &[(&str, Type)], is_static: bool) -> MethodInfo {
    MethodInfo {
        name: name.to_owned(),
        parameters: parameters
            .iter()
            .map(|(name, ty)| ParameterInfo {
                name: (*name).to_owned(),
                ty: ty.clone(),
            })
            .collect(),
        return_type: Type::Void,
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
