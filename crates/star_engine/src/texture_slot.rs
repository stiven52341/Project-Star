//! The texture of a component that names its image by file.

use star_assets::AssetPath;
use star_core::{EngineError, EngineResult};
use star_render::{TextureFilter, TextureId};

/// The image a component last loaded, or tried to: it is loaded again only
/// when the component names another file or filter.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct TextureSlot {
    loaded: Option<LoadedImage>,
}

#[derive(Debug, Clone, PartialEq)]
struct LoadedImage {
    path: AssetPath,
    filter: TextureFilter,
    /// `None` if it could not be loaded; it is not tried again until the
    /// path or the filter change.
    texture: Option<TextureId>,
}

impl TextureSlot {
    /// Nothing loaded yet.
    pub(crate) const EMPTY: Self = Self { loaded: None };

    /// The texture of the image at `path` sampled with `filter`: the one
    /// already loaded for them, or the one `load` gives now. `None` while
    /// the image cannot be loaded; the error is passed to `on_error` only
    /// the first time.
    pub(crate) fn texture(
        &mut self,
        path: &AssetPath,
        filter: TextureFilter,
        load: impl FnOnce(&AssetPath, TextureFilter) -> EngineResult<TextureId>,
        on_error: impl FnOnce(&EngineError),
    ) -> Option<TextureId> {
        if let Some(loaded) = &self.loaded
            && loaded.path == *path
            && loaded.filter == filter
        {
            return loaded.texture;
        }
        let texture = load(path, filter).inspect_err(|error| on_error(error)).ok();
        self.loaded = Some(LoadedImage {
            path: path.clone(),
            filter,
            texture,
        });
        texture
    }
}
