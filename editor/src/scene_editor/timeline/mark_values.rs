//! The values of the key or call selected, over the lanes: its time, and
//! the value of a key or the method of a call, to change them by hand.

use eframe::egui;
use star_engine::star_scene::{FieldValue, TrackAnimation};

use super::Timeline;
use super::edits::{self, Mark};
use crate::texts;

/// How much a time or a decimal changes per pixel dragged.
const DRAG_SPEED: f64 = 0.01;

/// Shows what is selected, if anything, to edit. Returns whether the
/// animation changed.
pub(super) fn show(
    ui: &mut egui::Ui,
    timeline: &mut Timeline,
    animation: &mut TrackAnimation,
) -> bool {
    let texts = texts::current();
    ui.horizontal(|ui| match timeline.selection.as_slice() {
        [] => {
            ui.weak(texts.timeline_hint());
            false
        }
        &[mark] => {
            let moved = time_row(ui, timeline, animation, mark);
            // Moving it may have given it another place in its track.
            let Some(&mark) = timeline.selection.first() else {
                return moved;
            };
            moved | value_row(ui, animation, mark)
        }
        marks => {
            ui.weak(texts.marks_selected(marks.len()));
            false
        }
    })
    .inner
}

/// The time of `mark`, to move it.
fn time_row(
    ui: &mut egui::Ui,
    timeline: &mut Timeline,
    animation: &mut TrackAnimation,
    mark: Mark,
) -> bool {
    let Some(time) = edits::mark_time(animation, mark) else {
        return false;
    };
    ui.weak(texts::current().key_time());
    let mut moved_to = time;
    let dragged = ui
        .add(
            egui::DragValue::new(&mut moved_to)
                .speed(DRAG_SPEED)
                .range(0.0..=animation.length)
                .suffix(" s"),
        )
        .changed();
    if !dragged || (moved_to - time).abs() < f32::EPSILON {
        return false;
    }
    timeline.selection = edits::move_marks(animation, &[mark], moved_to - time);
    true
}

/// The value of a key, or the method of a call.
fn value_row(ui: &mut egui::Ui, animation: &mut TrackAnimation, mark: Mark) -> bool {
    let texts = texts::current();
    match mark {
        Mark::Key { track, key } => {
            let Some(track) = animation.tracks.get_mut(track) else {
                return false;
            };
            ui.weak(texts.key_value(&track.property));
            track
                .keys
                .get_mut(key)
                .is_some_and(|key| edit_value(ui, &mut key.value))
        }
        Mark::Call(call) => {
            let Some(call) = animation.calls.get_mut(call) else {
                return false;
            };
            ui.weak(texts.call_method());
            let mut method = call.method.clone();
            let typed = ui.text_edit_singleline(&mut method).changed();
            // A call names a method: a blank name is not taken.
            if !typed || method.trim().is_empty() {
                return false;
            }
            method.trim().clone_into(&mut call.method);
            true
        }
    }
}

/// A value of the kind it has.
fn edit_value(ui: &mut egui::Ui, value: &mut FieldValue) -> bool {
    match value {
        FieldValue::Bool(truth) => ui.checkbox(truth, "").changed(),
        FieldValue::Int(number) => ui.add(egui::DragValue::new(number)).changed(),
        FieldValue::Float(number) => ui
            .add(egui::DragValue::new(number).speed(DRAG_SPEED))
            .changed(),
        FieldValue::Text(text) => ui.text_edit_singleline(text).changed(),
        FieldValue::Color(color) => ui.color_edit_button_rgba_unmultiplied(color).changed(),
    }
}
