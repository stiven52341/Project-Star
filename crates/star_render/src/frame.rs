//! Description of what to draw in one frame.

use std::ops::Range;

use star_core::{EngineError, EngineResult};
use star_math::Mat4;

use crate::skinning::MAX_JOINTS;
use crate::{AmbientLight, Camera, Color, Light, Light2d, Material, Mesh3dId, MeshId, Sprite};

/// One mesh to draw, placed in the world by `transform`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DrawCommand {
    pub(crate) mesh: MeshId,
    pub(crate) transform: Mat4,
}

/// One 3D mesh to draw with a material, placed in the world by `transform`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SurfaceDraw {
    pub(crate) mesh: Mesh3dId,
    pub(crate) material: Material,
    pub(crate) transform: Mat4,
    /// For a skinned draw, where its joint matrices are among the frame's.
    pub(crate) joints: Option<(usize, usize)>,
}

/// Everything to draw in a frame: background, camera, meshes and sprites.
///
/// Games fill it every frame; the renderer turns it into GPU work.
/// Its storage is reused between frames to avoid allocations.
#[derive(Debug, Clone, Default)]
pub struct Frame {
    clear_color: Color,
    camera: Camera,
    ambient_light: AmbientLight,
    lights: Vec<Light>,
    /// `None` until the game sets it: [`AmbientLight::FULL`].
    ambient_light_2d: Option<AmbientLight>,
    lights_2d: Vec<Light2d>,
    draws: Vec<DrawCommand>,
    surfaces: Vec<SurfaceDraw>,
    /// The joint matrices of every skinned draw, one after the other.
    joints: Vec<Mat4>,
    sprites: Vec<Sprite>,
    /// Sprites over everything, in window pixels.
    overlays: Vec<Sprite>,
}

impl Frame {
    /// Creates an empty frame with a black background and a default 2D camera.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Starts a new frame filled with `clear_color`, discarding the previous
    /// meshes, sprites and lights. The camera and the ambient lights are
    /// kept.
    pub fn begin(&mut self, clear_color: Color) {
        self.clear_color = clear_color;
        self.lights.clear();
        self.lights_2d.clear();
        self.draws.clear();
        self.surfaces.clear();
        self.joints.clear();
        self.sprites.clear();
        self.overlays.clear();
    }

    /// Changes the color the frame is filled with before drawing.
    pub fn set_clear_color(&mut self, clear_color: Color) {
        self.clear_color = clear_color;
    }

    /// Sets the point of view used to draw the frame.
    pub fn set_camera(&mut self, camera: Camera) {
        self.camera = camera;
    }

    /// Sets the light that reaches every 3D surface from everywhere (none
    /// at first).
    pub fn set_ambient_light(&mut self, light: AmbientLight) {
        self.ambient_light = light;
    }

    /// Lights the 3D meshes of this frame with `light`, besides the others.
    /// A frame can have up to [`MAX_LIGHTS`](crate::MAX_LIGHTS): rendering
    /// a frame with more is an error.
    pub fn add_light(&mut self, light: impl Into<Light>) {
        self.lights.push(light.into());
    }

    /// The light that reaches every 3D surface from everywhere.
    #[must_use]
    pub fn ambient_light(&self) -> AmbientLight {
        self.ambient_light
    }

    /// Sets the light that reaches every sprite ([`AmbientLight::FULL`] at
    /// first, so sprites show their own colors). Darken it for 2D lights
    /// to be seen.
    pub fn set_ambient_light_2d(&mut self, light: AmbientLight) {
        self.ambient_light_2d = Some(light);
    }

    /// The light that reaches every sprite.
    #[must_use]
    pub fn ambient_light_2d(&self) -> AmbientLight {
        self.ambient_light_2d.unwrap_or(AmbientLight::FULL)
    }

    /// Lights the sprites of this frame with `light`, besides the others.
    /// A frame can have up to [`MAX_LIGHTS`](crate::MAX_LIGHTS) 2D lights:
    /// rendering a frame with more is an error.
    pub fn add_light_2d(&mut self, light: impl Into<Light2d>) {
        self.lights_2d.push(light.into());
    }

    /// The 2D lights of this frame, in the order they were added.
    #[must_use]
    pub fn lights_2d(&self) -> &[Light2d] {
        &self.lights_2d
    }

    /// The lights of this frame, in the order they were added.
    #[must_use]
    pub fn lights(&self) -> &[Light] {
        &self.lights
    }

    /// Draws `mesh` placed in the world by `transform` (position, rotation, scale).
    pub fn draw(&mut self, mesh: MeshId, transform: Mat4) {
        self.draws.push(DrawCommand { mesh, transform });
    }

    /// Draws the 3D `mesh` with `material`, placed in the world by
    /// `transform`. 3D meshes are drawn first, then colored meshes, then
    /// sprites.
    pub fn draw_mesh_3d(&mut self, mesh: Mesh3dId, material: Material, transform: Mat4) {
        self.surfaces.push(SurfaceDraw {
            mesh,
            material,
            transform,
            joints: None,
        });
    }

