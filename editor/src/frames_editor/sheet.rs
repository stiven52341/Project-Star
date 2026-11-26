//! The sprite sheet of a `.frames` file, drawn in the editor: its image fit
//! in the space it has, cut by a grid into numbered cells, those of the
//! clip selected marked; and the clip playing in a small preview.

use eframe::egui;
use star_engine::star_render::{SpriteSheet, TextureRegion};
use star_engine::star_scene::{FrameClip, SpriteFrames};

use crate::texts;

const GRID_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(255, 255, 255, 140);
const GRID_WIDTH: f32 = 1.0;
const CLIP_CELL_COLOR: egui::Color32 = egui::Color32::from_rgba_premultiplied(40, 110, 200, 90);
const HOVER_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 210, 80);
const HOVER_WIDTH: f32 = 2.0;
const NUMBER_COLOR: egui::Color32 = egui::Color32::WHITE;
const NUMBER_SHADOW: egui::Color32 = egui::Color32::from_rgba_premultiplied(0, 0, 0, 200);
const NUMBER_SIZE: f32 = 11.0;
/// Space between a cell's corner and its number.
const NUMBER_MARGIN: f32 = 3.0;
/// Cells narrower or lower than this, in points, are not numbered: the
/// numbers would cover them.
const MIN_NUMBERED_CELL: f32 = 22.0;
/// The side of the preview, in points.
const PREVIEW_SIDE: f32 = 96.0;
const PREVIEW_BACKGROUND: egui::Color32 = egui::Color32::from_gray(40);
const PLACEHOLDER_COLOR: egui::Color32 = egui::Color32::from_gray(60);

/// Draws the sheet of `frames` (its image, if it is loaded) in the space
/// left, marking the cells of `clip`. Returns the cell clicked.
pub(super) fn show_sheet(
    ui: &mut egui::Ui,
    image: Option<&egui::TextureHandle>,
    frames: &SpriteFrames,
    clip: Option<&FrameClip>,
) -> Option<u32> {
    let (space, response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::click());
    let painter = ui.painter_at(space);
    let sheet = if let Some(texture) = image {
        let sheet = fit(texture.size_vec2(), space);
        let whole = egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0));
        painter.image(texture.id(), sheet, whole, egui::Color32::WHITE);
        sheet
    } else {
        painter.rect_filled(space, 0.0, PLACEHOLDER_COLOR);
        space
    };
    let (columns, rows) = (frames.columns, frames.rows);
    for &frame in clip.map(|clip| clip.frames.as_slice()).unwrap_or_default() {
        painter.rect_filled(cell_rect(sheet, columns, rows, frame), 0.0, CLIP_CELL_COLOR);
    }
    paint_grid(&painter, sheet, columns, rows);
    let hovered = response
        .hover_pos()
        .and_then(|point| cell_at(sheet, columns, rows, point));
    if let Some(cell) = hovered {
        painter.rect_stroke(
            cell_rect(sheet, columns, rows, cell),
            0.0,
            egui::Stroke::new(HOVER_WIDTH, HOVER_COLOR),
            egui::StrokeKind::Inside,
        );
    }
    response.clicked().then_some(hovered).flatten()
}

/// The lines between the cells, and each cell's number if they are big
/// enough to show it.
fn paint_grid(painter: &egui::Painter, sheet: egui::Rect, columns: u16, rows: u16) {
    let stroke = egui::Stroke::new(GRID_WIDTH, GRID_COLOR);
    let cell = cell_size(sheet, columns, rows);
    for column in 1..columns {
        let x = sheet.left() + cell.x * f32::from(column);
        painter.vline(x, sheet.y_range(), stroke);
    }
    for row in 1..rows {
        let y = sheet.top() + cell.y * f32::from(row);
        painter.hline(sheet.x_range(), y, stroke);
    }
    if cell.x < MIN_NUMBERED_CELL || cell.y < MIN_NUMBERED_CELL {
        return;
    }
    let font = egui::FontId::proportional(NUMBER_SIZE);
    let cells = u32::from(columns) * u32::from(rows);
    for frame in 0..cells {
        let corner = cell_rect(sheet, columns, rows, frame).min + egui::Vec2::splat(NUMBER_MARGIN);
        let number = frame.to_string();
        let shadow = corner + egui::Vec2::splat(1.0);
        painter.text(
            shadow,
            egui::Align2::LEFT_TOP,
            &number,
            font.clone(),
            NUMBER_SHADOW,
        );
        painter.text(
            corner,
            egui::Align2::LEFT_TOP,
            number,
            font.clone(),
            NUMBER_COLOR,
        );
    }
}

/// The clip `clip` playing, one cell after another at its speed, in a small
/// square; always looping, so a short clip can be watched.
pub(super) fn show_preview(
    ui: &mut egui::Ui,
    image: Option<&egui::TextureHandle>,
    frames: &SpriteFrames,
    clip: &FrameClip,
) {
    ui.label(texts::current().clip_preview());
    let (space, _) = ui.allocate_exact_size(egui::Vec2::splat(PREVIEW_SIDE), egui::Sense::hover());
    let painter = ui.painter_at(space);
    painter.rect_filled(space, 0.0, PREVIEW_BACKGROUND);
    let (Some(texture), Some(frame)) = (image, frame_at(clip, ui.input(|input| input.time))) else {
        return;
    };
    let uv = cell_uv(frames.columns, frames.rows, frame);
    let size = texture.size_vec2() * uv.size();
    painter.image(texture.id(), fit(size, space), uv, egui::Color32::WHITE);
    ui.ctx().request_repaint();
}

