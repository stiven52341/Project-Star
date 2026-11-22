//! The input actions of `project.toml`, which scripts name.

use std::collections::BTreeMap;

use serde::Deserialize;
use star_script::{GameSettings, InputAction};

/// The parts of `project.toml` that change how scripts are compiled; the
/// rest of the file is read by the project's manifest.
#[derive(Debug, Default, Deserialize)]
struct ScriptSettingsFile {
    #[serde(default)]
    input: InputSection,
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
pub(super) fn read_game_settings(text: &str) -> Result<GameSettings, String> {
    let file: ScriptSettingsFile =
        toml::from_str(text).map_err(|error| error.message().to_owned())?;
    let actions = file
        .input
        .actions
        .into_iter()
        .map(|(name, bindings)| InputAction { name, bindings })
        .collect();
    Ok(GameSettings { actions })
}

#[cfg(test)]
mod tests {
    use super::*;

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
