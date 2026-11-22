//! The folder a game's assets are read from.

use std::path::{Path, PathBuf};

use star_core::{EngineError, EngineResult, Image, Sound};

use crate::{AssetPath, ImageFormat, SoundFormat};

/// Name of the asset folder of a game, next to its executable or its project.
pub const DEFAULT_ASSET_FOLDER: &str = "assets";

/// The folder with a game's images, sounds and scenes, from which assets are
/// read by [`AssetPath`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetFolder {
    root: PathBuf,
}

impl AssetFolder {
    /// The asset folder at `root`.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    /// The folder a game uses when it does not choose one:
    /// [`DEFAULT_ASSET_FOLDER`] next to the executable if it exists (a
    /// shipped game), otherwise in the working directory (during development).
    #[must_use]
    pub fn find_default() -> Self {
        let next_to_executable = std::env::current_exe()
            .ok()
            .and_then(|executable| Some(executable.parent()?.join(DEFAULT_ASSET_FOLDER)))
            .filter(|folder| folder.is_dir());
        Self::new(next_to_executable.unwrap_or_else(|| PathBuf::from(DEFAULT_ASSET_FOLDER)))
    }

    /// Where the folder is.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Where the file of the asset at `path` is.
    #[must_use]
    pub fn file_of(&self, path: &AssetPath) -> PathBuf {
        self.root.join(path.to_relative_path())
    }

    /// The bytes of the asset at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] if the file does not exist or cannot be read.
    pub fn read(&self, path: &AssetPath) -> EngineResult<Vec<u8>> {
        let file = self.file_of(path);
        std::fs::read(&file).map_err(|error| {
            EngineError::Asset(format!(
                "cannot read \"{path}\" ({}): {error}",
                file.display()
            ))
        })
    }

    /// The text of the asset at `path`, e.g. a scene file.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] if the file cannot be read or is not UTF-8 text.
    pub fn read_text(&self, path: &AssetPath) -> EngineResult<String> {
        String::from_utf8(self.read(path)?)
            .map_err(|_| EngineError::Asset(format!("\"{path}\" is not UTF-8 text")))
    }

    /// Reads and decodes the image at `path`, in the format of its extension.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] if the format is not supported, or the
    /// file cannot be read or is not a valid image.
    pub fn load_image(&self, path: &AssetPath) -> EngineResult<Image> {
        let format = ImageFormat::of(path)?;
        let bytes = self.read(path)?;
        format
            .decode(&bytes)
            .map_err(|error| EngineError::Asset(format!("\"{path}\": {error}")))
    }

    /// Reads and decodes the whole sound at `path`, in the format of its
    /// extension.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] if the format is not supported, or the
    /// file cannot be read or is not a valid sound.
    pub fn load_sound(&self, path: &AssetPath) -> EngineResult<Sound> {
        let format = SoundFormat::of(path)?;
        let bytes = self.read(path)?;
        format
            .decode(&bytes)
            .map_err(|error| EngineError::Asset(format!("\"{path}\": {error}")))
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::PathBuf;

    use super::AssetFolder;

    /// An empty asset folder in the temporary directory, deleted when dropped.
    pub(crate) struct TemporaryFolder(pub(crate) AssetFolder);

    impl TemporaryFolder {
        pub(crate) fn new(test_name: &str) -> Self {
            let root: PathBuf = std::env::temp_dir()
                .join(format!("star_assets_{test_name}_{}", std::process::id()));
            // Left over from an interrupted run.
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).unwrap();
            Self(AssetFolder::new(root))
        }

        /// Writes `bytes` to the file at `path`, creating its folders.
        pub(crate) fn write(&self, path: &str, bytes: &[u8]) {
            let file = self.0.root().join(path);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file, bytes).unwrap();
        }
    }

    impl Drop for TemporaryFolder {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(self.0.root());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::TemporaryFolder;
    use super::*;
    use crate::png_image::test_support::one_pixel_png;

    fn path(path: &str) -> AssetPath {
        AssetPath::new(path).unwrap()
    }

    #[test]
    fn assets_are_read_from_their_folder() {
        let folder = TemporaryFolder::new("read");
        folder.write("levels/one.txt", b"hello");

        assert_eq!(folder.0.read(&path("levels/one.txt")).unwrap(), b"hello");
    }

    #[test]
    fn text_assets_must_be_utf8() {
        let folder = TemporaryFolder::new("text");
        folder.write("level.scene", "name = \"Nivel\"".as_bytes());
        folder.write("binary.scene", &[0xFF, 0xFE]);

        assert_eq!(
            folder.0.read_text(&path("level.scene")).unwrap(),
            "name = \"Nivel\""
        );
        assert!(folder.0.read_text(&path("binary.scene")).is_err());
    }

    #[test]
    fn missing_assets_name_their_path_and_file() {
        let folder = TemporaryFolder::new("missing");

        let error = folder.0.read(&path("levels/none.txt")).unwrap_err();

        assert!(error.to_string().contains("\"levels/none.txt\""), "{error}");
        assert!(
            error
                .to_string()
                .contains(&folder.0.root().display().to_string()),
            "{error}"
        );
    }

    #[test]
    fn images_are_decoded_by_their_extension() {
        let folder = TemporaryFolder::new("image");
        folder.write("sprites/dot.png", &one_pixel_png([1, 2, 3, 4]));

        let image = folder.0.load_image(&path("sprites/dot.png")).unwrap();

        assert_eq!(image.pixels(), [1, 2, 3, 4]);
    }

    #[test]
    fn sounds_are_decoded_by_their_extension() {
        let folder = TemporaryFolder::new("sound");
        folder.write(
            "sounds/tone.wav",
            include_bytes!("../test_data/tone_mono.wav"),
        );

        let sound = folder.0.load_sound(&path("sounds/tone.wav")).unwrap();

        assert_eq!(sound.channels(), 1);
    }

    #[test]
    fn corrupt_sounds_name_their_path() {
        let folder = TemporaryFolder::new("corrupt_sound");
        folder.write("broken.ogg", b"not a sound");

        let error = folder.0.load_sound(&path("broken.ogg")).unwrap_err();

        assert!(error.to_string().contains("\"broken.ogg\""), "{error}");
    }

    #[test]
    fn corrupt_images_name_their_path() {
        let folder = TemporaryFolder::new("corrupt");
        folder.write("broken.png", b"not a png");

        let error = folder.0.load_image(&path("broken.png")).unwrap_err();

        assert!(error.to_string().contains("\"broken.png\""), "{error}");
    }

    #[test]
    fn the_default_folder_is_called_assets() {
        assert!(
            AssetFolder::find_default()
                .root()
                .ends_with(DEFAULT_ASSET_FOLDER)
        );
    }
}
