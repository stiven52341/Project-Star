//! The lanes of the timeline: a ruler of seconds, where the playhead is
//! dragged; a lane per track with its keys, and one with the calls. Keys
//! and calls are selected with a click (`Shift` or `Ctrl` adds to the
//! selection), moved by dragging, deleted with `Delete`, copied with
//! `Ctrl+C` and pasted at the playhead with `Ctrl+V`; a double click on a
//! track's lane adds a key there with the value the track has then.

use eframe::egui;
use star_engine::star_scene::{NodeOutline, TrackAnimation};
use star_script::semantic::ClassTable;

use super::edits::{self, Mark};
use super::{KeyDrag, Timeline, calls, mark_values, paths, track_names};

/// How tall the ruler is.
const RULER_HEIGHT: f32 = 18.0;
/// How tall each lane is.
pub(super) const LANE_HEIGHT: f32 = 22.0;
/// How wide the column of the tracks' names is.
pub(super) const NAMES_WIDTH: f32 = 280.0;
/// The narrowest the lanes get, however small the panel.
const MIN_LANES_WIDTH: f32 = 120.0;
/// Room left at each end of the lanes, so the keys at the start and the
/// end can be seen whole and grabbed.
const LANE_MARGIN: f32 = 8.0;
/// Half the width of a key's diamond.
const KEY_RADIUS: f32 = 5.0;
/// How far from a key a click still takes it.
const HIT_RADIUS: f32 = 7.0;
/// The fewest pixels between two ticks of the ruler.
const MIN_TICK_GAP: f32 = 48.0;
/// The steps the ruler's ticks can have, in seconds.
const TICK_STEPS: [f32; 9] = [0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0];
/// How tall the ruler's ticks are.
const TICK_HEIGHT: f32 = 5.0;
/// How thick the playhead is.
const PLAYHEAD_WIDTH: f32 = 1.5;
/// The playhead's color.
const PLAYHEAD_COLOR: egui::Color32 = egui::Color32::from_rgb(230, 90, 80);
/// How faint the lanes' background is.
const LANE_FILL: egui::Color32 = egui::Color32::from_rgba_premultiplied(8, 8, 8, 8);

/// Where times are drawn: the animation's length across the lanes.
#[derive(Debug, Clone, Copy)]
pub(super) struct Scale {
    left: f32,
    width: f32,
    length: f32,
}

impl Scale {
    fn new(lanes: egui::Rect, length: f32) -> Self {
        Self {
            left: lanes.left() + LANE_MARGIN,
            width: (lanes.width() - 2.0 * LANE_MARGIN).max(1.0),
            length,
        }
    }

    pub(super) fn x_of(self, time: f32) -> f32 {
        self.left + time / self.length * self.width
    }

    pub(super) fn time_at(self, x: f32) -> f32 {
        ((x - self.left) / self.width * self.length).clamp(0.0, self.length)
    }
}

/// A lane and the marks drawn on it, where they are.
struct Lane {
    response: egui::Response,
    /// The track it shows, or `None` for the calls.
    track: Option<usize>,
    marks: Vec<(Mark, egui::Pos2)>,
}

