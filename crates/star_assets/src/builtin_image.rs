//! Images the engine brings (decision 86): a white square and a white
//! circle, named by asset paths no file can have taken by chance, so a
//! `Sprite` shows a solid shape of its color without an image file. A body
//! on a node whose sprite shows one takes that shape.

use star_core::{EngineResult, Image};

use crate::AssetPath;

/// The image of a white square.
pub const SQUARE_IMAGE: &str = "@square";
/// The image of a white circle, smooth at its edge.
pub const CIRCLE_IMAGE: &str = "@circle";

/// The side of the circle's image, in pixels: smooth when stretched over
/// most of a window.
const CIRCLE_SIZE: u32 = 128;
/// The side of the square's image: one pixel would do, but filters blend
/// a texture's edge with what lies past it.
const SQUARE_SIZE: u32 = 4;
const OPAQUE: u8 = 255;

/// An image the engine brings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltinImage {
    /// [`SQUARE_IMAGE`].
    Square,
    /// [`CIRCLE_IMAGE`].
    Circle,
}

impl BuiltinImage {
    /// Every one, in the order they are offered.
    pub const ALL: [Self; 2] = [Self::Square, Self::Circle];

    /// The image `path` names, if it is one the engine brings.
    #[must_use]
    pub fn of(path: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|image| image.path() == path)
    }

    /// The path that names it.
    #[must_use]
    pub const fn path(self) -> &'static str {
        match self {
            Self::Square => SQUARE_IMAGE,
            Self::Circle => CIRCLE_IMAGE,
        }
    }

    /// Its pixels.
    ///
    /// # Errors
    ///
    /// Never in practice: its size and pixels always match.
    pub fn image(self) -> EngineResult<Image> {
        match self {
            Self::Square => {
                let pixels = vec![OPAQUE; (SQUARE_SIZE * SQUARE_SIZE * 4) as usize];
                Image::from_rgba8(SQUARE_SIZE, SQUARE_SIZE, pixels)
            }
            Self::Circle => circle(),
        }
    }
}

impl AssetPath {
    /// The image the engine brings that the path names, if any.
    #[must_use]
    pub fn builtin_image(&self) -> Option<BuiltinImage> {
        BuiltinImage::of(self.as_str())
    }
}

/// A white disc filling the image, its edge a pixel wide and translucent.
fn circle() -> EngineResult<Image> {
    // The image is a few pixels across.
    #[allow(clippy::cast_precision_loss)]
    let radius = CIRCLE_SIZE as f32 / 2.0;
    let mut pixels = Vec::with_capacity((CIRCLE_SIZE * CIRCLE_SIZE * 4) as usize);
    for row in 0..CIRCLE_SIZE {
        for column in 0..CIRCLE_SIZE {
            #[allow(clippy::cast_precision_loss)]
            let (x, y) = (column as f32 + 0.5 - radius, row as f32 + 0.5 - radius);
            let inside = (radius - (x * x + y * y).sqrt()).clamp(0.0, 1.0);
            // `inside` is between 0 and 1.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let alpha = (inside * f32::from(OPAQUE)).round() as u8;
            pixels.extend_from_slice(&[OPAQUE, OPAQUE, OPAQUE, alpha]);
        }
    }
    Image::from_rgba8(CIRCLE_SIZE, CIRCLE_SIZE, pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha(image: &Image, column: u32, row: u32) -> u8 {
        image.pixels()[((row * image.width() + column) * 4 + 3) as usize]
    }

    #[test]
    fn paths_name_the_images_the_engine_brings() {
        for image in BuiltinImage::ALL {
            let path = AssetPath::new(image.path()).unwrap();
            assert_eq!(path.builtin_image(), Some(image));
        }
        assert_eq!(BuiltinImage::of("square.png"), None);
    }

    #[test]
    fn the_circle_is_solid_inside_and_clear_at_its_corners() {
        let circle = BuiltinImage::Circle.image().unwrap();
        let middle = CIRCLE_SIZE / 2;

        assert_eq!(alpha(&circle, middle, middle), OPAQUE);
        assert_eq!(alpha(&circle, 0, 0), 0);
        let edge = alpha(&circle, middle, 0);
        assert!(edge > 0 && edge < OPAQUE, "a smooth edge: {edge}");
        let square = BuiltinImage::Square.image().unwrap();
        assert!(square.pixels().iter().all(|&part| part == OPAQUE));
    }
}
