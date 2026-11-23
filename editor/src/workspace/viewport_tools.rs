//! The tools of the scene view: the bar that chooses one, the keys that
//! do too (`Q`, `W`, `E`, `R`, as in other editors) and the painting of
//! the handles over the node selected.

use eframe::egui;
use star_engine::prelude::Vec2;

use crate::scene_editor::{Handle, HandleShape, Tip, Tool};
use crate::texts;

const TOOLS: [(Tool, egui::Key); 4] = [
    (Tool::View, egui::Key::Q),
    (Tool::Move, egui::Key::W),
    (Tool::Rotate, egui::Key::E),
    (Tool::Scale, egui::Key::R),
];
const TOOLBAR_PADDING: f32 = 4.0;
const TOOLBAR_ROUNDING: u8 = 4;

const X_COLOR: egui::Color32 = egui::Color32::from_rgb(230, 85, 80);
const Y_COLOR: egui::Color32 = egui::Color32::from_rgb(125, 205, 90);
const Z_COLOR: egui::Color32 = egui::Color32::from_rgb(80, 145, 240);
/// Between X's red and Y's green, unlike the highlight's yellow.
const XY_COLOR: egui::Color32 = egui::Color32::from_rgb(235, 140, 60);
const CENTER_COLOR: egui::Color32 = egui::Color32::from_rgb(225, 225, 225);
/// The color of the handle under the pointer or being dragged.
const ACTIVE_COLOR: egui::Color32 = egui::Color32::from_rgb(255, 215, 60);
/// How much of the square in the middle lets the scene show through.
const CENTER_FILL_ALPHA: f32 = 0.25;
const LINE_WIDTH: f32 = 2.5;
const ACTIVE_LINE_WIDTH: f32 = 3.5;
const ARROW_LENGTH: f32 = 12.0;
const ARROW_HALF_WIDTH: f32 = 5.0;
const BOX_HALF_SIDE: f32 = 4.5;

/// The bar of tools at the top left of the view, `margin` in from its
/// corner.
pub(super) fn show_toolbar(ui: &mut egui::Ui, rect: egui::Rect, margin: f32, tool: &mut Tool) {
    let texts = texts::current();
    let area = rect.shrink(margin);
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(area)
            .layout(egui::Layout::left_to_right(egui::Align::Min)),
        |ui| {
            egui::Frame::new()
                .fill(ui.visuals().panel_fill)
                .corner_radius(TOOLBAR_ROUNDING)
                .inner_margin(TOOLBAR_PADDING)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (choice, _) in TOOLS {
                            ui.selectable_value(tool, choice, texts.tool_name(choice))
                                .on_hover_text(texts.tool_hint(choice));
                        }
                    });
                });
        },
    );
}

/// The tool whose key was pressed, unless a text field is being typed in.
pub(super) fn tool_from_keys(ui: &egui::Ui) -> Option<Tool> {
    if ui.ctx().egui_wants_keyboard_input() {
        return None;
    }
    ui.input(|input| {
        TOOLS
            .iter()
            .find(|(_, key)| input.key_pressed(*key))
            .map(|(tool, _)| *tool)
    })
}

/// Paints `shapes`, placed in pixels from the top left of `rect`, with
/// the handle `active` highlighted.
pub(super) fn paint_handles(
    painter: &egui::Painter,
    rect: egui::Rect,
    shapes: &[HandleShape],
    active: Option<Handle>,
) {
    let place = |point: Vec2| rect.left_top() + egui::vec2(point.x, point.y);
    for shape in shapes {
        let is_active = active == Some(shape.handle);
        let color = if is_active {
            ACTIVE_COLOR
        } else {
            handle_color(shape.handle)
        };
        let width = if is_active {
            ACTIVE_LINE_WIDTH
        } else {
            LINE_WIDTH
        };
        let points: Vec<egui::Pos2> = shape.points.iter().map(|point| place(*point)).collect();
        let stroke = egui::Stroke::new(width, color);
        if shape.handle == Handle::Center {
            painter.add(egui::Shape::convex_polygon(
                points,
                color.gamma_multiply(CENTER_FILL_ALPHA),
                stroke,
            ));
            continue;
        }
        if let (Some(&start), Some(&end)) = (points.first(), points.last()) {
            paint_tip(painter, shape.tip, start, end, color);
        }
        painter.add(if shape.closed {
            egui::Shape::closed_line(points, stroke)
        } else {
            egui::Shape::line(points, stroke)
        });
    }
}

const fn handle_color(handle: Handle) -> egui::Color32 {
    match handle {
        Handle::Axis(0) => X_COLOR,
        Handle::Axis(1) => Y_COLOR,
        Handle::Axis(_) => Z_COLOR,
        Handle::AxesXY => XY_COLOR,
        Handle::Center => CENTER_COLOR,
    }
}

/// The arrow or box at `end` of a line coming from `start`.
fn paint_tip(
    painter: &egui::Painter,
    tip: Tip,
    start: egui::Pos2,
    end: egui::Pos2,
    color: egui::Color32,
) {
    match tip {
        Tip::None => {}
        Tip::Box => {
            painter.rect_filled(
                egui::Rect::from_center_size(end, egui::Vec2::splat(BOX_HALF_SIDE * 2.0)),
                0.0,
                color,
            );
        }
        Tip::Arrow => {
            let along = (end - start).normalized();
            if !along.x.is_finite() {
                return;
            }
            let across = along.rot90() * ARROW_HALF_WIDTH;
            painter.add(egui::Shape::convex_polygon(
                vec![end + along * ARROW_LENGTH, end + across, end - across],
                color,
                egui::Stroke::NONE,
            ));
        }
    }
}