/// Shows the lanes of `animation`, which animates the node at `root` of
/// `nodes` and those under it, and the values of what is selected. Returns
/// whether the animation changed.
pub(super) fn show(
    ui: &mut egui::Ui,
    timeline: &mut Timeline,
    animation: &mut TrackAnimation,
    nodes: &[NodeOutline],
    root: usize,
    classes: Option<&ClassTable>,
) -> bool {
    let mut changed = mark_values::show(ui, timeline, animation);
    let area = egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let ruler = show_ruler(ui, timeline, animation.length);
            let scale = Scale::new(ruler, animation.length);
            let mut lanes = Vec::with_capacity(animation.tracks.len() + 1);
            let mut removed = None;
            for track in 0..animation.tracks.len() {
                let name = ui.horizontal(|ui| {
                    let outcome = track_names::show(ui, &mut animation.tracks[track], nodes, root);
                    let lane = show_lane(ui, timeline, animation, Some(track), scale);
                    (outcome, lane)
                });
                let (outcome, lane) = name.inner;
                changed |= outcome.changed;
                if outcome.removed {
                    removed = Some(track);
                }
                lanes.push(lane);
            }
            let calls_row = ui.horizontal(|ui| {
                let added = calls::show_name(ui, timeline, animation, nodes, root, classes);
                (added, show_lane(ui, timeline, animation, None, scale))
            });
            let (added, calls_lane) = calls_row.inner;
            changed |= added;
            lanes.push(calls_lane);
            draw_playhead(
                ui,
                scale,
                timeline.time,
                ruler.top(),
                ui.min_rect().bottom(),
            );
            if let Some(track) = removed {
                remove_track(timeline, animation, track);
                return true;
            }
            lanes.iter().fold(false, |moved, lane| {
                moved | follow_pointer(ui, timeline, animation, lane, scale)
            })
        });
    changed |= area.inner;
    if ui.rect_contains_pointer(area.inner_rect) && !ui.ctx().egui_wants_keyboard_input() {
        changed |= follow_keys(ui, timeline, animation);
    }
    changed
}

/// The ruler, with a tick and its time at each step; pressing or dragging
/// it moves the playhead there and stops playing. Returns where it is.
fn show_ruler(ui: &mut egui::Ui, timeline: &mut Timeline, length: f32) -> egui::Rect {
    ui.horizontal(|ui| {
        ui.add_space(NAMES_WIDTH);
        let width = ui.available_width().max(MIN_LANES_WIDTH);
        let (ruler, response) = ui.allocate_exact_size(
            egui::vec2(width, RULER_HEIGHT),
            egui::Sense::click_and_drag(),
        );
        let scale = Scale::new(ruler, length);
        if let Some(pointer) = response.interact_pointer_pos()
            && (response.is_pointer_button_down_on() || response.clicked())
        {
            timeline.time = scale.time_at(pointer.x);
            timeline.playing = false;
        }
        draw_ticks(ui, ruler, scale);
        ruler
    })
    .inner
}

fn draw_ticks(ui: &egui::Ui, ruler: egui::Rect, scale: Scale) {
    let painter = ui.painter_at(ruler.expand(LANE_MARGIN));
    let color = ui.visuals().weak_text_color();
    let pixels_per_second = scale.width / scale.length;
    let step = TICK_STEPS
        .into_iter()
        .find(|step| step * pixels_per_second >= MIN_TICK_GAP)
        .unwrap_or(scale.length);
    let mut tick = 0.0;
    while tick <= scale.length + step / 2.0 {
        let x = scale.x_of(tick.min(scale.length));
        painter.line_segment(
            [
                egui::pos2(x, ruler.bottom() - TICK_HEIGHT),
                egui::pos2(x, ruler.bottom()),
            ],
            egui::Stroke::new(1.0, color),
        );
        painter.text(
            egui::pos2(x + 2.0, ruler.top()),
            egui::Align2::LEFT_TOP,
            format!("{tick:.2}"),
            egui::FontId::proportional(10.0),
            color,
        );
        tick += step;
    }
}

