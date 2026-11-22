//! The behavior a game plugs into the engine.

use star_core::{EngineResult, Time};
use star_input::Input;
use star_render::{Frame, MeshId, Renderer, Vertex};

/// Logic of a game, called by the engine at fixed points of the game loop.
///
/// Every method has an empty default, so games only implement what they need.
pub trait Game {
    /// Called once when the window and the GPU are ready.
    /// Create meshes and other GPU resources here.
    ///
    /// # Errors
    ///
    /// Returning an error stops the application.
    fn start(&mut self, _context: &mut GameContext<'_>) -> EngineResult<()> {
        Ok(())
    }

    /// Advances the game simulation; called once per frame before drawing,
    /// with the time since the last frame and what the player is doing.
    fn update(&mut self, _time: &Time, _input: &Input) {}

    /// Describes what the current frame shows.
    fn draw(&self, _frame: &mut Frame) {}

    /// Whether the game wants to close; checked after each frame. By default
    /// the game runs until the user closes the window.
    fn is_finished(&self) -> bool {
        false
    }
}

/// Engine services available while a game starts.
pub struct GameContext<'a> {
    renderer: &'a mut Renderer,
}

impl<'a> GameContext<'a> {
    pub(crate) fn new(renderer: &'a mut Renderer) -> Self {
        Self { renderer }
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
