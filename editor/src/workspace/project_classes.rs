//! The class names already taken in a project: the engine's and those its
//! scripts and blueprints declare. Scripts and blueprints form one program,
//! so a new one may not declare a class that already exists.

use std::fs;
use std::path::Path;

use star_blueprint::Blueprint;
use star_script::semantic::native_classes;
use star_script::{Keyword, TokenKind, scan};

use crate::file_tree::{file_paths, scan_directory};
use crate::project::{BLUEPRINT_EXTENSION, SCRIPT_EXTENSION};

/// Whether `class` is a class of the engine, or one that a script or
/// blueprint of the `assets` folder declares, besides the file at
/// `skipped` (the one being renamed).
pub(super) fn is_class_taken(assets: &Path, class: &str, skipped: Option<&str>) -> bool {
    if native_classes().find(class).is_some() {
        return true;
    }
    let paths = scan_directory(assets)
        .map(|nodes| file_paths(&nodes))
        .unwrap_or_default();
    paths
        .iter()
        .filter(|path| Some(path.as_str()) != skipped)
        .filter_map(|path| Some((path, fs::read_to_string(assets.join(path)).ok()?)))
        .any(|(path, text)| {
            declared_classes(path, &text)
                .iter()
                .any(|name| name == class)
        })
}

/// The classes the file at `path` declares, if it is a script or a
/// blueprint.
fn declared_classes(path: &str, text: &str) -> Vec<String> {
    if path.ends_with(SCRIPT_EXTENSION) {
        script_classes(text)
    } else if path.ends_with(BLUEPRINT_EXTENSION) {
        Blueprint::from_text(text)
            .map(|blueprint| vec![blueprint.class])
            .unwrap_or_default()
    } else {
        Vec::new()
    }
}

/// The names after each `class` of a script, read from its tokens: a
/// script with mistakes still declares its classes.
fn script_classes(text: &str) -> Vec<String> {
    let tokens = scan(text).tokens;
    tokens
        .windows(2)
        .filter_map(|pair| match (&pair[0].kind, &pair[1].kind) {
            (TokenKind::Keyword(Keyword::Class), TokenKind::Identifier(name)) => Some(name.clone()),
            _ => None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::TestDirectory;

    fn assets(name: &str) -> TestDirectory {
        let assets = TestDirectory::new(name);
        fs::create_dir_all(assets.path().join("enemies")).unwrap();
        fs::write(
            assets.path().join("enemies/rock.star"),
            "class Rock extends Node {\n  export speed: float\n}\nclass Pebble {}\n",
        )
        .unwrap();
        fs::write(
            assets.path().join("Ship.blueprint"),
            "format = 1\nclass = \"Ship\"\nextends = \"Node\"\n",
        )
        .unwrap();
        assets
    }

    #[test]
    fn classes_of_scripts_blueprints_and_the_engine_are_taken() {
        let assets = assets("project_classes_taken");

        for class in ["Rock", "Pebble", "Ship", "Sprite", "Node"] {
            assert!(is_class_taken(assets.path(), class, None), "{class}");
        }
        assert!(!is_class_taken(assets.path(), "Comet", None));
    }

    #[test]
    fn the_file_being_renamed_does_not_take_its_own_class() {
        let assets = assets("project_classes_skipped");

        assert!(!is_class_taken(
            assets.path(),
            "Ship",
            Some("Ship.blueprint")
        ));
        assert!(is_class_taken(
            assets.path(),
            "Rock",
            Some("Ship.blueprint")
        ));
    }

    #[test]
    fn scripts_with_mistakes_still_declare_their_classes() {
        assert_eq!(
            script_classes("class Rock extends Node {\n  export speed: float;\n"),
            ["Rock"]
        );
        assert_eq!(
            script_classes("// class Comet\nvar class = 1"),
            Vec::<String>::new()
        );
    }
}
