//! Which part of a graph the canvas shows: how far it is moved and how
//! close it is.

use eframe::egui::{Pos2, Rect, Vec2, vec2};

/// The closest the view gets.
const MAX_ZOOM: f32 = 2.0;
/// The farthest the view gets.
const MIN_ZOOM: f32 = 0.25;
/// Where the graph's origin starts, from the canvas's top-left corner, so
/// nodes at the origin are not stuck to the edge.
const START_OFFSET: Vec2 = vec2(40.0, 40.0);

/// Pan and zoom of a canvas. A point of the graph is drawn at
/// `canvas origin + pan + point × zoom`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct CanvasView {
    pan: Vec2,
    zoom: f32,
}

impl Default for CanvasView {
    fn default() -> Self {
        Self {
            pan: START_OFFSET,
            zoom: 1.0,
        }
    }
}

impl CanvasView {
    pub(super) const fn zoom(self) -> f32 {
        self.zoom
    }

    /// Where `point` of the graph is on the screen.
    pub(super) fn to_screen(self, canvas_origin: Pos2, point: Pos2) -> Pos2 {
        canvas_origin + self.pan + point.to_vec2() * self.zoom
    }

    /// Where `rect` of the graph is on the screen.
    pub(super) fn rect_to_screen(self, canvas_origin: Pos2, rect: Rect) -> Rect {
        Rect::from_min_max(
            self.to_screen(canvas_origin, rect.min),
            self.to_screen(canvas_origin, rect.max),
        )
    }

    /// The point of the graph drawn at `screen_point`.
    pub(super) fn to_graph(self, canvas_origin: Pos2, screen_point: Pos2) -> Pos2 {
        ((screen_point - canvas_origin - self.pan) / self.zoom).to_pos2()
    }

    /// Moves the graph so `point` is at the middle of a canvas of `size`.
    pub(super) fn look_at(&mut self, point: Pos2, size: Vec2) {
        self.pan = size / 2.0 - point.to_vec2() * self.zoom;
    }

    /// Moves the graph by `delta` screen points.
    pub(super) fn pan_by(&mut self, delta: Vec2) {
        self.pan += delta;
    }

    /// Multiplies the zoom by `factor`, within its limits, keeping the
    /// point under `screen_point` where it is.
    pub(super) fn zoom_around(&mut self, canvas_origin: Pos2, screen_point: Pos2, factor: f32) {
        let fixed = self.to_graph(canvas_origin, screen_point);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        self.pan = screen_point - canvas_origin - fixed.to_vec2() * self.zoom;
    }
}

#[cfg(test)]
mod tests {
    use eframe::egui::pos2;

    use super::*;

    const ORIGIN: Pos2 = pos2(10.0, 20.0);

    #[test]
    fn the_graph_origin_starts_near_the_corner() {
        let view = CanvasView::default();

        assert_eq!(view.to_screen(ORIGIN, Pos2::ZERO), ORIGIN + START_OFFSET);
    }

    #[test]
    fn screen_and_graph_points_convert_both_ways() {
        let mut view = CanvasView::default();
        view.zoom_around(ORIGIN, pos2(300.0, 200.0), 1.5);
        view.pan_by(vec2(-35.0, 12.0));
        let point = pos2(123.0, -45.0);

        let back = view.to_graph(ORIGIN, view.to_screen(ORIGIN, point));

        assert!((back - point).length() < 1e-3, "{back:?}");
    }

    #[test]
    fn zooming_keeps_the_point_under_the_pointer() {
        let mut view = CanvasView::default();
        let pointer = pos2(250.0, 180.0);
        let under_pointer = view.to_graph(ORIGIN, pointer);

        view.zoom_around(ORIGIN, pointer, 1.8);

        assert!((view.to_screen(ORIGIN, under_pointer) - pointer).length() < 1e-3);
    }

    #[test]
    fn looking_at_a_point_puts_it_in_the_middle() {
        let mut view = CanvasView::default();
        view.zoom_around(ORIGIN, ORIGIN, 1.5);
        let point = pos2(300.0, -80.0);
        let size = vec2(800.0, 600.0);

        view.look_at(point, size);

        let middle = ORIGIN + size / 2.0;
        assert!((view.to_screen(ORIGIN, point) - middle).length() < 1e-3);
    }

    #[test]
    fn zoom_stays_within_its_limits() {
        let mut view = CanvasView::default();

        view.zoom_around(ORIGIN, ORIGIN, 100.0);
        assert_eq!(view.zoom(), MAX_ZOOM);

        view.zoom_around(ORIGIN, ORIGIN, 0.001);
        assert_eq!(view.zoom(), MIN_ZOOM);
    }
}
