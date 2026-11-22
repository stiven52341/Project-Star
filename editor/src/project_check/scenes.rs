//! Checking `.scene` files: that they can be read, and that the StarScript
//! classes their nodes name can build those nodes.

use star_engine::star_core::EngineError;
use star_engine::star_scene::SceneOutline;
use star_script::semantic::ClassTable;

use super::Problem;

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
        .filter_map(|node| {
            let class = node.class.as_deref()?;
            let error = classes.scene_class(class).err()?;
            Some(Problem::in_file(
                name,
                format!("node \"{}\": {error}", node.name),
            ))
        })
        .collect()
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
