//! The textures a game loaded from asset files.

use std::collections::HashMap;
use std::time::Duration;

use star_assets::{AssetFolder, AssetPath, FileWatcher};
use star_core::EngineResult;
use star_render::{TextureFilter, TextureId};

/// A texture made from an asset file: the same image drawn with another
/// filter is another texture on the GPU.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoadedTexture<Id> {
    pub(crate) path: AssetPath,
    pub(crate) filter: TextureFilter,
    pub(crate) id: Id,
}

/// Remembers which asset files are already on the GPU, so loading one twice
/// gives the same texture instead of decoding and uploading it again. With
/// hot-reload, it also watches their files to upload them again when they
/// change.
///
/// Generic over the texture ids only so the tests need no GPU.
pub(crate) struct TextureLibrary<Id = TextureId> {
    loaded: HashMap<(AssetPath, TextureFilter), Id>,
    watcher: Option<FileWatcher<LoadedTexture<Id>>>,
}

impl<Id: Copy> TextureLibrary<Id> {
    /// An empty library; `hot_reload` decides whether files are watched.
    pub(crate) fn new(hot_reload: bool) -> Self {
        Self {
            loaded: HashMap::new(),
            watcher: hot_reload.then(FileWatcher::default),
        }
    }

    /// The texture of the file at `path` in `folder` with `filter`; `upload`
    /// creates it the first time.
    pub(crate) fn load(
        &mut self,
        folder: &AssetFolder,
        path: AssetPath,
        filter: TextureFilter,
        upload: impl FnOnce(&AssetPath) -> EngineResult<Id>,
    ) -> EngineResult<Id> {
        let key = (path, filter);
        if let Some(&id) = self.loaded.get(&key) {
            return Ok(id);
        }
        let id = upload(&key.0)?;
        // Packaged assets have no file to watch.
        if let (Some(watcher), Some(file)) = (&mut self.watcher, folder.file_of(&key.0)) {
            let texture = LoadedTexture {
                path: key.0.clone(),
                filter,
                id,
            };
            watcher.watch(texture, file);
        }
        self.loaded.insert(key, id);
        Ok(id)
    }

    /// Lets `elapsed` time pass, and calls `reload` with every texture whose
    /// file changed since it was loaded or last reloaded. Does nothing
    /// without hot-reload.
    pub(crate) fn reload_changed(
        &mut self,
        elapsed: Duration,
        reload: impl FnMut(&LoadedTexture<Id>),
    ) {
        if let Some(watcher) = &mut self.watcher {
            watcher.poll(elapsed, reload);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::time::SystemTime;

    use star_assets::DEFAULT_CHECK_INTERVAL;
    use star_core::EngineError;

    use super::*;

    fn path(path: &str) -> AssetPath {
        AssetPath::new(path).unwrap()
    }

    fn folder() -> AssetFolder {
        AssetFolder::new("assets")
    }

    #[test]
    fn each_file_is_uploaded_once() {
        let mut library = TextureLibrary::new(false);
        let mut uploads = 0;
        let mut upload = |_: &AssetPath| {
            uploads += 1;
            Ok(uploads)
        };

        let first = library.load(
            &folder(),
            path("hero.png"),
            TextureFilter::Smooth,
            &mut upload,
        );
        let again = library.load(
            &folder(),
            path("hero.png"),
            TextureFilter::Smooth,
            &mut upload,
        );

        assert_eq!(first.unwrap(), 1);
        assert_eq!(again.unwrap(), 1);
        assert_eq!(uploads, 1);
    }

    #[test]
    fn other_files_and_filters_are_other_textures() {
        let mut library = TextureLibrary::new(false);
        let mut uploads = 0;
        let mut upload = |_: &AssetPath| {
            uploads += 1;
            Ok(uploads)
        };
        let mut load = |name, filter| library.load(&folder(), path(name), filter, &mut upload);

        load("hero.png", TextureFilter::Smooth).unwrap();
        let pixelated = load("hero.png", TextureFilter::Pixelated);
        let other = load("enemy.png", TextureFilter::Smooth);

        assert_eq!(pixelated.unwrap(), 2);
        assert_eq!(other.unwrap(), 3);
    }

    #[test]
    fn failed_loads_are_retried() {
        let mut library = TextureLibrary::new(false);

        let failed = library.load(&folder(), path("hero.png"), TextureFilter::Smooth, |_| {
            Err(EngineError::Asset("missing".to_owned()))
        });
        let retried = library.load(&folder(), path("hero.png"), TextureFilter::Smooth, |_| {
            Ok(7)
        });

        assert!(failed.is_err());
        assert_eq!(retried.unwrap(), 7);
    }

    #[test]
    fn with_hot_reload_changed_files_are_reloaded() {
        let root = std::env::temp_dir().join(format!("star_engine_reload_{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("hero.png");
        std::fs::write(&file, b"image").unwrap();
        let folder = AssetFolder::new(&root);
        let mut library = TextureLibrary::new(true);
        library
            .load(&folder, path("hero.png"), TextureFilter::Pixelated, |_| {
                Ok(3)
            })
            .unwrap();

        File::options()
            .write(true)
            .open(&file)
            .unwrap()
            .set_modified(SystemTime::UNIX_EPOCH)
            .unwrap();
        let mut reloaded = Vec::new();
        library.reload_changed(DEFAULT_CHECK_INTERVAL, |texture| {
            reloaded.push(texture.clone());
        });
        std::fs::remove_dir_all(&root).unwrap();

        assert_eq!(
            reloaded,
            [LoadedTexture {
                path: path("hero.png"),
                filter: TextureFilter::Pixelated,
                id: 3,
            }]
        );
    }

    #[test]
    fn without_hot_reload_nothing_is_watched() {
        let mut library = TextureLibrary::new(false);
        library
            .load(&folder(), path("hero.png"), TextureFilter::Smooth, |_| {
                Ok(1)
            })
            .unwrap();

        library.reload_changed(DEFAULT_CHECK_INTERVAL, |_| panic!("nothing is watched"));
    }
}
