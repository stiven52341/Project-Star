//! Description of what to draw in one frame.

use star_math::Mat4;

use crate::{Camera, Color, MeshId, Sprite};

/// One mesh to draw, placed in the world by `transform`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DrawCommand {
    pub(crate) mesh: MeshId,
    pub(crate) transform: Mat4,
}

/// Everything to draw in a frame: background, camera, meshes and sprites.
///
/// Games fill it every frame; the renderer turns it into GPU work.
/// Its storage is reused between frames to avoid allocations.
#[derive(Debug, Clone, Default)]
pub struct Frame {
    clear_color: Color,
    camera: Camera,
    draws: Vec<DrawCommand>,
    sprites: Vec<Sprite>,
}

impl Frame {
    /// Creates an empty frame with a black background and a default 2D camera.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts a new frame filled with `clear_color`, discarding the previous
    /// meshes and sprites. The camera is kept.
    pub fn begin(&mut self, clear_color: Color) {
        self.clear_color = clear_color;
        self.draws.clear();
        self.sprites.clear();
    }

    /// Sets the point of view used to draw the frame.
    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
    }

    /// Draws `mesh` placed in the world by `transform` (position, rotation, scale).
    pub fn draw(&mut self, mesh: MeshId, transform: Mat4) {
        self.draws.push(DrawCommand { mesh, transform });
    }

    /// Draws `sprite`. Sprites are drawn after the meshes, by layer and, in
    /// each layer, in the order they were added. Sprites in a row with the
    /// same texture are drawn together, so keeping sprites that share a
    /// texture together (for instance in a sprite sheet) draws faster.
    pub fn draw_sprite(&mut self, sprite: Sprite) {
        self.sprites.push(sprite);
    }

    /// Color the frame is filled with before drawing.
    #[must_use]
    pub fn clear_color(&self) -> Color {
        self.clear_color
    }

    /// Point of view used to draw the frame.
    #[must_use]
    pub fn camera(&self) -> &Camera {
        &self.camera
    }

    /// Number of meshes drawn in this frame.
    #[must_use]
    pub fn draw_count(&self) -> usize {
        self.draws.len()
    }

    /// Number of sprites drawn in this frame.
    #[must_use]
    pub fn sprite_count(&self) -> usize {
        self.sprites.len()
    }

    pub(crate) fn draws(&self) -> &[DrawCommand] {
        &self.draws
    }

    pub(crate) fn sprites(&self) -> &[Sprite] {
        &self.sprites
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn any_mesh_id() -> MeshId {
        MeshId(0)
    }

    fn any_sprite() -> Sprite {
        Sprite::new(
            crate::TextureId(0),
            star_math::Vec2::ZERO,
            star_math::Vec2::ONE,
        )
    }

    #[test]
    fn draw_records_a_command() {
        let mut frame = Frame::new();

        frame.draw(any_mesh_id(), Mat4::IDENTITY);

        assert_eq!(frame.draw_count(), 1);
    }

    #[test]
    fn draw_sprite_records_a_sprite() {
        let mut frame = Frame::new();

        frame.draw_sprite(any_sprite());

        assert_eq!(frame.sprite_count(), 1);
    }

    #[test]
    fn begin_discards_previous_draws_and_sets_background() {
        let mut frame = Frame::new();
        frame.draw(any_mesh_id(), Mat4::IDENTITY);
        frame.draw_sprite(any_sprite());

        frame.begin(Color::WHITE);

        assert_eq!(frame.draw_count(), 0);
        assert_eq!(frame.sprite_count(), 0);
        assert_eq!(frame.clear_color(), Color::WHITE);
    }
}
