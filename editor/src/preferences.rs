//! The user's preferences for the editor, remembered between sessions:
//! the language it is shown in and the size of the icons of “Assets”.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::config_file::{config_file, read_or_default, write};
use crate::texts::{self, Language};
use crate::workspace::{DEFAULT_ICON_SIZE, ICON_SIZES};

const PREFERENCES_FILE_NAME: &str = "preferences.toml";

/// What the user chose. Missing entries take their defaults, so files
/// written by older editors still load.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
struct Preferences {
    language: Language,
    /// The size of the icons of “Assets”, in percent.
    asset_icon_size: u16,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            language: Language::default(),
            asset_icon_size: DEFAULT_ICON_SIZE,
        }
    }
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

    /// The size of the icons of “Assets”, in percent, within the sizes the
    /// panel offers even if the file says otherwise.
    pub(crate) fn asset_icon_size(&self) -> u16 {
        self.preferences
            .asset_icon_size
            .clamp(*ICON_SIZES.start(), *ICON_SIZES.end())
    }

    /// Shows the icons of “Assets” at `size` percent, also in later
    /// sessions.
    pub(crate) fn set_asset_icon_size(&mut self, size: u16) {
        if self.preferences.asset_icon_size != size {
            self.preferences.asset_icon_size = size;
            self.save();
        }
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
        assert!(text.contains("language = \"es\""), "{text}");
    }

    #[test]
    fn the_icon_size_chosen_is_remembered_within_its_range() {
        let folder = TestDirectory::new("preferences_icon_size");
        let file = folder.path().join(PREFERENCES_FILE_NAME);
        let mut store = PreferencesStore::load_from(Some(file.clone()));
        assert_eq!(store.asset_icon_size(), DEFAULT_ICON_SIZE);

        store.set_asset_icon_size(150);
        assert_eq!(
            PreferencesStore::load_from(Some(file.clone())).asset_icon_size(),
            150
        );

        fs::write(&file, "asset_icon_size = 5000").expect("written");
        assert_eq!(
            PreferencesStore::load_from(Some(file)).asset_icon_size(),
            *ICON_SIZES.end()
        );
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
