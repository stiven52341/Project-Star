//! Sprites whose image is a file, loaded by the engine when needed.

use star_assets::AssetPath;
use star_core::EngineResult;
use star_math::Vec2;
use star_render::{Color, SpriteSheet, TextureFilter, TextureId};

use crate::texture_slot::TextureSlot;

/// Shows an image of the asset folder at a 2D node, like a
/// [`SpriteNode`](star_scene::SpriteNode), but named by its file: the engine
/// loads the texture the first time it draws it, and again whenever `path`
/// or `filter` change. So code without access to the renderer, such as
/// systems, can give nodes images.
#[derive(Debug, Clone, PartialEq)]
pub struct SpriteFile {
    /// The image, in the asset folder.
    pub path: AssetPath,
    /// How the image is sampled.
    pub filter: TextureFilter,
    /// Width and height in world units, before the node's scale.
    pub size: Vec2,
    /// Sprites in higher layers are drawn on top of lower ones.
    pub layer: i32,
    /// Multiplied by the image's colors.
    pub color: Color,
    /// How the image is cut into cells, and the one shown.
    pub sheet: SpriteSheet,
    loaded: TextureSlot,
}

impl SpriteFile {
    /// The image at `path`, `size` world units big, smooth, with its own
    /// colors, in layer 0.
    #[must_use]
    pub const fn new(path: AssetPath, size: Vec2) -> Self {
        Self {
            path,
            filter: TextureFilter::Smooth,
            size,
            layer: 0,
            color: Color::WHITE,
            sheet: SpriteSheet::SINGLE,
            loaded: TextureSlot::EMPTY,
        }
    }

    /// The same sprite sampled with `filter`.
    #[must_use]
    pub fn with_filter(mut self, filter: TextureFilter) -> Self {
        self.filter = filter;
        self
    }

    /// The same sprite in `layer`.
    #[must_use]
    pub fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    /// The same sprite tinted with `color`.
    #[must_use]
    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// The same sprite showing a cell of a sprite `sheet`.
    #[must_use]
    pub fn with_sheet(mut self, sheet: SpriteSheet) -> Self {
        self.sheet = sheet;
        self
    }

    /// The texture to draw: the one already loaded for the current path and
    /// filter, or the one `load` gives now. `None` while the image cannot be
    /// loaded; the error is passed to `on_error` only the first time.
    pub(crate) fn texture(
        &mut self,
        load: impl FnOnce(&AssetPath, TextureFilter) -> EngineResult<TextureId>,
        on_error: impl FnOnce(&star_core::EngineError),
    ) -> Option<TextureId> {
        self.loaded.texture(&self.path, self.filter, load, on_error)
    }
}

#[cfg(test)]
mod tests {
    use star_core::EngineError;

    use super::*;

    fn sprite(path: &str) -> SpriteFile {
        SpriteFile::new(AssetPath::new(path).unwrap(), Vec2::ONE)
    }

    fn loaded(index: usize) -> impl FnOnce(&AssetPath, TextureFilter) -> EngineResult<TextureId> {
        move |_, _| Ok(TextureId::from_index(index))
    }

    fn unexpected() -> impl FnOnce(&AssetPath, TextureFilter) -> EngineResult<TextureId> {
        |path, _| panic!("{path} should not be loaded again")
    }

    #[test]
    fn the_image_is_loaded_once() {
        let mut sprite = sprite("hero.png");

        assert_eq!(
            sprite.texture(loaded(4), |_| {}),
            Some(TextureId::from_index(4))
        );
        assert_eq!(
            sprite.texture(unexpected(), |_| {}),
            Some(TextureId::from_index(4))
        );
    }

    #[test]
    fn changing_the_path_or_the_filter_loads_it_again() {
        let mut sprite = sprite("hero.png");
        sprite.texture(loaded(1), |_| {});

        sprite.path = AssetPath::new("enemy.png").unwrap();
        assert_eq!(
            sprite.texture(loaded(2), |_| {}),
            Some(TextureId::from_index(2))
        );
        sprite.filter = TextureFilter::Pixelated;
        assert_eq!(
            sprite.texture(loaded(3), |_| {}),
            Some(TextureId::from_index(3))
        );
    }

    #[test]
    fn missing_images_are_reported_once_and_not_retried() {
        let mut sprite = sprite("missing.png");
        let mut errors = 0;

        let first = sprite.texture(
            |_, _| Err(EngineError::Asset("missing".to_owned())),
            |_| errors += 1,
        );
        let second = sprite.texture(unexpected(), |_| errors += 1);

        assert_eq!((first, second, errors), (None, None, 1));
    }
}
