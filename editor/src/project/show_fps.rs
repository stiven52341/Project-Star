//! When the project's game shows its frames per second (decision 85):
//! `show_fps` in the `[project]` table of `project.toml`, chosen in the
//! Project menu.

use std::fs;
use std::io;
use std::path::Path;

use serde::Deserialize;
use star_script::ShowFps;

use super::MANIFEST_FILE_NAME;
use super::project_table::{invalid, with_project_key};

/// The key of the choice.
const SHOW_FPS_KEY: &str = "show_fps";

/// The parts of `project.toml` read here.
#[derive(Debug, Default, Deserialize)]
struct ShowFpsFile {
    #[serde(default)]
    project: ShowFpsSection,
}

/// `[project]`
#[derive(Debug, Default, Deserialize)]
struct ShowFpsSection {
    #[serde(default)]
    show_fps: Option<String>,
}

/// When the game of the project in `directory` shows its frames per
/// second; the default if `project.toml` does not say, or cannot be read.
pub(crate) fn read_show_fps(directory: &Path) -> ShowFps {
    fs::read_to_string(directory.join(MANIFEST_FILE_NAME))
        .ok()
        .and_then(|text| show_fps_of(&text))
        .unwrap_or_default()
}

/// Makes the game of the project in `directory` show its frames per
/// second as `choice` says.
///
/// # Errors
///
/// Fails if `project.toml` cannot be read or written, or has no
/// `[project]` table to write in (it then stays as it was).
pub(crate) fn set_show_fps(directory: &Path, choice: ShowFps) -> io::Result<()> {
    let file = directory.join(MANIFEST_FILE_NAME);
    let text = fs::read_to_string(&file)?;
    fs::write(&file, with_show_fps(&text, choice)?)
}

fn show_fps_of(text: &str) -> Option<ShowFps> {
    let file: ShowFpsFile = toml::from_str(text).ok()?;
    ShowFps::from_name(&file.project.show_fps?)
}

fn with_show_fps(text: &str, choice: ShowFps) -> io::Result<String> {
    let value = toml::Value::String(choice.name().to_owned());
    let new_text = with_project_key(text, SHOW_FPS_KEY, &value)?;
    if show_fps_of(&new_text) == Some(choice) {
        Ok(new_text)
    } else {
        Err(invalid(format!(
            "{SHOW_FPS_KEY} could not be written in {MANIFEST_FILE_NAME}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = "# Mine.\n[project]\nname = \"Demo\"\n\n[input.actions]\n";

    #[test]
    fn the_choice_is_written_into_the_project_table() {
        let always = with_show_fps(MANIFEST, ShowFps::Always).unwrap();
        let never = with_show_fps(&always, ShowFps::Never).unwrap();

        assert_eq!(
            always,
            "# Mine.\n[project]\nname = \"Demo\"\nshow_fps = \"always\"\n\n[input.actions]\n"
        );
        assert_eq!(show_fps_of(&never), Some(ShowFps::Never));
        assert_eq!(never.matches(SHOW_FPS_KEY).count(), 1);
    }

    #[test]
    fn projects_that_do_not_say_show_them_when_played_from_the_editor() {
        let directory = crate::project::TestDirectory::new("show_fps_default");

        assert_eq!(read_show_fps(directory.path()), ShowFps::Editor);
        assert_eq!(show_fps_of(MANIFEST), None);
    }
}