/// The lane of the track `track` (or of the calls, if `None`), with its
/// marks drawn.
fn show_lane(
    ui: &mut egui::Ui,
    timeline: &Timeline,
    animation: &TrackAnimation,
    track: Option<usize>,
    scale: Scale,
) -> Lane {
    let width = ui.available_width().max(MIN_LANES_WIDTH);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(width, LANE_HEIGHT),
        egui::Sense::click_and_drag(),
    );
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 2.0, LANE_FILL);
    let y = rect.center().y;
    let times: Vec<(Mark, f32)> = match track {
        Some(track) => animation.tracks[track]
            .keys
            .iter()
            .enumerate()
            .map(|(key, value)| (Mark::Key { track, key }, value.time))
            .collect(),
        None => animation
            .calls
            .iter()
            .enumerate()
            .map(|(call, value)| (Mark::Call(call), value.time))
            .collect(),
    };
    let marks: Vec<(Mark, egui::Pos2)> = times
        .into_iter()
        .map(|(mark, time)| (mark, egui::pos2(scale.x_of(time), y)))
        .collect();
    for &(mark, at) in &marks {
        let selected = timeline.selection.contains(&mark);
        let color = if selected {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().text_color()
        };
        match mark {
            Mark::Key { .. } => draw_key(&painter, at, color),
            Mark::Call(call) => {
                calls::draw_call(&painter, at, &animation.calls[call].method, color);
            }
        }
    }
    Lane {
        response,
        track,
        marks,
    }
}

/// A key: a diamond centered at `at`.
pub(super) fn draw_key(painter: &egui::Painter, at: egui::Pos2, color: egui::Color32) {
    let points = vec![
        at + egui::vec2(0.0, -KEY_RADIUS),
        at + egui::vec2(KEY_RADIUS, 0.0),
        at + egui::vec2(0.0, KEY_RADIUS),
        at + egui::vec2(-KEY_RADIUS, 0.0),
    ];
    painter.add(egui::Shape::convex_polygon(
        points,
        color,
        egui::Stroke::NONE,
    ));
}

fn draw_playhead(ui: &egui::Ui, scale: Scale, time: f32, top: f32, bottom: f32) {
    let x = scale.x_of(time);
    ui.painter().line_segment(
        [egui::pos2(x, top), egui::pos2(x, bottom)],
        egui::Stroke::new(PLAYHEAD_WIDTH, PLAYHEAD_COLOR),
    );
}

/// The mark of `lane` nearest to `at`, if one is close enough to take.
fn mark_at(lane: &Lane, at: egui::Pos2) -> Option<Mark> {
    lane.marks
        .iter()
        .map(|&(mark, place)| (mark, place.distance(at)))
        .filter(|&(_, distance)| distance <= HIT_RADIUS)
        .min_by(|first, second| first.1.total_cmp(&second.1))
        .map(|(mark, _)| mark)
}

/// What the pointer does on `lane`: select, drag and add marks. Returns
/// whether the animation changed.
fn follow_pointer(
    ui: &egui::Ui,
    timeline: &mut Timeline,
    animation: &mut TrackAnimation,
    lane: &Lane,
    scale: Scale,
) -> bool {
    let response = &lane.response;
    let adding = ui.input(|input| input.modifiers.shift || input.modifiers.command);
    if response.drag_started()
        && let Some(origin) = ui.input(|input| input.pointer.press_origin())
    {
        if let Some(mark) = mark_at(lane, origin) {
            if !timeline.selection.contains(&mark) {
                select(timeline, mark, adding);
            }
            timeline.drag = Some(KeyDrag {
                from: scale.time_at(origin.x),
                original: animation.clone(),
                marks: timeline.selection.clone(),
            });
        }
        return false;
    }
    if response.drag_stopped() {
        timeline.drag = None;
        return false;
    }
    if response.dragged() {
        return drag_marks(timeline, animation, response, scale);
    }
    let Some(pointer) = response.interact_pointer_pos() else {
        return false;
    };
    if response.double_clicked() {
        return add_key_at(timeline, animation, lane, scale.time_at(pointer.x));
    }
    if response.clicked() {
        match mark_at(lane, pointer) {
            Some(mark) if adding && timeline.selection.contains(&mark) => {
                timeline.selection.retain(|&selected| selected != mark);
            }
            Some(mark) => select(timeline, mark, adding),
            None if !adding => {
                timeline.selection.clear();
                timeline.time = scale.time_at(pointer.x);
                timeline.playing = false;
            }
            None => {}
        }
    }
    false
}

fn select(timeline: &mut Timeline, mark: Mark, adding: bool) {
    if !adding {
        timeline.selection.clear();
    }
    timeline.selection.push(mark);
}

