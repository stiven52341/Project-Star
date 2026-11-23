//! The asset files packaged with the game, on systems that package them.

/// The asset files inside the game's package: the `assets` folder of an
/// Android APK. Other systems keep assets in a folder on disk, so they
/// have none.
#[derive(Debug, Clone)]
pub struct PackagedAssets {
    #[cfg(target_os = "android")]
    app: crate::android::AndroidApp,
    #[cfg(not(target_os = "android"))]
    _none: (),
}

impl PackagedAssets {
    /// The assets packaged with this game; `None` on systems that do not
    /// package them, or before Android started the game.
    #[must_use]
    pub fn of_this_game() -> Option<Self> {
        #[cfg(target_os = "android")]
        {
            crate::android::app().map(|app| Self { app })
        }
        #[cfg(not(target_os = "android"))]
        {
            None
        }
    }

    /// The bytes of the packaged file at `path`, relative to the package's
    /// asset folder, with `/` between its parts.
    ///
    /// # Errors
    ///
    /// Returns [`std::io::ErrorKind::NotFound`] if the package has no such
    /// file, and the system's error if it cannot be read.
    pub fn read(&self, path: &str) -> std::io::Result<Vec<u8>> {
        #[cfg(target_os = "android")]
        {
            use std::io::Read as _;
            let name = std::ffi::CString::new(path)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidInput, error))?;
            let mut asset = self
                .app
                .asset_manager()
                .open(&name)
                .ok_or_else(|| std::io::Error::from(std::io::ErrorKind::NotFound))?;
            let mut bytes = Vec::new();
            asset.read_to_end(&mut bytes)?;
            Ok(bytes)
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = (self, path);
            Err(std::io::Error::from(std::io::ErrorKind::NotFound))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(not(target_os = "android"))]
    #[test]
    fn desktop_games_package_no_assets() {
        assert!(PackagedAssets::of_this_game().is_none());
    }
}
