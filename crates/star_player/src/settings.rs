//! The parts of `project.toml` that change how scripts are compiled and
//! played: the input actions scripts name, and the scene a game without a
//! `Game` class plays. The editor's check reads them here too.

use std::collections::BTreeMap;

use serde::Deserialize;
use star_script::{GameSettings, InputAction, MainScene, ShowFps};

/// The parts of `project.toml` that change how scripts are compiled; the
/// rest of the file is read by the project's manifest.
#[derive(Debug, Default, Deserialize)]
struct ScriptSettingsFile {
    #[serde(default)]
    project: ProjectSection,
    #[serde(default)]
    input: InputSection,
}

/// `[project]`: only what scripts need of it.
#[derive(Debug, Default, Deserialize)]
struct ProjectSection {
    /// The project's name, the title of its window.
    #[serde(default)]
    name: String,
    /// The scene played when no class extends `Game`, in the assets folder.
    #[serde(default)]
    main_scene: Option<String>,
    /// When the frames per second are shown: `"editor"`, `"always"` or
    /// `"never"`.
    #[serde(default)]
    show_fps: Option<String>,
}

/// `[input]`
#[derive(Debug, Default, Deserialize)]
struct InputSection {
    /// `[input.actions]`: `jump = ["Space", "GamepadSouth"]`.
    #[serde(default)]
    actions: BTreeMap<String, Vec<String>>,
}

/// The settings scripts are compiled with, read from the text of
/// `project.toml`.
///
/// # Errors
///
/// Returns why the text is not valid TOML, or why `[input.actions]` is not
/// a table of lists of binding names.
pub fn read_game_settings(text: &str) -> Result<GameSettings, String> {
    let file: ScriptSettingsFile =
        toml::from_str(text).map_err(|error| error.message().to_owned())?;
    let actions = file
        .input
        .actions
        .into_iter()
        .map(|(name, bindings)| InputAction { name, bindings })
        .collect();
    let show_fps = match file.project.show_fps.as_deref() {
        None => ShowFps::default(),
        Some(name) => ShowFps::from_name(name).ok_or_else(|| {
            let choices: Vec<&str> = ShowFps::ALL.iter().map(|choice| choice.name()).collect();
            format!(
                "show_fps is one of \"{}\", not \"{name}\"",
                choices.join("\", \"")
            )
        })?,
    };
    let title = file.project.name;
    let main_scene = file
        .project
        .main_scene
        .map(|path| MainScene { path, title });
    Ok(GameSettings {
        actions,
        main_scene,
        show_fps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fps_are_shown_as_the_project_says() {
        let read = |text: &str| read_game_settings(text).map(|settings| settings.show_fps);

        assert_eq!(read("[project]\nname = \"Demo\"\n"), Ok(ShowFps::Editor));
        assert_eq!(
            read("[project]\nshow_fps = \"always\"\n"),
            Ok(ShowFps::Always)
        );
        let error = read("[project]\nshow_fps = \"often\"\n").unwrap_err();
        assert!(
            error.contains("\"never\"") && error.contains("often"),
            "{error}"
        );
    }

    #[test]
    fn the_main_scene_is_played_with_the_name_of_the_project() {
        let text = "[project]\nname = \"Demo\"\nmain_scene = \"scenes/one.scene\"\n";

        let settings = read_game_settings(text).expect("valid settings");

        assert_eq!(
            settings.main_scene,
            Some(MainScene {
                path: "scenes/one.scene".to_owned(),
                title: "Demo".to_owned(),
            })
        );
    }

    #[test]
    fn actions_are_read_with_their_bindings() {
        let text =
            "[project]\nname = \"Demo\"\n\n[input.actions]\njump = [\"Space\", \"GamepadSouth\"]\n";

        let settings = read_game_settings(text).expect("valid settings");

        assert_eq!(
            settings.actions,
            [InputAction {
                name: "jump".to_owned(),
                bindings: vec!["Space".to_owned(), "GamepadSouth".to_owned()],
            }]
        );
    }

    #[test]
    fn projects_without_actions_have_none() {
        let settings = read_game_settings("[project]\nname = \"Demo\"\n").expect("valid");

        assert_eq!(settings.actions, []);
    }

    #[test]
    fn actions_must_be_lists_of_names() {
        assert!(read_game_settings("[input.actions]\njump = \"Space\"\n").is_err());
        assert!(read_game_settings("[input]\nactions = 3\n").is_err());
    }
}
