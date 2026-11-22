//! Sprites: textured rectangles placed in the 2D world.

use star_math::Vec2;

use crate::{Color, TextureId};

/// The part of a texture a sprite shows, in texture coordinates: `(0, 0)` is
/// the top-left corner of the texture and `(1, 1)` the bottom-right one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextureRegion {
    /// Top-left corner.
    pub min: Vec2,
    /// Bottom-right corner.
    pub max: Vec2,
}

impl TextureRegion {
    /// The whole texture.
    pub const FULL: Self = Self::new(Vec2::ZERO, Vec2::ONE);

    /// The region between the corners `min` (top-left) and `max` (bottom-right).
    /// Swapping the coordinates of an axis mirrors the image along it.
    #[must_use]
    pub const fn new(min: Vec2, max: Vec2) -> Self {
        Self { min, max }
    }

    /// One cell of a sprite sheet cut into `columns` x `rows` equal cells,
    /// counted from the top-left one: `(0, 0)`. `None` if the sheet has no
    /// cells or the cell is outside it.
    #[must_use]
    pub fn cell(columns: u16, rows: u16, column: u16, row: u16) -> Option<Self> {
        if column >= columns || row >= rows {
            return None;
        }
        let (width, height) = (1.0 / f32::from(columns), 1.0 / f32::from(rows));
        let min = Vec2::new(f32::from(column) * width, f32::from(row) * height);
        Some(Self::new(min, min + Vec2::new(width, height)))
    }
}

impl Default for TextureRegion {
    fn default() -> Self {
        Self::FULL
    }
}

/// A textured rectangle in the 2D world (the `z = 0` plane).
///
/// Sprites are drawn after meshes, ordered by [`layer`](Self::layer); within
/// a layer, in the order they were added to the frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sprite {
    /// Image shown.
    pub texture: TextureId,
    /// Part of the texture shown.
    pub region: TextureRegion,
    /// Where the [`anchor`](Self::anchor) is in the world.
    pub position: Vec2,
    /// Width and height in world units. A negative component mirrors the
    /// sprite along that axis.
    pub size: Vec2,
    /// Counter-clockwise rotation around the anchor, in radians.
    pub rotation: f32,
    /// Point of the sprite placed at `position` and rotated around, from
    /// `(0, 0)` (bottom-left corner) to `(1, 1)` (top-right corner).
    pub anchor: Vec2,
    /// Multiplied by the texture's colors: white keeps them; a lower alpha
    /// makes the sprite translucent.
    pub color: Color,
    /// Sprites in higher layers are drawn on top of lower ones.
    pub layer: i32,
}

impl Sprite {
    /// The center of the sprite.
    pub const CENTER: Vec2 = Vec2::new(0.5, 0.5);

    /// The whole `texture`, centered at `position`, `size` world units big,
    /// with its own colors, unrotated, in layer 0.
    #[must_use]
    pub const fn new(texture: TextureId, position: Vec2, size: Vec2) -> Self {
        Self {
            texture,
            region: TextureRegion::FULL,
            position,
            size,
            rotation: 0.0,
            anchor: Self::CENTER,
            color: Color::WHITE,
            layer: 0,
        }
    }

    /// The same sprite showing only `region` of its texture.
    #[must_use]
    pub const fn with_region(mut self, region: TextureRegion) -> Self {
        self.region = region;
        self
    }

    /// The same sprite rotated `rotation` radians counter-clockwise.
    #[must_use]
    pub const fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// The same sprite placed and rotated by another `anchor`.
    #[must_use]
    pub const fn with_anchor(mut self, anchor: Vec2) -> Self {
        self.anchor = anchor;
        self
    }

    /// The same sprite tinted with `color`.
    #[must_use]
    pub const fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    /// The same sprite in `layer`.
    #[must_use]
    pub const fn with_layer(mut self, layer: i32) -> Self {
        self.layer = layer;
        self
    }

    /// The corners in the world, with the point of the texture each shows:
    /// bottom-left, bottom-right, top-right, top-left.
    pub(crate) fn corners(&self) -> [(Vec2, Vec2); 4] {
        let (sin, cos) = self.rotation.sin_cos();
        let TextureRegion { min, max } = self.region;
        // (fraction of the width, fraction of the height, u, v); `v` grows downwards.
        let corners = [
            (0.0, 0.0, min.x, max.y),
            (1.0, 0.0, max.x, max.y),
            (1.0, 1.0, max.x, min.y),
            (0.0, 1.0, min.x, min.y),
        ];
        corners.map(|(across, up, u, v)| {
            let local = Vec2::new(
                (across - self.anchor.x) * self.size.x,
                (up - self.anchor.y) * self.size.y,
            );
            let rotated = Vec2::new(
                local.x.mul_add(cos, -local.y * sin),
                local.x.mul_add(sin, local.y * cos),
            );
            (self.position + rotated, Vec2::new(u, v))
        })
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    const EPSILON: f32 = 1e-5;

    fn sprite() -> Sprite {
        Sprite::new(TextureId(0), Vec2::new(10.0, 20.0), Vec2::new(4.0, 2.0))
    }

    fn assert_near(actual: Vec2, expected: Vec2) {
        assert!(
            (actual - expected).length() < EPSILON,
            "{actual:?} is not {expected:?}"
        );
    }

    #[test]
    fn sprites_are_centered_on_their_position() {
        let [bottom_left, _, top_right, _] = sprite().corners();

        assert_near(bottom_left.0, Vec2::new(8.0, 19.0));
        assert_near(top_right.0, Vec2::new(12.0, 21.0));
    }

    #[test]
    fn the_top_of_the_texture_is_shown_at_the_top() {
        let [bottom_left, _, top_right, _] = sprite().corners();

        assert_near(bottom_left.1, Vec2::new(0.0, 1.0));
        assert_near(top_right.1, Vec2::new(1.0, 0.0));
    }

    #[test]
    fn sprites_rotate_around_their_anchor() {
        let rotated = sprite().with_anchor(Vec2::ZERO).with_rotation(FRAC_PI_2);

        let [bottom_left, bottom_right, ..] = rotated.corners();

        assert_near(bottom_left.0, Vec2::new(10.0, 20.0));
        assert_near(bottom_right.0, Vec2::new(10.0, 24.0));
    }

    #[test]
    fn regions_choose_the_texture_coordinates() {
        let region = TextureRegion::new(Vec2::new(0.25, 0.5), Vec2::new(0.5, 1.0));

        let [bottom_left, _, top_right, _] = sprite().with_region(region).corners();

        assert_near(bottom_left.1, Vec2::new(0.25, 1.0));
        assert_near(top_right.1, Vec2::new(0.5, 0.5));
    }

    #[test]
    fn sheet_cells_split_the_texture_evenly() {
        let cell = TextureRegion::cell(4, 2, 3, 1).unwrap();

        assert_near(cell.min, Vec2::new(0.75, 0.5));
        assert_near(cell.max, Vec2::ONE);
    }

    #[test]
    fn cells_outside_the_sheet_do_not_exist() {
        assert_eq!(TextureRegion::cell(4, 2, 4, 0), None);
        assert_eq!(TextureRegion::cell(0, 0, 0, 0), None);
    }
}
