//! The behavior a game plugs into the engine.

use star_assets::{AssetFolder, AssetPath};
use star_core::EngineResult;
use star_ecs::World;
use star_render::{Frame, Image, MeshId, Renderer, TextureFilter, TextureId, Vertex};

use crate::texture_library::TextureLibrary;

/// Logic of a game, called by the engine at fixed points of the game loop.
///
/// Every method has an empty default, so games only implement what they need.
pub trait Game {
    /// Called once when the window and the GPU are ready.
    /// Create meshes, textures and the first entities here.
    ///
    /// # Errors
    ///
    /// Returning an error stops the application.
    fn start(&mut self, _context: &mut GameContext<'_>) -> EngineResult<()> {
        Ok(())
    }

    /// Advances the game simulation; called once per frame before drawing,
    /// after the systems of [`Stage::First`](star_ecs::Stage::First) and
    /// before the others. The time since the last frame and what the player
    /// is doing are resources of `world`: see [`EngineWorld`](crate::EngineWorld).
    fn update(&mut self, _world: &mut World) {}

    /// Describes what the current frame shows; called after every system ran.
    fn draw(&self, _world: &World, _frame: &mut Frame) {}

    /// Whether the game wants to close; checked after each frame. By default
    /// the game runs until the user closes the window.
    fn is_finished(&self) -> bool {
        false
    }
}

/// Engine services available while a game starts.
pub struct GameContext<'a> {
    renderer: &'a mut Renderer,
    world: &'a mut World,
    assets: &'a AssetFolder,
    textures: &'a mut TextureLibrary,
}

impl<'a> GameContext<'a> {
    pub(crate) fn new(
        renderer: &'a mut Renderer,
        world: &'a mut World,
        assets: &'a AssetFolder,
        textures: &'a mut TextureLibrary,
    ) -> Self {
        Self {
            renderer,
            world,
            assets,
            textures,
        }
    }

    /// The folder the game's assets are read from.
    #[must_use]
    pub fn assets(&self) -> &AssetFolder {
        self.assets
    }

    /// The entities, components and resources of the game.
    pub fn world(&mut self) -> &mut World {
        self.world
    }

    /// Uploads a triangle list to the GPU: every 3 vertices form a triangle.
    ///
    /// # Errors
    ///
    /// Returns an error if `vertices` is empty or not a multiple of 3.
    pub fn create_mesh(&mut self, vertices: &[Vertex]) -> EngineResult<MeshId> {
        self.renderer.create_mesh(vertices)
    }

    /// Uploads line segments to the GPU: every 2 vertices form a segment.
    ///
    /// # Errors
    ///
    /// Returns an error if `vertices` is empty or odd.
    pub fn create_lines(&mut self, vertices: &[Vertex]) -> EngineResult<MeshId> {
        self.renderer.create_lines(vertices)
    }

    /// Uploads `image` to the GPU, to be drawn with `filter`.
    ///
    /// # Errors
    ///
    /// Returns an error if the GPU does not support textures that big.
    pub fn create_texture(
        &mut self,
        image: &Image,
        filter: TextureFilter,
    ) -> EngineResult<TextureId> {
        self.renderer.create_texture(image, filter)
    }

    /// Loads the image at `path` in the asset folder, e.g. `"sprites/hero.png"`,
    /// and uploads it to the GPU. Loading the same file with the same
    /// filter again gives the same texture, without reading it again.
    ///
    /// # Errors
    ///
    /// Returns an error if `path` is not a valid [`AssetPath`], the file
    /// cannot be read or decoded, or the GPU cannot hold the image.
    pub fn load_texture(&mut self, path: &str, filter: TextureFilter) -> EngineResult<TextureId> {
        let Self {
            renderer,
            assets,
            textures,
            ..
        } = self;
        textures.load(AssetPath::new(path)?, filter, |path| {
            renderer.create_texture(&assets.load_image(path)?, filter)
        })
    }
}

/// Game that draws nothing; used when an [`App`](crate::app::App) runs without one.
pub(crate) struct EmptyGame;

impl Game for EmptyGame {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn games_run_until_the_window_is_closed_by_default() {
        assert!(!EmptyGame.is_finished());
    }
}
