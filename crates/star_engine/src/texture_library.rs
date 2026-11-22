//! The textures a game loaded from asset files.

use std::collections::HashMap;

use star_assets::AssetPath;
use star_core::EngineResult;
use star_render::{TextureFilter, TextureId};

/// Names one loaded texture: the same image drawn with another filter is
/// another texture on the GPU.
type TextureKey = (AssetPath, TextureFilter);

/// Remembers which asset files are already on the GPU, so loading one twice
/// gives the same texture instead of decoding and uploading it again.
///
/// Generic over the texture ids only so the tests need no GPU.
pub(crate) struct TextureLibrary<Id = TextureId> {
    loaded: HashMap<TextureKey, Id>,
}

impl<Id: Copy> TextureLibrary<Id> {
    pub(crate) fn new() -> Self {
        Self {
            loaded: HashMap::new(),
        }
    }

    /// The texture of `path` with `filter`; `upload` creates it the first time.
    pub(crate) fn load(
        &mut self,
        path: AssetPath,
        filter: TextureFilter,
        upload: impl FnOnce(&AssetPath) -> EngineResult<Id>,
    ) -> EngineResult<Id> {
        let key = (path, filter);
        if let Some(&id) = self.loaded.get(&key) {
            return Ok(id);
        }
        let id = upload(&key.0)?;
        self.loaded.insert(key, id);
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use star_core::EngineError;

    use super::*;

    fn path(path: &str) -> AssetPath {
        AssetPath::new(path).unwrap()
    }

    #[test]
    fn each_file_is_uploaded_once() {
        let mut library = TextureLibrary::new();
        let mut uploads = 0;
        let mut upload = |_: &AssetPath| {
            uploads += 1;
            Ok(uploads)
        };

        let first = library.load(path("hero.png"), TextureFilter::Smooth, &mut upload);
        let again = library.load(path("hero.png"), TextureFilter::Smooth, &mut upload);

        assert_eq!(first.unwrap(), 1);
        assert_eq!(again.unwrap(), 1);
        assert_eq!(uploads, 1);
    }

    #[test]
    fn other_files_and_filters_are_other_textures() {
        let mut library = TextureLibrary::new();
        let mut uploads = 0;
        let mut upload = |_: &AssetPath| {
            uploads += 1;
            Ok(uploads)
        };

        library
            .load(path("hero.png"), TextureFilter::Smooth, &mut upload)
            .unwrap();
        let pixelated = library.load(path("hero.png"), TextureFilter::Pixelated, &mut upload);
        let other = library.load(path("enemy.png"), TextureFilter::Smooth, &mut upload);

        assert_eq!(pixelated.unwrap(), 2);
        assert_eq!(other.unwrap(), 3);
    }

    #[test]
    fn failed_loads_are_retried() {
        let mut library = TextureLibrary::new();

        let failed = library.load(path("hero.png"), TextureFilter::Smooth, |_| {
            Err(EngineError::Asset("missing".to_owned()))
        });
        let retried = library.load(path("hero.png"), TextureFilter::Smooth, |_| Ok(7));

        assert!(failed.is_err());
        assert_eq!(retried.unwrap(), 7);
    }
}