/// The cell `clip` shows `seconds` after it started, going round it.
fn frame_at(clip: &FrameClip, seconds: f64) -> Option<u32> {
    let count = clip.frames.len();
    if count == 0 {
        return None;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the editor's clock and the speed are positive, and the frame is rounded down on purpose"
    )]
    let shown = (seconds * f64::from(clip.speed)).floor() as usize;
    clip.frames.get(shown % count).copied()
}

/// `size` scaled to fit `space` keeping its shape, centered in it.
fn fit(size: egui::Vec2, space: egui::Rect) -> egui::Rect {
    if size.x <= 0.0 || size.y <= 0.0 {
        return space;
    }
    let scale = (space.width() / size.x).min(space.height() / size.y);
    egui::Rect::from_center_size(space.center(), size * scale)
}

fn cell_size(sheet: egui::Rect, columns: u16, rows: u16) -> egui::Vec2 {
    egui::vec2(
        sheet.width() / f32::from(columns.max(1)),
        sheet.height() / f32::from(rows.max(1)),
    )
}

/// Where the cell `frame` is drawn, for a sheet drawn in `sheet`.
fn cell_rect(sheet: egui::Rect, columns: u16, rows: u16, frame: u32) -> egui::Rect {
    let uv = cell_uv(columns, rows, frame);
    egui::Rect::from_min_max(
        sheet.lerp_inside(uv.min.to_vec2()),
        sheet.lerp_inside(uv.max.to_vec2()),
    )
}

/// The part of the image the cell `frame` is, from `(0, 0)` (top-left) to
/// `(1, 1)` (bottom-right).
fn cell_uv(columns: u16, rows: u16, frame: u32) -> egui::Rect {
    let region = SpriteSheet::new(columns, rows, frame).region_in(TextureRegion::FULL);
    egui::Rect::from_min_max(
        egui::pos2(region.min.x, region.min.y),
        egui::pos2(region.max.x, region.max.y),
    )
}

/// The cell under `point`, for a sheet drawn in `sheet`.
fn cell_at(sheet: egui::Rect, columns: u16, rows: u16, point: egui::Pos2) -> Option<u32> {
    if !sheet.contains(point) {
        return None;
    }
    let cell = cell_size(sheet, columns, rows);
    let offset = point - sheet.min;
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the point is inside the sheet, so the column and row are small and not negative"
    )]
    let (column, row) = (
        ((offset.x / cell.x) as u32).min(u32::from(columns) - 1),
        ((offset.y / cell.y) as u32).min(u32::from(rows) - 1),
    );
    Some(row * u32::from(columns) + column)
}

#[cfg(test)]
mod tests {
    use star_engine::star_scene::DEFAULT_CLIP_SPEED;

    use super::*;

    fn sheet() -> egui::Rect {
        egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(400.0, 200.0))
    }

    fn clip(frames: &[u32]) -> FrameClip {
        FrameClip {
            name: "run".to_owned(),
            frames: frames.to_vec(),
            speed: DEFAULT_CLIP_SPEED,
            looping: true,
        }
    }

    #[test]
    fn cells_are_found_row_by_row_from_the_top_left() {
        assert_eq!(cell_at(sheet(), 4, 2, egui::pos2(15.0, 25.0)), Some(0));
        assert_eq!(cell_at(sheet(), 4, 2, egui::pos2(409.0, 25.0)), Some(3));
        assert_eq!(cell_at(sheet(), 4, 2, egui::pos2(115.0, 125.0)), Some(5));
        assert_eq!(cell_at(sheet(), 4, 2, egui::pos2(5.0, 25.0)), None);
    }

    #[test]
    fn cells_are_drawn_where_they_are_found() {
        let cell = cell_rect(sheet(), 4, 2, 5);

        assert_eq!(cell.min, egui::pos2(110.0, 120.0));
        assert_eq!(cell.max, egui::pos2(210.0, 220.0));
        assert_eq!(cell_at(sheet(), 4, 2, cell.center()), Some(5));
    }

    #[test]
    fn images_fit_their_space_keeping_their_shape() {
        let fitted = fit(egui::vec2(64.0, 32.0), sheet());

        assert_eq!(fitted.size(), egui::vec2(400.0, 200.0));
        let tall = fit(egui::vec2(10.0, 40.0), sheet());
        assert_eq!(tall.size(), egui::vec2(50.0, 200.0));
        assert_eq!(tall.center(), sheet().center());
    }

    #[test]
    fn previews_go_round_their_clip_at_its_speed() {
        let run = clip(&[4, 5, 6]);
        let step = 1.0 / f64::from(DEFAULT_CLIP_SPEED);

        let shown: Vec<Option<u32>> = (0..4)
            .map(|index| frame_at(&run, (f64::from(index) + 0.5) * step))
            .collect();

        assert_eq!(shown, [Some(4), Some(5), Some(6), Some(4)]);
        assert_eq!(frame_at(&clip(&[]), 1.0), None);
    }
}
