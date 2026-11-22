//! Well-known folders of the current user.

use std::path::PathBuf;

/// The user's home folder, if the system defines one.
#[must_use]
pub fn home_directory() -> Option<PathBuf> {
    let variable = if cfg!(target_os = "windows") {
        "USERPROFILE"
    } else {
        "HOME"
    };
    std::env::var_os(variable)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

/// Folder where applications store the user's settings:
/// `%APPDATA%` on Windows, `~/Library/Application Support` on macOS and
/// `$XDG_CONFIG_HOME` (or `~/.config`) elsewhere.
#[must_use]
pub fn config_directory() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        non_empty_variable("APPDATA")
    } else if cfg!(target_os = "macos") {
        home_directory().map(|home| home.join("Library").join("Application Support"))
    } else {
        non_empty_variable("XDG_CONFIG_HOME")
            .or_else(|| home_directory().map(|home| home.join(".config")))
    }
}

fn non_empty_variable(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_directory_is_absolute_when_known() {
        if let Some(directory) = config_directory() {
            assert!(directory.is_absolute(), "{}", directory.display());
        }
    }
}
