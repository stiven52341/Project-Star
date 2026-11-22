//! PNG images, decoded to RGBA8 pixels.

use std::io::Cursor;

use png::{ColorType, Decoder, Transformations};
use star_core::{EngineError, EngineResult, Image};

/// Opacity of the pixels of images without an alpha channel.
const OPAQUE: u8 = u8::MAX;

/// Decodes a PNG image from its bytes. Every kind of PNG is accepted
/// (grayscale, color, palette, with or without transparency, 8 or 16 bits);
/// the result always has 4 bytes per pixel.
///
/// # Errors
///
/// Returns [`EngineError::Asset`] if `bytes` is not a valid PNG image.
pub fn decode_png(bytes: &[u8]) -> EngineResult<Image> {
    let invalid = |error: png::DecodingError| EngineError::Asset(format!("invalid PNG: {error}"));
    let mut decoder = Decoder::new(Cursor::new(bytes));
    // 8 bits per channel, palettes expanded to colors and transparency to alpha.
    decoder.set_transformations(Transformations::normalize_to_color8());
    let mut reader = decoder.read_info().map_err(invalid)?;
    let size = reader
        .output_buffer_size()
        .ok_or_else(|| EngineError::Asset("the PNG image is too big".to_owned()))?;
    let mut samples = vec![0; size];
    let frame = reader.next_frame(&mut samples).map_err(invalid)?;
    samples.truncate(frame.buffer_size());
    let pixels = to_rgba(&samples, frame.color_type)?;
    Image::from_rgba8(frame.width, frame.height, pixels)
}

/// Converts 8-bit `samples` of `color_type` to red, green, blue and alpha.
fn to_rgba(samples: &[u8], color_type: ColorType) -> EngineResult<Vec<u8>> {
    let pixels = match color_type {
        ColorType::Rgba => samples.to_vec(),
        ColorType::Rgb => samples
            .chunks_exact(3)
            .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], OPAQUE])
            .collect(),
        ColorType::GrayscaleAlpha => samples
            .chunks_exact(2)
            .flat_map(|gray_alpha| {
                let [gray, alpha] = [gray_alpha[0], gray_alpha[1]];
                [gray, gray, gray, alpha]
            })
            .collect(),
        ColorType::Grayscale => samples
            .iter()
            .flat_map(|&gray| [gray, gray, gray, OPAQUE])
            .collect(),
        // Palettes are expanded by the decoder's transformations.
        ColorType::Indexed => {
            return Err(EngineError::Asset(
                "the PNG palette could not be expanded".to_owned(),
            ));
        }
    };
    Ok(pixels)
}

#[cfg(test)]
pub(crate) mod test_support {
    use png::{BitDepth, ColorType, Encoder};

    /// A PNG image of a single pixel of color `rgba`.
    pub(crate) fn one_pixel_png(rgba: [u8; 4]) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = Encoder::new(&mut bytes, 1, 1);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&rgba).unwrap();
        writer.finish().unwrap();
        bytes
    }
}

#[cfg(test)]
mod tests {
    use png::{BitDepth, Encoder};

    use super::*;

    /// A `width` x 1 PNG image of `color_type` made of `samples`.
    fn encode(width: u32, color_type: ColorType, depth: BitDepth, samples: &[u8]) -> Vec<u8> {
        encode_with(width, color_type, depth, samples, |_| {})
    }

    fn encode_with(
        width: u32,
        color_type: ColorType,
        depth: BitDepth,
        samples: &[u8],
        configure: impl FnOnce(&mut Encoder<'_, &mut Vec<u8>>),
    ) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = Encoder::new(&mut bytes, width, 1);
        encoder.set_color(color_type);
        encoder.set_depth(depth);
        configure(&mut encoder);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(samples).unwrap();
        writer.finish().unwrap();
        bytes
    }

    #[test]
    fn color_images_keep_their_pixels() {
        let samples = [1, 2, 3, 4, 5, 6, 7, 8];
        let png = encode(2, ColorType::Rgba, BitDepth::Eight, &samples);

        let image = decode_png(&png).unwrap();

        assert_eq!((image.width(), image.height()), (2, 1));
        assert_eq!(image.pixels(), samples);
    }

    #[test]
    fn images_without_alpha_are_opaque() {
        let png = encode(1, ColorType::Rgb, BitDepth::Eight, &[10, 20, 30]);

        assert_eq!(decode_png(&png).unwrap().pixels(), [10, 20, 30, 255]);
    }

    #[test]
    fn grayscale_images_become_gray_colors() {
        let gray = encode(1, ColorType::Grayscale, BitDepth::Eight, &[90]);
        let gray_alpha = encode(1, ColorType::GrayscaleAlpha, BitDepth::Eight, &[90, 40]);

        assert_eq!(decode_png(&gray).unwrap().pixels(), [90, 90, 90, 255]);
        assert_eq!(decode_png(&gray_alpha).unwrap().pixels(), [90, 90, 90, 40]);
    }

    #[test]
    fn sixteen_bit_images_are_reduced_to_eight_bits() {
        let png = encode(
            1,
            ColorType::Rgb,
            BitDepth::Sixteen,
            &[0xAB, 0xCD, 0, 0, 0xFF, 0xFF],
        );

        assert_eq!(decode_png(&png).unwrap().pixels(), [0xAB, 0, 0xFF, 255]);
    }

    #[test]
    fn palette_images_are_expanded_with_their_transparency() {
        let png = encode_with(2, ColorType::Indexed, BitDepth::Eight, &[1, 0], |encoder| {
            encoder.set_palette(vec![255, 0, 0, 0, 0, 255]);
            encoder.set_trns(vec![255, 128]);
        });

        assert_eq!(
            decode_png(&png).unwrap().pixels(),
            [0, 0, 255, 128, 255, 0, 0, 255]
        );
    }

    #[test]
    fn corrupt_data_is_an_asset_error() {
        let error = decode_png(b"not a png").unwrap_err();

        assert!(matches!(error, EngineError::Asset(_)), "{error}");
    }
}
