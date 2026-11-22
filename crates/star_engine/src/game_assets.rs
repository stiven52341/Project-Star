//! The assets of a running game: where they are read from and what is loaded.

use std::time::Duration;

use star_assets::{AssetFolder, AssetPath};
use star_core::EngineResult;
use star_render::{Renderer, TextureFilter, TextureId};
use star_scene::Scene;

use crate::texture_library::TextureLibrary;

/// The asset folder of a game and the textures loaded from it.
pub(crate) struct GameAssets {
    folder: AssetFolder,
    textures: TextureLibrary,
}

impl GameAssets {
    /// The assets of `folder`, watched for changes if `hot_reload`.
    pub(crate) fn new(folder: AssetFolder, hot_reload: bool) -> Self {
        Self {
            folder,
            textures: TextureLibrary::new(hot_reload),
        }
    }

    pub(crate) fn folder(&self) -> &AssetFolder {
        &self.folder
    }

    /// The texture of the image at `path`, loaded and uploaded the first time.
    pub(crate) fn load_texture(
        &mut self,
        renderer: &mut Renderer,
        path: &str,
        filter: TextureFilter,
    ) -> EngineResult<TextureId> {
        let folder = &self.folder;
        self.textures
            .load(folder, AssetPath::new(path)?, filter, |path| {
                renderer.create_texture(&folder.load_image(path)?, filter)
            })
    }

    /// Reads the scene file at `path`, loading the textures its sprites show.
    pub(crate) fn load_scene(
        &mut self,
        renderer: &mut Renderer,
        path: &str,
    ) -> EngineResult<Scene> {
        let path = AssetPath::new(path)?;
        let text = self.folder.read_text(&path)?;
        Scene::from_text(&text, |texture: &AssetPath, filter| {
            self.load_texture(renderer, texture.as_str(), filter)
        })
        .map_err(|error| star_core::EngineError::Scene(format!("\"{path}\": {error}")))
    }

    /// Uploads again the textures whose files changed. A file that cannot be
    /// loaded, e.g. one an image editor is still writing, only logs a warning:
    /// the texture keeps its last image until the file changes again.
    pub(crate) fn reload_changed_textures(&mut self, renderer: &mut Renderer, elapsed: Duration) {
        let folder = &self.folder;
        self.textures.reload_changed(elapsed, |texture| {
            let reloaded = folder
                .load_image(&texture.path)
                .and_then(|image| renderer.replace_texture(texture.id, &image, texture.filter));
            match reloaded {
                Ok(()) => log::info!("Reloaded \"{}\"", texture.path),
                Err(error) => log::warn!("Cannot reload \"{}\": {error}", texture.path),
            }
        });
    }
}
