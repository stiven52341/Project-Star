//! The project's main scene, in `project.toml`: what “Play” plays when no
//! class extends `Game`.
//!
//! The file is the user's: its comments and order are kept, so the line is
//! written into its text rather than the whole file rewritten.

use std::fs;
use std::io;
use std::path::Path;

use serde::Deserialize;

use super::MANIFEST_FILE_NAME;
use super::project_table::{invalid, with_project_key};

/// The key of the main scene.
const MAIN_SCENE_KEY: &str = "main_scene";

/// The parts of `project.toml` read here.
#[derive(Debug, Default, Deserialize)]
struct MainSceneFile {
    #[serde(default)]
    project: MainSceneSection,
}

/// `[project]`
#[derive(Debug, Default, Deserialize)]
struct MainSceneSection {
    #[serde(default)]
    main_scene: Option<String>,
}

/// The main scene of the project in `directory`, as a path inside its
/// assets folder; `None` if it has none or its `project.toml` cannot be
/// read.
pub(crate) fn read_main_scene(directory: &Path) -> Option<String> {
    let text = fs::read_to_string(directory.join(MANIFEST_FILE_NAME)).ok()?;
    main_scene_of(&text)
}

/// Makes `path`, inside the assets folder, the main scene of the project
/// in `directory`.
///
/// # Errors
///
/// Fails if `project.toml` cannot be read or written, or has no
/// `[project]` table to write in (it then stays as it was).
pub(crate) fn set_main_scene(directory: &Path, path: &str) -> io::Result<()> {
    let file = directory.join(MANIFEST_FILE_NAME);
    let text = fs::read_to_string(&file)?;
    fs::write(&file, with_main_scene(&text, path)?)
}

fn main_scene_of(text: &str) -> Option<String> {
    toml::from_str::<MainSceneFile>(text)
        .ok()?
        .project
        .main_scene
}

/// `text` with `path` as its main scene: in place of the one it had, or
/// at the end of its `[project]` table.
fn with_main_scene(text: &str, path: &str) -> io::Result<String> {
    let new_text = with_project_key(text, MAIN_SCENE_KEY, &toml::Value::String(path.to_owned()))?;
    if main_scene_of(&new_text).as_deref() == Some(path) {
        Ok(new_text)
    } else {
        Err(invalid(format!(
            "the main scene could not be written in {MANIFEST_FILE_NAME}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str =
        "# Mine.\n[project]\nname = \"Demo\"\n\n[input.actions]\njump = [\"Space\"]\n";

    #[test]
    fn the_main_scene_is_added_at_the_end_of_the_project_table() {
        let text = with_main_scene(MANIFEST, "scenes/one.scene").unwrap();

        assert_eq!(
            text,
            "# Mine.\n[project]\nname = \"Demo\"\nmain_scene = \"scenes/one.scene\"\n\n\
             [input.actions]\njump = [\"Space\"]\n"
        );
    }

    #[test]
    fn a_new_main_scene_replaces_the_old_one() {
        let first = with_main_scene(MANIFEST, "one.scene").unwrap();

        let second = with_main_scene(&first, "two \"b\".scene").unwrap();

        assert_eq!(main_scene_of(&second).as_deref(), Some("two \"b\".scene"));
        assert_eq!(second.matches(MAIN_SCENE_KEY).count(), 1);
    }

    #[test]
    fn manifests_without_a_project_table_are_refused() {
        assert!(with_main_scene("[input]\n", "one.scene").is_err());
    }
}
