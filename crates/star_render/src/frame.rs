//! Description of what to draw in one frame.

use star_math::Mat4;

use crate::{Camera, Color, MeshId};

/// One mesh to draw, placed in the world by `transform`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DrawCommand {
    pub(crate) mesh: MeshId,
    pub(crate) transform: Mat4,
}

/// Everything to draw in a frame: background, camera and meshes.
///
/// Games fill it every frame; the renderer turns it into GPU work.
/// Its storage is reused between frames to avoid allocations.
#[derive(Debug, Clone, Default)]
pub struct Frame {
    clear_color: Color,
    camera: Camera,
    draws: Vec<DrawCommand>,
}

impl Frame {
    /// Creates an empty frame with a black background and a default 2D camera.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts a new frame filled with `clear_color`, discarding the previous
    /// draw commands. The camera is kept.
    pub fn begin(&mut self, clear_color: Color) {
        self.clear_color = clear_color;
        self.draws.clear();
    }

    /// Sets the point of view used to draw the frame.
    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
    }

    /// Draws `mesh` placed in the world by `transform` (position, rotation, scale).
    pub fn draw(&mut self, mesh: MeshId, transform: Mat4) {
        self.draws.push(DrawCommand { mesh, transform });
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

    pub(crate) fn draws(&self) -> &[DrawCommand] {
        &self.draws
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn any_mesh_id() -> MeshId {
        MeshId::from_index(0)
    }

    #[test]
    fn draw_records_a_command() {
        let mut frame = Frame::new();

        frame.draw(any_mesh_id(), Mat4::IDENTITY);

        assert_eq!(frame.draw_count(), 1);
    }

    #[test]
    fn begin_discards_previous_draws_and_sets_background() {
        let mut frame = Frame::new();
        frame.draw(any_mesh_id(), Mat4::IDENTITY);

        frame.begin(Color::WHITE);

        assert_eq!(frame.draw_count(), 0);
        assert_eq!(frame.clear_color(), Color::WHITE);
    }
}
