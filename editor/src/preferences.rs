//! The user's preferences for the editor, remembered between sessions:
//! for now, the language it is shown in.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config_file::{config_file, read_or_default, write};
use crate::texts::{self, Language};

const PREFERENCES_FILE_NAME: &str = "preferences.toml";

/// What the user chose. Missing entries take their defaults, so files
/// written by older editors still load.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    language: Language,
}

/// The preferences, kept in sync with their file and applied to the editor.
pub(crate) struct PreferencesStore {
    file: Option<PathBuf>,
    preferences: Preferences,
}

impl PreferencesStore {
    /// Loads the preferences saved in the user's configuration folder and
    /// applies them.
    pub(crate) fn load() -> Self {
        let store = Self::load_from(config_file(PREFERENCES_FILE_NAME));
        texts::set_language(store.language());
        store
    }

    fn load_from(file: Option<PathBuf>) -> Self {
        let preferences = file.as_deref().map(read_or_default).unwrap_or_default();
        Self { file, preferences }
    }

    pub(crate) const fn language(&self) -> Language {
        self.preferences.language
    }

    /// Shows the editor in `language` from now on, also in later sessions.
    pub(crate) fn set_language(&mut self, language: Language) {
        self.preferences.language = language;
        texts::set_language(language);
        self.save();
    }

    fn save(&self) {
        let Some(file) = &self.file else {
            return;
        };
        if let Err(error) = write(&self.preferences, file) {
            log::warn!("Cannot save the editor preferences: {error}");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::project::TestDirectory;

    #[test]
    fn english_is_used_until_another_language_is_chosen() {
        let folder = TestDirectory::new("preferences_default");

        let store = PreferencesStore::load_from(Some(folder.path().join(PREFERENCES_FILE_NAME)));

        assert_eq!(store.language(), Language::English);
    }

    #[test]
    fn the_language_chosen_is_remembered() {
        let folder = TestDirectory::new("preferences_language");
        let file = folder.path().join(PREFERENCES_FILE_NAME);
        let mut store = PreferencesStore::load_from(Some(file.clone()));

        store.preferences.language = Language::Spanish;
        store.save();
        let reloaded = PreferencesStore::load_from(Some(file.clone()));

        assert_eq!(reloaded.language(), Language::Spanish);
        let text = fs::read_to_string(file).expect("written");
        assert_eq!(text.trim(), "language = \"es\"");
    }

    #[test]
    fn unknown_languages_fall_back_to_english() {
        let folder = TestDirectory::new("preferences_unknown");
        let file = folder.path().join(PREFERENCES_FILE_NAME);
        fs::create_dir_all(folder.path()).expect("folder");
        fs::write(&file, "language = \"tlh\"").expect("written");

        let store = PreferencesStore::load_from(Some(file));

        assert_eq!(store.language(), Language::English);
    }
}
