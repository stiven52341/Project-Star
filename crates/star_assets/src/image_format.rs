//! The image file formats the engine decodes.

use star_core::{EngineError, EngineResult, Image};

use crate::AssetPath;
use crate::png_image::decode_png;

/// A kind of image file, recognized by its extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ImageFormat {
    /// Lossless, with transparency: `.png`.
    Png,
}

impl ImageFormat {
    /// The format of the image at `path`, from its extension (in any case).
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] if the extension is not that of a
    /// supported image format.
    pub fn of(path: &AssetPath) -> EngineResult<Self> {
        let extension = path.extension().unwrap_or_default();
        if extension.eq_ignore_ascii_case("png") {
            Ok(Self::Png)
        } else {
            Err(EngineError::Asset(format!(
                "\"{path}\" is not a supported image: use .png"
            )))
        }
    }

    /// Decodes an image of this format from its bytes, to 4 bytes per pixel.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] if `bytes` is not a valid image of this format.
    pub fn decode(self, bytes: &[u8]) -> EngineResult<Image> {
        match self {
            Self::Png => decode_png(bytes),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn format_of(path: &str) -> EngineResult<ImageFormat> {
        ImageFormat::of(&AssetPath::new(path).unwrap())
    }

    #[test]
    fn the_format_comes_from_the_extension_in_any_case() {
        assert_eq!(format_of("hero.png").unwrap(), ImageFormat::Png);
        assert_eq!(format_of("sprites/HERO.PNG").unwrap(), ImageFormat::Png);
    }

    #[test]
    fn unknown_extensions_are_asset_errors() {
        for path in ["hero.bmp", "hero"] {
            let error = format_of(path).unwrap_err();

            assert!(error.to_string().contains(path), "{error}");
        }
    }
}
