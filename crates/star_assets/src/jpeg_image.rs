//! JPEG images, decoded to RGBA8 pixels.

use star_core::{EngineError, EngineResult, Image};
use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

/// Decodes a JPEG image from its bytes. Color and grayscale images are
/// accepted; the result always has 4 bytes per pixel, all opaque (JPEG has
/// no transparency).
///
/// # Errors
///
/// Returns [`EngineError::Asset`] if `bytes` is not a valid JPEG image.
pub fn decode_jpeg(bytes: &[u8]) -> EngineResult<Image> {
    let invalid = |error| EngineError::Asset(format!("invalid JPEG: {error}"));
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::RGBA);
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(bytes), options);
    let pixels = decoder.decode().map_err(invalid)?;
    let info = decoder
        .info()
        .ok_or_else(|| EngineError::Asset("the JPEG image has no size".to_owned()))?;
    Image::from_rgba8(u32::from(info.width), u32::from(info.height), pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// JPEG is lossy: how far a decoded channel may be from the original.
    const TOLERANCE: u8 = 4;

    fn assert_every_pixel_near(image: &Image, expected: [u8; 4]) {
        for pixel in image.pixels().chunks_exact(4) {
            for (&channel, expected) in pixel.iter().zip(expected) {
                assert!(channel.abs_diff(expected) <= TOLERANCE, "{pixel:?}");
            }
        }
    }

    #[test]
    fn color_images_are_opaque_rgba() {
        let image = decode_jpeg(include_bytes!("../test_data/red_8x4.jpg")).unwrap();

        assert_eq!((image.width(), image.height()), (8, 4));
        assert_every_pixel_near(&image, [200, 40, 40, 255]);
    }

    #[test]
    fn grayscale_images_become_gray_colors() {
        let image = decode_jpeg(include_bytes!("../test_data/gray_8x4.jpg")).unwrap();

        assert_every_pixel_near(&image, [128, 128, 128, 255]);
    }

    #[test]
    fn corrupt_data_is_an_asset_error() {
        let error = decode_jpeg(b"not a jpeg").unwrap_err();

        assert!(matches!(error, EngineError::Asset(_)), "{error}");
    }
}
