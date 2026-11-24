//! Scripts and blueprints dragged from the assets panel and dropped on a
//! node of the tree: the node is built as one of their classes.

use std::fs;
use std::path::Path;

use star_blueprint::Blueprint;

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
        let classes = if is_script {
            script_classes(&text)
        } else {
            blueprint_class(&text).into_iter().collect()
        };
        Some(Self { path, classes })
    }
}

/// The classes of a script that extend another class.
fn script_classes(text: &str) -> Vec<String> {
    star_script::parse_source(text).map_or_else(
        |_| Vec::new(),
        |source| {
            source
                .classes
                .into_iter()
                .filter(|class| class.base.is_some())
                .map(|class| class.name.text)
                .collect()
        },
    )
}

/// The class of a blueprint, if it extends another class.
fn blueprint_class(text: &str) -> Option<String> {
    Blueprint::from_text(text)
        .ok()
        .filter(|blueprint| blueprint.extends.is_some())
        .map(|blueprint| blueprint.class)
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

        assert_eq!(classes, ["Rock", "Boss"]);
    }

    #[test]
    fn blueprints_offer_their_class_if_it_extends_another() {
        let rock = "format = 1\nclass = \"Rock\"\nextends = \"Sprite\"\n";
        let helper = "format = 1\nclass = \"Helper\"\n";

        assert_eq!(blueprint_class(rock).as_deref(), Some("Rock"));
        assert_eq!(blueprint_class(helper), None);
    }

    #[test]
    fn scripts_that_do_not_parse_offer_nothing() {
        assert_eq!(script_classes("class {"), Vec::<String>::new());
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
        assert_eq!(missing.classes, Vec::<String>::new());
        assert!(ClassDrop::read(assets.path(), "hero.png".to_owned()).is_none());
    }
}
