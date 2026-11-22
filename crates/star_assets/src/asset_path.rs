//! Names of assets inside a game's asset folder.

use std::fmt;
use std::path::PathBuf;

use star_core::{EngineError, EngineResult};

/// Separates the folders of an [`AssetPath`] on every platform.
const SEPARATOR: char = '/';

/// Where an asset is inside the asset folder, e.g. `"sprites/hero.png"`.
///
/// Asset paths are the same on every platform: relative, with `/` between
/// folders, and never leaving the asset folder. That keeps a game working
/// after it is copied to another system, where `C:\…` or `../` would not.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AssetPath(String);

impl AssetPath {
    /// Checks that `path` is a valid asset path.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] if `path` is empty, absolute, uses `\`
    /// or `:`, has empty folder names, or has `.` or `..` in it.
    pub fn new(path: &str) -> EngineResult<Self> {
        validate(path).map_err(|reason| {
            EngineError::Asset(format!("\"{path}\" is not a valid asset path: {reason}"))
        })?;
        Ok(Self(path.to_owned()))
    }

    /// The path as written, e.g. `"sprites/hero.png"`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The extension of the file name, e.g. `"png"`; `None` without one.
    #[must_use]
    pub fn extension(&self) -> Option<&str> {
        let file_name = self.0.rsplit(SEPARATOR).next()?;
        match file_name.rsplit_once('.') {
            Some((stem, extension)) if !stem.is_empty() && !extension.is_empty() => Some(extension),
            _ => None,
        }
    }

    /// The path with the separators of the current platform, relative to
    /// the asset folder.
    pub(crate) fn to_relative_path(&self) -> PathBuf {
        self.0.split(SEPARATOR).collect()
    }
}

impl fmt::Display for AssetPath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl TryFrom<&str> for AssetPath {
    type Error = EngineError;

    fn try_from(path: &str) -> EngineResult<Self> {
        Self::new(path)
    }
}

/// Why `path` is not a valid asset path, if it is not.
fn validate(path: &str) -> Result<(), &'static str> {
    if path.is_empty() {
        return Err("it is empty");
    }
    if path.starts_with(SEPARATOR) {
        return Err("it must be relative to the asset folder");
    }
    if path.contains('\\') {
        return Err("folders are separated with `/`");
    }
    if path.contains(':') {
        return Err("it cannot contain `:`");
    }
    for folder in path.split(SEPARATOR) {
        match folder {
            "" => return Err("it has an empty folder name"),
            "." | ".." => return Err("it cannot contain `.` or `..`"),
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_paths_with_folders_are_valid() {
        let path = AssetPath::new("sprites/heroes/knight.png").unwrap();

        assert_eq!(path.as_str(), "sprites/heroes/knight.png");
        assert_eq!(path.to_string(), "sprites/heroes/knight.png");
    }

    #[test]
    fn paths_that_could_leave_the_folder_or_break_elsewhere_are_rejected() {
        for invalid in [
            "",
            "/etc/passwd",
            "../secret.png",
            "sprites/../../secret.png",
            "./hero.png",
            "sprites//hero.png",
            "sprites/",
            "sprites\\hero.png",
            "C:hero.png",
        ] {
            let error = AssetPath::new(invalid).unwrap_err();

            assert!(matches!(error, EngineError::Asset(_)), "{invalid}: {error}");
        }
    }

    #[test]
    fn the_extension_is_that_of_the_file_name() {
        let extension = |path| AssetPath::new(path).unwrap().extension().map(str::to_owned);

        assert_eq!(extension("hero.png").as_deref(), Some("png"));
        assert_eq!(extension("maps.v2/level.tar.gz").as_deref(), Some("gz"));
        assert_eq!(extension("maps.v2/readme"), None);
        assert_eq!(extension(".hidden"), None);
        assert_eq!(extension("trailing."), None);
    }

    #[test]
    fn the_relative_path_uses_the_platform_separators() {
        let path = AssetPath::new("sprites/hero.png").unwrap();

        assert_eq!(
            path.to_relative_path(),
            PathBuf::from("sprites").join("hero.png")
        );
    }
}
