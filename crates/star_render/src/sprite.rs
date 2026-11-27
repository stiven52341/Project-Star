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

/// A part of an image in pixels: a rectangle `width` x `height` pixels big
/// whose top-left corner is `x` pixels right of the image's top-left one
/// and `y` pixels below it. It is how people and files name the part of an
/// image a sprite shows; the GPU's [`TextureRegion`] is made from it once
/// the image's size is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PixelRegion {
    /// Pixels from the image's left side to the region's.
    pub x: u32,
    /// Pixels from the image's top side to the region's.
    pub y: u32,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
}

impl PixelRegion {
    /// The region `width` x `height` pixels big at `x`, `y`.
    #[must_use]
    pub const fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// The part of a texture of `image_size` pixels (width, height) this
    /// region shows, cut to the image. A region left with no pixels, or an
    /// image without them, shows the whole texture: a sprite never
    /// vanishes because its region was set badly.
    #[must_use]
    pub fn in_texture(self, image_size: [u32; 2]) -> TextureRegion {
        let [image_width, image_height] = image_size;
        let right = self.x.saturating_add(self.width).min(image_width);
        let bottom = self.y.saturating_add(self.height).min(image_height);
        if self.x >= right || self.y >= bottom {
            return TextureRegion::FULL;
        }
        let fraction = |pixels: u32, side: u32| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "image sides fit the GPU's limits, far below 2^24, exact in f32"
            )]
            let fraction = pixels as f32 / side as f32;
            fraction
        };
        TextureRegion::new(
            Vec2::new(
                fraction(self.x, image_width),
                fraction(self.y, image_height),
            ),
            Vec2::new(fraction(right, image_width), fraction(bottom, image_height)),
        )
    }

    /// The pixels `region` covers in an image of `image_size` pixels,
    /// rounded to whole ones: the way back from [`in_texture`](Self::in_texture).
    /// A mirrored axis counts from its smaller coordinate.
    #[must_use]
    pub fn of_texture(region: TextureRegion, image_size: [u32; 2]) -> Self {
        let [image_width, image_height] = image_size;
        let pixel = |fraction: f32, side: u32| {
            #[expect(
                clippy::cast_precision_loss,
                reason = "image sides fit the GPU's limits, far below 2^24, exact in f32"
            )]
            let pixels = (fraction.clamp(0.0, 1.0) * side as f32).round();
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "clamped between 0 and the side, which is a u32"
            )]
            let pixels = pixels as u32;
            pixels
        };
        let (left, right) = (
            region.min.x.min(region.max.x),
            region.min.x.max(region.max.x),
        );
        let (top, bottom) = (
            region.min.y.min(region.max.y),
            region.min.y.max(region.max.y),
        );
        let x = pixel(left, image_width);
        let y = pixel(top, image_height);
        Self::new(
            x,
            y,
            pixel(right, image_width) - x,
            pixel(bottom, image_height) - y,
        )
    }
}

/// A sprite sheet: an image cut into `columns` x `rows` equal cells, of
/// which a sprite shows one, its frame. Frames are counted from the
/// top-left cell, row by row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpriteSheet {
    columns: u16,
    rows: u16,
    frame: u32,
}

impl SpriteSheet {
    /// The whole image as one cell.
    pub const SINGLE: Self = Self {
        columns: 1,
        rows: 1,
        frame: 0,
    };

    /// The sheet of `columns` x `rows` cells showing `frame`. A sheet has
    /// at least one column and one row, and frames past the last cell start
    /// over from the first, so every value shows a cell.
    #[must_use]
    pub fn new(columns: u16, rows: u16, frame: u32) -> Self {
        let (columns, rows) = (columns.max(1), rows.max(1));
        let frames = u32::from(columns) * u32::from(rows);
        Self {
            columns,
            rows,
            frame: frame % frames,
        }
    }

    /// How many cells wide the image is.
    #[must_use]
    pub const fn columns(self) -> u16 {
        self.columns
    }

