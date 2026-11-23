//! Small copies of the images of the assets folder, for their icons in the
//! assets panel: each read once, when it is first shown, and shrunk so a
//! folder of big images does not fill the GPU.

use std::collections::HashMap;
use std::path::Path;

use eframe::egui;
use star_engine::prelude::{AssetFolder, AssetPath};

/// The longest side of a thumbnail, in pixels.
const MAX_SIDE: usize = 96;
const BYTES_PER_PIXEL: usize = 4;

/// The thumbnails made, by path inside the assets folder; `None` for images
/// that could not be read, which get a page instead.
pub(super) struct Thumbnails {
    folder: AssetFolder,
    made: HashMap<String, Option<egui::TextureHandle>>,
}

impl Thumbnails {
    pub(super) fn new(assets: &Path) -> Self {
        Self {
            folder: AssetFolder::new(assets),
            made: HashMap::new(),
        }
    }

    /// Forgets every thumbnail, so changed images are read again.
    pub(super) fn clear(&mut self) {
        self.made.clear();
    }

    /// The thumbnail of the image at `path`, made now if it is the first
    /// time it is asked for.
    pub(super) fn get(&mut self, ctx: &egui::Context, path: &str) -> Option<&egui::TextureHandle> {
        self.made
            .entry(path.to_owned())
            .or_insert_with(|| make(ctx, &self.folder, path))
            .as_ref()
    }
}

fn make(ctx: &egui::Context, folder: &AssetFolder, path: &str) -> Option<egui::TextureHandle> {
    let image = AssetPath::new(path)
        .and_then(|asset| folder.load_image(&asset))
        .inspect_err(|error| log::warn!("{error}"))
        .ok()?;
    let width = usize::try_from(image.width()).ok()?;
    let height = usize::try_from(image.height()).ok()?;
    let (size, pixels) = shrink([width, height], image.pixels());
    Some(ctx.load_texture(
        format!("thumbnail:{path}"),
        egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
        egui::TextureOptions::LINEAR,
    ))
}

/// The image of `size` (width, height) with `pixels` (four bytes each, row
/// by row), shrunk to fit [`MAX_SIDE`] by picking the nearest pixel; as it
/// is if it already fits.
fn shrink(size: [usize; 2], pixels: &[u8]) -> ([usize; 2], Vec<u8>) {
    let [width, height] = size;
    let longest = width.max(height);
    if longest <= MAX_SIDE {
        return (size, pixels.to_vec());
    }
    let scaled = |side: usize| (side * MAX_SIDE / longest).max(1);
    let (new_width, new_height) = (scaled(width), scaled(height));
    let mut shrunk = Vec::with_capacity(new_width * new_height * BYTES_PER_PIXEL);
    for row in 0..new_height {
        let source_row = row * height / new_height;
        for column in 0..new_width {
            let source_column = column * width / new_width;
            let start = (source_row * width + source_column) * BYTES_PER_PIXEL;
            let pixel = pixels
                .get(start..start + BYTES_PER_PIXEL)
                .unwrap_or(&[0; BYTES_PER_PIXEL]);
            shrunk.extend_from_slice(pixel);
        }
    }
    ([new_width, new_height], shrunk)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_images_are_kept_as_they_are() {
        let pixels = vec![7; 2 * 3 * BYTES_PER_PIXEL];

        assert_eq!(shrink([2, 3], &pixels), ([2, 3], pixels));
    }

    #[test]
    fn big_images_shrink_to_fit_keeping_their_shape() {
        let (width, height) = (MAX_SIDE * 4, MAX_SIDE * 2);
        // Black on the left half, white on the right one.
        let row: Vec<u8> = (0..width)
            .flat_map(|column| [if column < width / 2 { 0 } else { 255 }; BYTES_PER_PIXEL])
            .collect();
        let pixels = row.repeat(height);

        let (size, shrunk) = shrink([width, height], &pixels);

        assert_eq!(size, [MAX_SIDE, MAX_SIDE / 2]);
        assert_eq!(shrunk.len(), MAX_SIDE * MAX_SIDE / 2 * BYTES_PER_PIXEL);
        assert_eq!(shrunk[0], 0);
        assert_eq!(shrunk[(MAX_SIDE - 1) * BYTES_PER_PIXEL], 255);
    }
}
