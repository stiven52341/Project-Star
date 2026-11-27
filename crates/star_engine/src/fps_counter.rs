//! The frames-per-second counter (decision 85): how fast the game runs,
//! measured over half a second, shown in the top-left corner of its window
//! with a tiny pixel font of the engine's own, as overlays.

use std::fmt::Write as _;

use star_core::{EngineResult, Image};
use star_math::Vec2;
use star_render::{Color, Frame, Sprite, TextureId, TextureRegion};

/// How long the counter measures before it shows a new number, in
/// seconds: long enough to be read, short enough to follow changes.
pub const FPS_UPDATE_SECONDS: f32 = 0.5;

/// Whether the frames-per-second counter is shown: a resource of the
/// world, which games may change; F3 turns it on and off while playing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FpsDisplay {
    /// Whether the counter is drawn.
    pub shown: bool,
}

/// The characters of the font, in the order of its cells; the last cell
/// of the image is solid, for the counter's background.
const GLYPHS: &str = "0123456789.FPSms ";
/// Width and height of a character, in pixels of the font.
const GLYPH_WIDTH: u16 = 3;
const GLYPH_HEIGHT: u16 = 5;
/// The empty column after each cell of the image, so the nearest pixel of
/// a cell never takes its neighbor's.
const CELL_GAP: u16 = 1;
/// Each row of each character, left to right in its three lowest bits.
const GLYPH_ROWS: [[u8; GLYPH_HEIGHT as usize]; 17] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b110, 0b010, 0b010, 0b111], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b111, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b010, 0b010, 0b010], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b111], // 9
    [0b000, 0b000, 0b000, 0b000, 0b010], // .
    [0b111, 0b100, 0b110, 0b100, 0b100], // F
    [0b110, 0b101, 0b110, 0b100, 0b100], // P
    [0b011, 0b100, 0b010, 0b001, 0b110], // S
    [0b000, 0b000, 0b111, 0b111, 0b101], // m
    [0b000, 0b011, 0b110, 0b011, 0b110], // s
    [0b000, 0b000, 0b000, 0b000, 0b000], // space
];

/// How many window pixels a pixel of the font takes.
const FONT_SCALE: f32 = 3.0;
/// From the window's top-left corner to the counter's, in window pixels.
const MARGIN: f32 = 8.0;
/// Around the text, inside its background, in pixels of the font.
const PADDING: f32 = 1.0;
const TEXT_COLOR: Color = Color::WHITE;
const BACKGROUND_COLOR: Color = Color::rgba(0.0, 0.0, 0.0, 0.6);
const MILLISECONDS_PER_SECOND: f32 = 1000.0;

/// Counts the frames and keeps the text that shows how fast they go.
#[derive(Debug, Default)]
pub(crate) struct FpsCounter {
    frames: u32,
    elapsed: f32,
    /// What the counter shows; empty until the first measure.
    label: String,
}

impl FpsCounter {
    /// Counts a frame that took `delta` seconds; every
    /// [`FPS_UPDATE_SECONDS`], the label shows the frames per second and
    /// the milliseconds per frame of that time.
    pub(crate) fn tick(&mut self, delta: f32) {
        self.frames += 1;
        self.elapsed += delta;
        if self.elapsed < FPS_UPDATE_SECONDS {
            return;
        }
        // At most a few hundred frames are counted between measures.
        #[allow(clippy::cast_precision_loss)]
        let frames = self.frames as f32;
        let per_second = frames / self.elapsed;
        let milliseconds = self.elapsed * MILLISECONDS_PER_SECOND / frames;
        self.label.clear();
        // Writing to a `String` cannot fail.
        let _ = write!(self.label, "{per_second:.0} FPS {milliseconds:.1} ms");
        self.frames = 0;
        self.elapsed = 0.0;
    }

    /// What the counter shows; empty until the first measure.
    pub(crate) fn label(&self) -> &str {
        &self.label
    }

    /// Adds the label to `frame` as overlays in the window's top-left
    /// corner, its characters cut from `font` (the texture of
    /// [`font_image`]) over a dark background.
    pub(crate) fn draw(&self, font: TextureId, frame: &mut Frame) {
        let label = self.label();
        let characters = u16::try_from(label.chars().count()).unwrap_or(u16::MAX);
        if characters == 0 {
            return;
        }
        let pixel = FONT_SCALE;
        let advance = (f32::from(GLYPH_WIDTH) + 1.0) * pixel;
        let width = f32::from(characters) * advance - pixel + 2.0 * PADDING * pixel;
        let height = (f32::from(GLYPH_HEIGHT) + 2.0 * PADDING) * pixel;
        let top_left = Vec2::new(MARGIN, MARGIN);
        frame.draw_overlay(
            corner_sprite(font, top_left, Vec2::new(width, height))
                .with_region(cell(solid_cell()))
                .with_color(BACKGROUND_COLOR),
        );
        let text = top_left + Vec2::new(PADDING * pixel, PADDING * pixel);
        let glyph = Vec2::new(
            f32::from(GLYPH_WIDTH) * pixel,
            f32::from(GLYPH_HEIGHT) * pixel,
        );
        for (index, character) in (0_u16..).zip(label.chars()) {
            let Some(place) = GLYPHS.chars().position(|known| known == character) else {
                continue;
            };
            let at = text + Vec2::new(f32::from(index) * advance, 0.0);
            frame.draw_overlay(
                corner_sprite(font, at, glyph)
                    .with_region(cell(place))
                    .with_color(TEXT_COLOR)
                    .with_layer(1),
            );
        }
    }
}

