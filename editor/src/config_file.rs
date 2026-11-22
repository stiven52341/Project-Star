//! The editor's own files in the user's configuration folder (recent
//! projects, preferences), written as TOML.
//!
//! Losing them is harmless, so read failures give the defaults and are only
//! logged instead of interrupting the user.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::Serialize;
use serde::de::DeserializeOwned;
use star_engine::prelude::ENGINE_NAME;
use star_engine::star_platform::user_directories::config_directory;

/// Where the editor keeps the file `name`, if the system has a
/// configuration folder.
pub(crate) fn config_file(name: &str) -> Option<PathBuf> {
    config_directory().map(|directory| directory.join(ENGINE_NAME).join(name))
}

/// What `file` holds, or the default value if it is missing or corrupt.
pub(crate) fn read_or_default<T: DeserializeOwned + Default>(file: &Path) -> T {
    match fs::read_to_string(file) {
        Ok(text) => toml::from_str(&text).unwrap_or_else(|error: toml::de::Error| {
            log::warn!("Ignoring corrupt {}: {}", file.display(), error.message());
            T::default()
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => T::default(),
        Err(error) => {
            log::warn!("Cannot read {}: {error}", file.display());
            T::default()
        }
    }
}

/// Writes `value` to `file`, creating its folder if needed.
pub(crate) fn write<T: Serialize>(value: &T, file: &Path) -> io::Result<()> {
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(file, toml::to_string(value).map_err(io::Error::other)?)
}
