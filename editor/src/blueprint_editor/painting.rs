//! How a graph is drawn on the canvas: its grid, nodes, pins and wires.

use std::collections::HashMap;

use eframe::egui::{
    self, Align2, Color32, CornerRadius, FontId, Painter, Pos2, Rect, Shape, Stroke, pos2, vec2,
};
use star_blueprint::{Direction, Link, Node, NodeId, NodeKind, PinKind};

use super::layout::{HEADER_HEIGHT, NodeLayout, PlacedPin, wire_curve, wire_ends};
use crate::canvas_view::CanvasView;
use crate::style::ERROR_COLOR;
use crate::texts;

const BACKGROUND: Color32 = Color32::from_rgb(30, 30, 34);
const GRID_COLOR: Color32 = Color32::from_rgb(42, 42, 48);
/// Distance between grid lines, in graph units.
const GRID_SPACING: f32 = 40.0;
/// Below this distance on screen, grid lines are not drawn: they would only
/// fill the canvas.
const MIN_GRID_SPACING: f32 = 8.0;

const NODE_BODY: Color32 = Color32::from_rgba_premultiplied(44, 44, 52, 240);
const NODE_BORDER: Color32 = Color32::from_rgb(18, 18, 22);
const NODE_ROUNDING: u8 = 4;
const TITLE_COLOR: Color32 = Color32::from_rgb(240, 240, 240);
const PIN_LABEL_COLOR: Color32 = Color32::from_rgb(190, 190, 200);
/// Size of titles and pin labels at zoom 1.
const TITLE_SIZE: f32 = 14.0;
const PIN_LABEL_SIZE: f32 = 12.0;
/// Space between the edge of a node and its texts, at zoom 1.
const TEXT_PADDING: f32 = 10.0;

const ENTRY_COLOR: Color32 = Color32::from_rgb(150, 60, 60);
const FLOW_COLOR: Color32 = Color32::from_rgb(90, 90, 105);
const VARIABLE_COLOR: Color32 = Color32::from_rgb(55, 120, 70);
const CALL_COLOR: Color32 = Color32::from_rgb(50, 90, 150);
const VALUE_COLOR: Color32 = Color32::from_rgb(55, 110, 120);

const EXECUTION_COLOR: Color32 = Color32::from_rgb(225, 225, 225);
const DATA_COLOR: Color32 = Color32::from_rgb(120, 170, 230);
/// Radius of data pins, and half the size of execution pins, at zoom 1.
const PIN_RADIUS: f32 = 5.0;
/// Width of wires at zoom 1.
const WIRE_WIDTH: f32 = 2.5;
/// Width of the selected wire at zoom 1.
const SELECTED_WIRE_WIDTH: f32 = 4.0;
const SELECTED_BORDER: Color32 = Color32::from_rgb(240, 190, 80);
/// Inside of the selection box: the selection's color, see-through.
const SELECTION_FILL: Color32 = Color32::from_rgba_premultiplied(48, 38, 16, 48);
/// Width of the border of the selected node, and of nodes with mistakes.
const MARKED_BORDER_WIDTH: f32 = 2.0;

/// What a frame of the canvas is drawn with.
pub(super) struct Painting {
    pub(super) painter: Painter,
    pub(super) rect: Rect,
    pub(super) view: CanvasView,
}

impl Painting {
    fn screen(&self, point: Pos2) -> Pos2 {
        self.view.to_screen(self.rect.min, point)
    }

    /// `length` in graph units, on the screen.
    fn scaled(&self, length: f32) -> f32 {
        length * self.view.zoom()
    }

    pub(super) fn paint_background(&self) {
        self.painter.rect_filled(self.rect, 0.0, BACKGROUND);
        let spacing = self.scaled(GRID_SPACING);
        if spacing < MIN_GRID_SPACING {
            return;
        }
        let stroke = Stroke::new(1.0, GRID_COLOR);
        let first = self.screen(Pos2::ZERO);
        let start = pos2(
            first_line(first.x, self.rect.left(), spacing),
            first_line(first.y, self.rect.top(), spacing),
        );
        let mut x = start.x;
        while x <= self.rect.right() {
            self.painter.vline(x, self.rect.y_range(), stroke);
            x += spacing;
        }
        let mut y = start.y;
        while y <= self.rect.bottom() {
            self.painter.hline(self.rect.x_range(), y, stroke);
            y += spacing;
        }
    }

    /// A curve from the output to the input of `link`, if both exist; in
    /// the selection's color if it is `selected`.
    pub(super) fn paint_wire(
        &self,
        layouts: &HashMap<NodeId, NodeLayout>,
        link: &Link,
        selected: bool,
    ) {
        let Some((start, end)) = wire_ends(layouts, link) else {
            return;
        };
        let kind = layouts
            .get(&link.from.node)
            .and_then(|layout| layout.pin(&link.from.pin))
            .map_or(PinKind::Data, |placed| placed.pin.kind);
        let stroke = if selected {
            Stroke::new(self.scaled(SELECTED_WIRE_WIDTH), SELECTED_BORDER)
        } else {
            Stroke::new(self.scaled(WIRE_WIDTH), pin_color(kind))
        };
        self.paint_curve(
            wire_curve(start, end).map(|point| self.screen(point)),
            stroke,
        );
    }