/// A sprite of `font` with its top-left corner at `at`.
fn corner_sprite(font: TextureId, at: Vec2, size: Vec2) -> Sprite {
    Sprite::new(font, at, size).with_anchor(Vec2::new(0.0, 1.0))
}

/// The cell of the image after the characters, all of it white.
fn solid_cell() -> usize {
    GLYPH_ROWS.len()
}

/// How many cells the font's image has.
fn cell_count() -> u16 {
    u16::try_from(solid_cell() + 1).unwrap_or(u16::MAX)
}

/// Where cell `index` is in the font's image, without its gap.
fn cell(index: usize) -> TextureRegion {
    let stride = f32::from(GLYPH_WIDTH + CELL_GAP);
    let width = f32::from(cell_count()) * stride;
    let index = f32::from(u16::try_from(index).unwrap_or(u16::MAX));
    let left = index * stride / width;
    TextureRegion::new(
        Vec2::new(left, 0.0),
        Vec2::new(left + f32::from(GLYPH_WIDTH) / width, 1.0),
    )
}

/// The image of the font: its characters side by side, white on clear,
/// each with an empty column after it, and a solid cell at the end.
///
/// # Errors
///
/// Never in practice: its size and pixels always match.
pub(crate) fn font_image() -> EngineResult<Image> {
    let stride = u32::from(GLYPH_WIDTH + CELL_GAP);
    let (glyph_width, glyph_height) = (u32::from(GLYPH_WIDTH), u32::from(GLYPH_HEIGHT));
    let width = u32::from(cell_count()) * stride;
    let mut pixels = vec![0_u8; (width * glyph_height * 4) as usize];
    let solid = [0b111; GLYPH_HEIGHT as usize];
    let cells = GLYPH_ROWS.iter().chain(std::iter::once(&solid));
    for (index, rows) in (0_u32..).zip(cells) {
        for (row, bits) in (0_u32..).zip(rows) {
            for column in 0..glyph_width {
                if bits & (1 << (glyph_width - 1 - column)) != 0 {
                    let x = index * stride + column;
                    let start = ((row * width + x) * 4) as usize;
                    if let Some(pixel) = pixels.get_mut(start..start + 4) {
                        pixel.copy_from_slice(&[255; 4]);
                    }
                }
            }
        }
    }
    Image::from_rgba8(width, glyph_height, pixels)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_label_shows_the_frames_of_each_half_second() {
        let mut counter = FpsCounter::default();
        for _ in 0..14 {
            counter.tick(1.0 / 30.0);
        }
        assert_eq!(counter.label(), "", "not yet half a second");

        counter.tick(1.0 / 30.0);

        assert_eq!(counter.label(), "30 FPS 33.3 ms");
    }

    #[test]
    fn every_character_of_a_label_is_in_the_font() {
        let label = "0123456789 FPS 16.6 ms";

        assert!(label.chars().all(|character| GLYPHS.contains(character)));
        assert_eq!(GLYPHS.chars().count(), GLYPH_ROWS.len());
    }

    #[test]
    fn labels_are_drawn_over_a_background() {
        let mut counter = FpsCounter::default();
        let mut frame = Frame::new();
        counter.draw(TextureId::from_index(0), &mut frame);
        assert!(frame.overlays().is_empty(), "nothing to show yet");

        counter.tick(FPS_UPDATE_SECONDS);
        counter.draw(TextureId::from_index(0), &mut frame);

        // "2 FPS 500.0 ms": its 14 characters (spaces are blank cells)
        // and the background behind them.
        assert_eq!(frame.overlays().len(), counter.label().len() + 1);
        assert_eq!(frame.overlays()[0].color, BACKGROUND_COLOR);
    }

    #[test]
    fn the_font_has_a_cell_for_each_character_and_a_solid_one() {
        let image = font_image().unwrap();

        assert_eq!(image.height(), u32::from(GLYPH_HEIGHT));
        assert_eq!(
            image.width(),
            u32::from(cell_count() * (GLYPH_WIDTH + CELL_GAP))
        );
        let solid = cell(solid_cell());
        assert!(solid.max.x <= 1.0 && solid.min.x < solid.max.x);
    }
}