/// Moves what is being dragged as far as the pointer went since the drag
/// started, from where it was then.
fn drag_marks(
    timeline: &mut Timeline,
    animation: &mut TrackAnimation,
    response: &egui::Response,
    scale: Scale,
) -> bool {
    let (Some(drag), Some(pointer)) = (&timeline.drag, response.interact_pointer_pos()) else {
        return false;
    };
    let shift = scale.time_at(pointer.x) - drag.from;
    let mut moved = drag.original.clone();
    let marks = edits::move_marks(&mut moved, &drag.marks, shift);
    if moved == *animation {
        return false;
    }
    *animation = moved;
    timeline.selection = marks;
    true
}

/// A key at `time` in the track of `lane`, with the value the track has
/// then; calls are added from their menu instead.
fn add_key_at(
    timeline: &mut Timeline,
    animation: &mut TrackAnimation,
    lane: &Lane,
    time: f32,
) -> bool {
    let Some(track) = lane.track.and_then(|track| animation.tracks.get(track)) else {
        return false;
    };
    let (node, property) = (track.node.clone(), track.property.clone());
    let Some(value) = track.value_at(time) else {
        return false;
    };
    let Some(mark) = edits::set_key(animation, &node, &property, time, value) else {
        return false;
    };
    timeline.selection = vec![mark];
    true
}

/// `Delete`, `Ctrl+C` and `Ctrl+V` on the marks selected, while the
/// pointer is over the lanes.
fn follow_keys(ui: &egui::Ui, timeline: &mut Timeline, animation: &mut TrackAnimation) -> bool {
    let (delete, copy, paste) = ui.input(|input| {
        let event = |wanted: fn(&egui::Event) -> bool| input.events.iter().any(wanted);
        (
            input.key_pressed(egui::Key::Delete),
            event(|event| matches!(event, egui::Event::Copy)),
            event(|event| matches!(event, egui::Event::Paste(_))),
        )
    });
    if copy && !timeline.selection.is_empty() {
        timeline.copied = edits::copy_marks(animation, &timeline.selection);
    }
    if delete && !timeline.selection.is_empty() {
        edits::delete_marks(animation, &timeline.selection);
        timeline.selection.clear();
        return true;
    }
    if paste && !timeline.copied.is_empty() {
        timeline.selection = edits::paste(animation, &timeline.copied, timeline.time);
        return true;
    }
    false
}

/// Deletes the track `track` with its keys.
fn remove_track(timeline: &mut Timeline, animation: &mut TrackAnimation, track: usize) {
    let keys = animation.tracks[track].keys.len();
    let marks: Vec<Mark> = (0..keys).map(|key| Mark::Key { track, key }).collect();
    edits::delete_marks(animation, &marks);
    timeline.selection.clear();
}

/// The node at `path` from `root`, named for people: the animated node's
/// own name for the empty path.
pub(super) fn node_label(nodes: &[NodeOutline], root: usize, path: &str) -> String {
    if path.is_empty() {
        nodes
            .get(root)
            .map_or_else(String::new, |node| node.name.clone())
    } else {
        path.to_owned()
    }
}

/// Whether the node at `path` is under `root`.
pub(super) fn node_exists(nodes: &[NodeOutline], root: usize, path: &str) -> bool {
    paths::node_at(nodes, root, path).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scales_turn_times_into_places_and_back() {
        let lanes = egui::Rect::from_min_size(egui::pos2(100.0, 0.0), egui::vec2(216.0, 20.0));
        let scale = Scale::new(lanes, 2.0);

        assert_eq!(scale.x_of(0.0), 108.0);
        assert_eq!(scale.x_of(2.0), 308.0);
        assert_eq!(scale.time_at(208.0), 1.0);
        assert_eq!(scale.time_at(0.0), 0.0);
        assert_eq!(scale.time_at(1000.0), 2.0);
    }
}
