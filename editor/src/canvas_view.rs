//! Which part of a graph a canvas shows (a blueprint's, a state
//! machine's): how far it is moved and how close it is.

use eframe::egui::{self, Event, Modifiers, MouseWheelUnit, Pos2, Rect, Response, Vec2, vec2};

/// The closest the view gets.
const MAX_ZOOM: f32 = 2.0;
/// The farthest the view gets.
const MIN_ZOOM: f32 = 0.25;
/// Where the graph's origin starts, from the canvas's top-left corner, so
/// nodes at the origin are not stuck to the edge.
const START_OFFSET: Vec2 = vec2(40.0, 40.0);

/// How much one line of the mouse wheel zooms.
const ZOOM_PER_LINE: f32 = 0.1;

/// Pan and zoom of a canvas. A point of the graph is drawn at
/// `canvas origin + pan + point × zoom`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CanvasView {
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
    pub(crate) const fn zoom(self) -> f32 {
        self.zoom
    }

    /// Where `point` of the graph is on the screen.
    pub(crate) fn to_screen(self, canvas_origin: Pos2, point: Pos2) -> Pos2 {
        canvas_origin + self.pan + point.to_vec2() * self.zoom
    }

    /// Where `rect` of the graph is on the screen.
    pub(crate) fn rect_to_screen(self, canvas_origin: Pos2, rect: Rect) -> Rect {
        Rect::from_min_max(
            self.to_screen(canvas_origin, rect.min),
            self.to_screen(canvas_origin, rect.max),
        )
    }

    /// The point of the graph drawn at `screen_point`.
    pub(crate) fn to_graph(self, canvas_origin: Pos2, screen_point: Pos2) -> Pos2 {
        ((screen_point - canvas_origin - self.pan) / self.zoom).to_pos2()
    }

    /// Moves the graph so `point` is at the middle of a canvas of `size`.
    pub(crate) fn look_at(&mut self, point: Pos2, size: Vec2) {
        self.pan = size / 2.0 - point.to_vec2() * self.zoom;
    }

    /// Moves the graph by `delta` screen points.
    pub(crate) fn pan_by(&mut self, delta: Vec2) {
        self.pan += delta;
    }

    /// Multiplies the zoom by `factor`, within its limits, keeping the
    /// point under `screen_point` where it is.
    pub(crate) fn zoom_around(&mut self, canvas_origin: Pos2, screen_point: Pos2, factor: f32) {
        let fixed = self.to_graph(canvas_origin, screen_point);
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        self.pan = screen_point - canvas_origin - fixed.to_vec2() * self.zoom;
    }
}

/// What the mouse wheel or the touchpad asks of the view.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Scroll {
    /// Zoom by so many lines of the wheel (up is closer).
    Zoom(f32),
    /// Move the graph by so many screen points.
    Pan(Vec2),
}

/// Zooms and moves `view` with the wheel and the touchpad, while the
/// pointer is over the canvas of `response`, whose top-left is `origin`.
pub(crate) fn scroll_view(ui: &egui::Ui, view: &mut CanvasView, response: &Response, origin: Pos2) {
    let Some(pointer) = response.hover_pos() else {
        return;
    };
    let (scrolls, pinch) = ui.input(|input| {
        let scrolls: Vec<Scroll> = input
            .raw
            .events
            .iter()
            .filter_map(|event| match event {
                Event::MouseWheel {
                    unit,
                    delta,
                    modifiers,
                    ..
                } => scroll_of(*unit, *delta, *modifiers),
                _ => None,
            })
            .collect();
        (scrolls, input.zoom_delta())
    });
    for scroll in scrolls {
        match scroll {
            Scroll::Zoom(lines) => {
                view.zoom_around(origin, pointer, (lines * ZOOM_PER_LINE).exp());
            }
            Scroll::Pan(delta) => view.pan_by(delta),
        }
    }
    if (pinch - 1.0).abs() > f32::EPSILON {
        view.zoom_around(origin, pointer, pinch);
    }
}

/// What a wheel event asks: a mouse wheel counts lines and zooms, a
/// touchpad counts points and moves the view. With `Ctrl` it is egui's
/// zoom (read with pinching), so it is not counted twice.
fn scroll_of(unit: MouseWheelUnit, delta: Vec2, modifiers: Modifiers) -> Option<Scroll> {
    if modifiers.command || modifiers.ctrl {
        return None;
    }
    match unit {
        MouseWheelUnit::Point => Some(Scroll::Pan(delta)),
        MouseWheelUnit::Line | MouseWheelUnit::Page => Some(Scroll::Zoom(delta.y)),
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

    #[test]
    fn the_mouse_wheel_zooms_and_the_touchpad_moves() {
        let delta = vec2(3.0, -2.0);

        assert_eq!(
            scroll_of(MouseWheelUnit::Line, delta, Modifiers::NONE),
            Some(Scroll::Zoom(-2.0))
        );
        assert_eq!(
            scroll_of(MouseWheelUnit::Point, delta, Modifiers::NONE),
            Some(Scroll::Pan(delta))
        );
    }

    #[test]
    fn control_and_wheel_is_left_to_the_zoom_of_egui() {
        assert_eq!(
            scroll_of(MouseWheelUnit::Line, vec2(0.0, 1.0), Modifiers::COMMAND),
            None
        );
    }
}