    /// Draws a skinned `mesh` as [`Frame::draw_mesh_3d`] does, each vertex
    /// bent by the matrices of `joints` it follows: from the mesh's space,
    /// as it was bound, to where each joint takes it, before `transform`.
    /// A mesh without joints is drawn as it is.
    ///
    /// # Errors
    ///
    /// [`EngineError::CapacityExceeded`] if there are more than
    /// [`MAX_JOINTS`] joints; the mesh is not drawn.
    pub fn draw_skinned_mesh_3d(
        &mut self,
        mesh: Mesh3dId,
        material: Material,
        transform: Mat4,
        joints: &[Mat4],
    ) -> EngineResult<()> {
        if joints.len() > MAX_JOINTS {
            return Err(EngineError::CapacityExceeded {
                resource: "joints of a skinned mesh",
            });
        }
        let start = self.joints.len();
        self.joints.extend_from_slice(joints);
        self.surfaces.push(SurfaceDraw {
            mesh,
            material,
            transform,
            joints: Some((start, joints.len())),
        });
        Ok(())
    }

    /// Draws `sprite`. Sprites are drawn after the meshes, by layer and, in
    /// each layer, in the order they were added. Sprites in a row with the
    /// same texture are drawn together, so keeping sprites that share a
    /// texture together (for instance in a sprite sheet) draws faster.
    pub fn draw_sprite(&mut self, sprite: Sprite) {
        self.sprites.push(sprite);
    }

    /// Draws `sprite` over everything else, unlit and wherever the camera
    /// is: its `position` and `size` are in physical pixels of the window,
    /// from its top-left corner with Y growing downwards. For what a game
    /// shows on the screen rather than in its world, such as counters.
    /// Overlays are drawn by layer, like sprites.
    pub fn draw_overlay(&mut self, sprite: Sprite) {
        self.overlays.push(sprite);
    }

    /// The sprites drawn over everything in this frame, in the order they
    /// were added.
    #[must_use]
    pub fn overlays(&self) -> &[Sprite] {
        &self.overlays
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

    /// Number of 3D meshes drawn in this frame.
    #[must_use]
    pub fn mesh_3d_count(&self) -> usize {
        self.surfaces.len()
    }

    /// Number of sprites drawn in this frame.
    #[must_use]
    pub fn sprite_count(&self) -> usize {
        self.sprites.len()
    }

    pub(crate) fn draws(&self) -> &[DrawCommand] {
        &self.draws
    }

    pub(crate) fn surfaces(&self) -> &[SurfaceDraw] {
        &self.surfaces
    }

    /// The joint matrices of `draw`, if it is skinned.
    pub(crate) fn joints_of(&self, draw: &SurfaceDraw) -> Option<&[Mat4]> {
        let (start, count) = draw.joints?;
        self.joints.get(Range {
            start,
            end: start + count,
        })
    }

    /// The sprites drawn in this frame, in the order they were added.
    #[must_use]
    pub fn sprites(&self) -> &[Sprite] {
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
        frame.draw_mesh_3d(Mesh3dId(0), Material::default(), Mat4::IDENTITY);
        frame.draw_sprite(any_sprite());
        frame.set_ambient_light(AmbientLight::new(Color::WHITE, 0.5));
        frame.add_light(crate::DirectionalLight {
            direction: star_math::Vec3::UP,
            color: Color::WHITE,
            intensity: 1.0,
        });

        frame.set_ambient_light_2d(AmbientLight::new(Color::BLACK, 1.0));
        frame.add_light_2d(crate::PointLight2d {
            position: star_math::Vec2::ZERO,
            color: Color::WHITE,
            intensity: 1.0,
            range: 2.0,
        });

        frame.begin(Color::WHITE);

        assert_eq!(frame.lights().len(), 0);
        assert_eq!(frame.lights_2d().len(), 0);
        assert_eq!(frame.ambient_light().intensity, 0.5);
        assert_eq!(frame.ambient_light_2d().color, Color::BLACK);

        assert_eq!(frame.draw_count(), 0);
        assert_eq!(frame.mesh_3d_count(), 0);
        assert_eq!(frame.sprite_count(), 0);
        assert_eq!(frame.clear_color(), Color::WHITE);
    }

    #[test]
    fn sprites_show_their_own_colors_until_the_2d_ambient_light_changes() {
        assert_eq!(Frame::new().ambient_light_2d(), AmbientLight::FULL);
    }

    #[test]
    fn the_background_can_change_while_drawing() {
        let mut frame = Frame::new();
        frame.begin(Color::BLACK);
        frame.draw_sprite(any_sprite());

        frame.set_clear_color(Color::WHITE);

        assert_eq!(frame.clear_color(), Color::WHITE);
        assert_eq!(frame.sprite_count(), 1);
    }
}
