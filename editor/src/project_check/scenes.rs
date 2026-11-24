//! Checking `.scene` files: that they can be read, that the StarScript
//! classes their nodes name can build those nodes, and that the values
//! they give fields go to `export` fields that take them.

use star_engine::star_core::EngineError;
use star_engine::star_scene::{FieldValue, NodeOutline, SceneOutline};
use star_script::semantic::{COLOR_CLASS, ClassTable, FieldInfo, NODE_CLASS, Type};

use super::Problem;
use crate::texts;

/// The problems of the scene called `name` with the text `text`.
///
/// The class names are only checked when the scripts could be checked
/// (`classes`): otherwise every class would look unknown.
pub(super) fn check_scene(name: &str, text: &str, classes: Option<&ClassTable>) -> Vec<Problem> {
    let outline = match SceneOutline::from_text(text) {
        Ok(outline) => outline,
        Err(error) => return vec![Problem::in_file(name, reason(error))],
    };
    let Some(classes) = classes else {
        return Vec::new();
    };
    outline
        .nodes()
        .iter()
        .flat_map(|node| {
            node_problems(node, classes)
                .into_iter()
                .map(|problem| Problem::in_file(name, format!("node \"{}\": {problem}", node.name)))
        })
        .collect()
}

/// The problem of the main scene called `name` with the text `text`, a
/// project without a `Game` class plays: it must be seen from a main
/// camera. A scene that cannot be read has its problem given by
/// [`check_scene`].
pub(super) fn check_main_scene(name: &str, text: &str) -> Option<Problem> {
    let outline = SceneOutline::from_text(text).ok()?;
    outline
        .main_camera()
        .is_none()
        .then(|| Problem::in_file(name, texts::current().main_scene_without_camera.to_owned()))
}

/// What is wrong with the class of `node` and the values it gives fields.
fn node_problems(node: &NodeOutline, classes: &ClassTable) -> Vec<String> {
    let class = node.class.as_deref().unwrap_or(NODE_CLASS);
    let id = match classes.scene_class(class) {
        Ok(id) => id,
        Err(error) if node.class.is_some() => return vec![error.to_string()],
        // A node without a class is a plain node, with no fields to give.
        Err(_) => return unknown_fields(class, node),
    };
    node.fields
        .iter()
        .filter_map(|(field, value)| {
            match classes
                .ancestors(id)
                .find_map(|owner| exported_field(classes, owner, field))
            {
                None => Some(unknown_field(class, field)),
                Some(info) if !fits(classes, value, &info.ty) => Some(format!(
                    "the field `{field}` takes a value of type `{}`",
                    classes.type_name(&info.ty)
                )),
                Some(_) => None,
            }
        })
        .collect()
}

/// The `export` field `name` declared by the class `owner`.
fn exported_field<'classes>(
    classes: &'classes ClassTable,
    owner: star_script::semantic::ClassId,
    name: &str,
) -> Option<&'classes FieldInfo> {
    classes
        .class(owner)
        .fields
        .iter()
        .find(|field| field.is_exported && field.name == name)
}

/// One problem for each value `node` gives, as `class` has no fields.
fn unknown_fields(class: &str, node: &NodeOutline) -> Vec<String> {
    node.fields
        .keys()
        .map(|field| unknown_field(class, field))
        .collect()
}

fn unknown_field(class: &str, field: &str) -> String {
    format!("`{class}` has no `export` field `{field}`")
}

/// Whether a scene's `value` can be given to a field of type `ty`, as
/// games take it: whole numbers that fit, decimals also from whole
/// numbers, and each other kind only its own.
fn fits(classes: &ClassTable, value: &FieldValue, ty: &Type) -> bool {
    match (value, ty) {
        (FieldValue::Int(number), Type::Short) => i16::try_from(*number).is_ok(),
        (FieldValue::Int(number), Type::Int) => i32::try_from(*number).is_ok(),
        (FieldValue::Int(_), Type::BigInt | Type::Float | Type::Double)
        | (FieldValue::Float(_), Type::Float | Type::Double)
        | (FieldValue::Bool(_), Type::Bool)
        | (FieldValue::Text(_), Type::String) => true,
        (FieldValue::Color(_), Type::Class(id)) => classes.class(*id).name == COLOR_CLASS,
        _ => false,
    }
}

/// What is wrong with a scene, without the "scene error" prefix: the problem
/// already says it is in a scene file.
fn reason(error: EngineError) -> String {
    match error {
        EngineError::Scene(reason) => reason,
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use star_script::check_source;

    const SCENE_WITH_ROCK: &str = "format = 1\n\n[[node]]\nname = \"Level\"\n\n[[node]]\nname = \"Big rock\"\nparent = 0\nclass = \"Rock\"\n";

    fn messages(source: &str, scene: &str) -> Vec<String> {
        let checked = check_source(source).expect("valid program");
        check_scene(
            "assets/level.scene",
            scene,
            Some(checked.analysis.classes()),
        )
        .iter()
        .map(ToString::to_string)
        .collect()
    }

    #[test]
    fn scenes_that_name_node_classes_are_fine() {
        let source = "class Rock extends Node {\n  init() {\n    super.init(\"Rock\")\n  }\n}\n";

        assert_eq!(messages(source, SCENE_WITH_ROCK), Vec::<String>::new());
    }

    #[test]
    fn unknown_classes_name_the_node() {
        assert_eq!(
            messages("class Rocks {}\n", SCENE_WITH_ROCK),
            ["assets/level.scene: node \"Big rock\": there is no class \"Rock\""]
        );
    }

    #[test]
    fn values_go_to_export_fields_that_take_them() {
        let source = "class Rock extends Node {\n  export var speed = 1.0\n  export var lives = 3\n  \
                      var hidden = 0\n  init() {\n    super.init(\"Rock\")\n  }\n}\n";
        let scene = format!(
            "{SCENE_WITH_ROCK}[node.fields]\nspeed = 2\nlives = 2.5\nhidden = 1\nghost = true\n"
        );

        assert_eq!(
            messages(source, &scene),
            [
                "assets/level.scene: node \"Big rock\": `Rock` has no `export` field `ghost`",
                "assets/level.scene: node \"Big rock\": `Rock` has no `export` field `hidden`",
                "assets/level.scene: node \"Big rock\": the field `lives` takes a value of type `int`",
            ]
        );
    }

    #[test]
    fn plain_nodes_have_no_fields() {
        let scene = "format = 1\n[[node]]\nname = \"Level\"\n[node.fields]\nspeed = 2\n";

        assert_eq!(
            messages("class A {}\n", scene),
            ["assets/level.scene: node \"Level\": `Node` has no `export` field `speed`"]
        );
    }

    #[test]
    fn unreadable_scenes_are_problems() {
        let problems = check_scene("assets/level.scene", "format = 99\n", None);

        assert_eq!(problems.len(), 1);
        assert_eq!(problems[0].file, "assets/level.scene");
    }

    #[test]
    fn classes_are_not_checked_without_the_scripts() {
        assert_eq!(check_scene("assets/level.scene", SCENE_WITH_ROCK, None), []);
    }
}
