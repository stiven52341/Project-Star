//! Scripts and blueprints dragged from the assets panel and dropped on a
//! node of the tree: the node is built as one of their classes.

use std::fs;
use std::path::Path;

use star_blueprint::Blueprint;
use star_script::{CompileError, LineIndex};

use crate::project::{BLUEPRINT_EXTENSION, SCRIPT_EXTENSION};

/// What the assets panel carries while a script or a blueprint is dragged:
/// read once, when the drag starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClassDrop {
    /// Where it is inside the assets folder.
    pub(crate) path: String,
    /// The classes it declares that extend another class, in file order:
    /// only those can be nodes (a node's class extends `Node`). Empty if
    /// it has none or cannot be read.
    pub(crate) classes: Vec<String>,
    /// Why its classes could not be read, if it has mistakes: the first
    /// one, as `line:column: message` for a script. A file that does not
    /// read would otherwise look like one without node classes.
    pub(crate) mistake: Option<String>,
}

impl ClassDrop {
    /// Reads the classes of the script or blueprint at `path` inside the
    /// `assets` folder; `None` if it is neither.
    pub(crate) fn read(assets: &Path, path: String) -> Option<Self> {
        let is_script = path.ends_with(SCRIPT_EXTENSION);
        if !is_script && !path.ends_with(BLUEPRINT_EXTENSION) {
            return None;
        }
        let text = fs::read_to_string(assets.join(&path)).unwrap_or_default();
        let read = if is_script {
            script_classes(&text)
        } else {
            blueprint_class(&text).map(|class| class.into_iter().collect())
        };
        let (classes, mistake) = match read {
            Ok(classes) => (classes, None),
            Err(mistake) => (Vec::new(), Some(mistake)),
        };
        Some(Self {
            path,
            classes,
            mistake,
        })
    }
}

/// The classes of a script that extend another class, or its first
/// mistake if it does not parse.
fn script_classes(text: &str) -> Result<Vec<String>, String> {
    match star_script::parse_source(text) {
        Ok(source) => Ok(source
            .classes
            .into_iter()
            .filter(|class| class.base.is_some())
            .map(|class| class.name.text)
            .collect()),
        Err(mistakes) => Err(first_mistake(text, &mistakes)),
    }
}

/// `line:column: message` of the first of `mistakes` found in `text`.
fn first_mistake(text: &str, mistakes: &[CompileError]) -> String {
    mistakes.first().map_or_else(String::new, |mistake| {
        let position = LineIndex::new(text).position(mistake.span().start());
        format!("{}:{}: {mistake}", position.line, position.column)
    })
}

/// The class of a blueprint, if it extends another class, or why the
/// blueprint cannot be read.
fn blueprint_class(text: &str) -> Result<Option<String>, String> {
    Blueprint::from_text(text)
        .map(|blueprint| blueprint.extends.is_some().then_some(blueprint.class))
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::TestDirectory;

    #[test]
    fn scripts_offer_the_classes_that_extend_another() {
        let classes = script_classes(
            "class Helper {}\n\
             class Rock extends Sprite {}\n\
             class Boss extends Enemy {}\n",
        );

        assert_eq!(classes.unwrap(), ["Rock", "Boss"]);
    }

    #[test]
    fn blueprints_offer_their_class_if_it_extends_another() {
        let rock = "format = 1\nclass = \"Rock\"\nextends = \"Sprite\"\n";
        let helper = "format = 1\nclass = \"Helper\"\n";

        assert_eq!(blueprint_class(rock).unwrap().as_deref(), Some("Rock"));
        assert_eq!(blueprint_class(helper).unwrap(), None);
    }

    // Regression: a script with a syntax mistake was dragged with the
    // warning that none of its classes extends another.
    #[test]
    fn scripts_that_do_not_parse_give_their_first_mistake() {
        let mistake =
            script_classes("class Rock extends Node {\n    export speed: float\n}").unwrap_err();

        assert!(mistake.starts_with("2:12: "), "{mistake}");
    }

    #[test]
    fn blueprints_that_do_not_read_give_why() {
        assert!(blueprint_class("format = ").is_err());
    }

    #[test]
    fn only_scripts_and_blueprints_carry_classes() {
        let assets = TestDirectory::new("class_drop_read");
        fs::create_dir_all(assets.path()).unwrap();
        fs::write(
            assets.path().join("rock.star"),
            "class Rock extends Node {}",
        )
        .unwrap();

        let script = ClassDrop::read(assets.path(), "rock.star".to_owned()).unwrap();
        let missing = ClassDrop::read(assets.path(), "gone.blueprint".to_owned()).unwrap();

        assert_eq!(script.classes, ["Rock"]);
        assert_eq!(script.mistake, None);
        assert_eq!(missing.classes, Vec::<String>::new());
        assert!(ClassDrop::read(assets.path(), "hero.png".to_owned()).is_none());
    }
}