    /// How many cells tall the image is.
    #[must_use]
    pub const fn rows(self) -> u16 {
        self.rows
    }

    /// The cell shown.
    #[must_use]
    pub const fn frame(self) -> u32 {
        self.frame
    }

    /// The cell shown of `region`, cut as this sheet says.
    #[must_use]
    pub fn region_in(self, region: TextureRegion) -> TextureRegion {
        let columns = u32::from(self.columns);
        let (column, row) = (self.frame % columns, self.frame / columns);
        let span = region.max - region.min;
        let cell = Vec2::new(
            span.x / f32::from(self.columns),
            span.y / f32::from(self.rows),
        );
        #[expect(
            clippy::cast_precision_loss,
            reason = "a cell's column and row are below u16::MAX, exact in f32"
        )]
        let min = region.min + Vec2::new(cell.x * column as f32, cell.y * row as f32);
        TextureRegion::new(min, min + cell)
    }
}

impl Default for SpriteSheet {
    fn default() -> Self {
        Self::SINGLE
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
    fn sheet_frames_go_row_by_row_from_the_top_left() {
        let sheet = SpriteSheet::new(4, 2, 5);

        let cell = sheet.region_in(TextureRegion::FULL);

        assert_near(cell.min, Vec2::new(0.25, 0.5));
        assert_near(cell.max, Vec2::new(0.5, 1.0));
    }

    #[test]
    fn sheets_cut_the_region_they_are_given() {
        let region = TextureRegion::new(Vec2::new(0.5, 0.0), Vec2::new(1.0, 0.5));

        let cell = SpriteSheet::new(2, 1, 1).region_in(region);

        assert_near(cell.min, Vec2::new(0.75, 0.0));
        assert_near(cell.max, Vec2::new(1.0, 0.5));
    }

    #[test]
    fn every_sheet_shows_a_cell() {
        assert_eq!(SpriteSheet::new(0, 0, 7), SpriteSheet::SINGLE);
        assert_eq!(SpriteSheet::new(3, 2, 8).frame(), 2);
        assert_eq!(
            SpriteSheet::SINGLE.region_in(TextureRegion::FULL),
            TextureRegion::FULL
        );
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

    #[test]
    fn pixel_regions_become_fractions_of_the_texture() {
        let region = PixelRegion::new(16, 8, 32, 8).in_texture([64, 32]);

        assert_eq!(region.min, Vec2::new(0.25, 0.25));
        assert_eq!(region.max, Vec2::new(0.75, 0.5));
    }

    #[test]
    fn pixel_regions_are_cut_to_the_image() {
        let region = PixelRegion::new(32, 0, 100, 100).in_texture([64, 32]);

        assert_eq!(region.min, Vec2::new(0.5, 0.0));
        assert_eq!(region.max, Vec2::ONE);
    }

    #[test]
    fn pixel_regions_without_pixels_show_the_whole_texture() {
        assert_eq!(
            PixelRegion::new(10, 10, 0, 5).in_texture([64, 32]),
            TextureRegion::FULL
        );
        assert_eq!(
            PixelRegion::new(80, 0, 8, 8).in_texture([64, 32]),
            TextureRegion::FULL
        );
        assert_eq!(
            PixelRegion::new(0, 0, 8, 8).in_texture([0, 0]),
            TextureRegion::FULL
        );
    }

    #[test]
    fn texture_regions_go_back_to_their_pixels() {
        let pixels = PixelRegion::new(16, 8, 32, 8);

        assert_eq!(
            PixelRegion::of_texture(pixels.in_texture([64, 32]), [64, 32]),
            pixels
        );
    }

    #[test]
    fn mirrored_texture_regions_count_from_their_smaller_corner() {
        let mirrored = TextureRegion::new(Vec2::new(0.5, 0.0), Vec2::new(0.0, 1.0));

        assert_eq!(
            PixelRegion::of_texture(mirrored, [64, 32]),
            PixelRegion::new(0, 0, 32, 32)
        );
    }
}