    /// A wire being pulled from `pin`, which follows the pointer.
    pub(super) fn paint_pulled_wire(&self, pin: &PlacedPin, pointer: Pos2) {
        let at_pin = pin.position;
        let pointer = self.view.to_graph(self.rect.min, pointer);
        let (start, end) = match pin.pin.direction {
            Direction::Output => (at_pin, pointer),
            Direction::Input => (pointer, at_pin),
        };
        let stroke = Stroke::new(self.scaled(WIRE_WIDTH), pin_color(pin.pin.kind));
        self.paint_curve(
            wire_curve(start, end).map(|point| self.screen(point)),
            stroke,
        );
    }

    /// A wire along the curve with `control` points, on the screen.
    fn paint_curve(&self, control: [Pos2; 4], stroke: Stroke) {
        self.painter
            .add(egui::epaint::CubicBezierShape::from_points_stroke(
                control,
                false,
                Color32::TRANSPARENT,
                stroke,
            ));
    }

    /// The box being dragged to select nodes, between two screen points.
    pub(super) fn paint_selection_box(&self, start: Pos2, end: Pos2) {
        let area = Rect::from_two_pos(start, end);
        self.painter.rect_filled(area, 0, SELECTION_FILL);
        self.painter.rect_stroke(
            area,
            0,
            Stroke::new(1.0, SELECTED_BORDER),
            egui::StrokeKind::Inside,
        );
    }

    /// Draws `node`; its border says if it is `selected` or `has_mistakes`.
    pub(super) fn paint_node(
        &self,
        node: &Node,
        layout: &NodeLayout,
        selected: bool,
        has_mistakes: bool,
    ) {
        let rect = self.view.rect_to_screen(self.rect.min, layout.rect);
        self.painter
            .rect_filled(rect, CornerRadius::same(NODE_ROUNDING), NODE_BODY);
        let header = Rect::from_min_size(rect.min, vec2(rect.width(), self.scaled(HEADER_HEIGHT)));
        let header_rounding = CornerRadius {
            nw: NODE_ROUNDING,
            ne: NODE_ROUNDING,
            sw: 0,
            se: 0,
        };
        self.painter
            .rect_filled(header, header_rounding, header_color(&node.kind));
        self.painter.rect_stroke(
            rect,
            CornerRadius::same(NODE_ROUNDING),
            if selected {
                Stroke::new(MARKED_BORDER_WIDTH, SELECTED_BORDER)
            } else if has_mistakes {
                Stroke::new(MARKED_BORDER_WIDTH, ERROR_COLOR)
            } else {
                Stroke::new(1.0, NODE_BORDER)
            },
            egui::StrokeKind::Outside,
        );
        self.painter.text(
            header.left_center() + vec2(self.scaled(TEXT_PADDING), 0.0),
            Align2::LEFT_CENTER,
            texts::current().node_title(&node.kind),
            FontId::proportional(self.scaled(TITLE_SIZE)),
            TITLE_COLOR,
        );
        for placed in &layout.pins {
            self.paint_pin(&node.kind, placed);
        }
    }

    /// The pin's mark on the edge of the node, and its name inside.
    fn paint_pin(&self, kind: &NodeKind, placed: &PlacedPin) {
        let center = self.screen(placed.position);
        let size = self.scaled(PIN_RADIUS);
        let color = pin_color(placed.pin.kind);
        match placed.pin.kind {
            PinKind::Execution => self.painter.add(Shape::convex_polygon(
                vec![
                    center + vec2(-size, -size),
                    center + vec2(size, 0.0),
                    center + vec2(-size, size),
                ],
                color,
                Stroke::NONE,
            )),
            PinKind::Data => self.painter.circle_filled(center, size, color),
        };
        let label = texts::current().pin_label(kind, &placed.pin.name);
        if label.is_empty() {
            return;
        }
        let padding = self.scaled(TEXT_PADDING);
        let (position, align) = match placed.pin.direction {
            Direction::Input => (center + vec2(padding, 0.0), Align2::LEFT_CENTER),
            Direction::Output => (center - vec2(padding, 0.0), Align2::RIGHT_CENTER),
        };
        self.painter.text(
            position,
            align,
            label,
            FontId::proportional(self.scaled(PIN_LABEL_SIZE)),
            PIN_LABEL_COLOR,
        );
    }
}

/// The first grid line at or after `start`, given that one passes through
/// `line`.
fn first_line(line: f32, start: f32, spacing: f32) -> f32 {
    start + (line - start).rem_euclid(spacing)
}

/// The color of the node's title band: one for each family of nodes.
const fn header_color(kind: &NodeKind) -> Color32 {
    match kind {
        NodeKind::Entry => ENTRY_COLOR,
        NodeKind::Branch | NodeKind::ForEach | NodeKind::Return => FLOW_COLOR,
        NodeKind::GetVariable { .. } | NodeKind::SetVariable { .. } => VARIABLE_COLOR,
        NodeKind::Call(_) => CALL_COLOR,
        NodeKind::Literal { .. } | NodeKind::Operator { .. } => VALUE_COLOR,
    }
}

const fn pin_color(kind: PinKind) -> Color32 {
    match kind {
        PinKind::Execution => EXECUTION_COLOR,
        PinKind::Data => DATA_COLOR,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_lines_start_at_the_first_one_inside_the_canvas() {
        assert_eq!(first_line(105.0, 0.0, 40.0), 25.0);
        assert_eq!(first_line(-15.0, 0.0, 40.0), 25.0);
        assert_eq!(first_line(80.0, 80.0, 40.0), 80.0);
    }
}
