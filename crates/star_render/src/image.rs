//! Pictures in memory, ready to become textures.

use star_core::{EngineError, EngineResult};

/// Bytes of one pixel: red, green, blue and alpha.
const BYTES_PER_PIXEL: u32 = 4;

/// A picture in memory: rows of pixels from the top-left corner, each pixel
/// 4 bytes (red, green, blue, alpha) in sRGB space, as image files store them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Image {
    width: u32,
    height: u32,
    pixels: Vec<u8>,
}

impl Image {
    /// Creates an image of `width` x `height` pixels from their bytes.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Rendering`] if the image is empty or `pixels`
    /// does not hold exactly 4 bytes per pixel.
    pub fn from_rgba8(width: u32, height: u32, pixels: Vec<u8>) -> EngineResult<Self> {
        if width == 0 || height == 0 {
            return Err(EngineError::Rendering(format!(
                "images need at least one pixel, got {width}x{height}"
            )));
        }
        if Some(pixels.len()) != byte_count(width, height) {
            return Err(EngineError::Rendering(format!(
                "a {width}x{height} image needs {BYTES_PER_PIXEL} bytes per pixel, got {} bytes",
                pixels.len()
            )));
        }
        Ok(Self {
            width,
            height,
            pixels,
        })
    }

    /// Width in pixels.
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }

    /// Height in pixels.
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }

    /// The bytes of every pixel, row by row from the top-left corner.
    #[must_use]
    pub fn pixels(&self) -> &[u8] {
        &self.pixels
    }
}

/// Bytes needed by `width` x `height` pixels, if they fit in memory.
fn byte_count(width: u32, height: u32) -> Option<usize> {
    let pixels = usize::try_from(width)
        .ok()?
        .checked_mul(usize::try_from(height).ok()?)?;
    pixels.checked_mul(usize::try_from(BYTES_PER_PIXEL).ok()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn images_keep_their_size_and_pixels() {
        let pixels = vec![255; 2 * 3 * 4];

        let image = Image::from_rgba8(2, 3, pixels.clone()).unwrap();

        assert_eq!((image.width(), image.height()), (2, 3));
        assert_eq!(image.pixels(), pixels);
    }

    #[test]
    fn empty_images_are_rejected() {
        assert!(Image::from_rgba8(0, 4, Vec::new()).is_err());
    }

    #[test]
    fn pixels_must_match_the_size() {
        assert!(Image::from_rgba8(2, 2, vec![0; 15]).is_err());
        assert!(Image::from_rgba8(2, 2, vec![0; 17]).is_err());
    }

    #[test]
    fn sizes_too_big_for_memory_are_rejected() {
        assert_eq!(byte_count(u32::MAX, u32::MAX), None);
    }
}
