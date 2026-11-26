//! The key buttons of the inspector: while the timeline is shown, each
//! property of a node the animator animates has a diamond beside its name
//! that puts a key with its value at the playhead. The diamond is filled
//! where the property has a key at the playhead, and drawn as an outline
//! where it has a track.

use std::cell::RefCell;

use eframe::egui;

use crate::texts;

/// How wide and tall a key button is.
const BUTTON_SIZE: f32 = 12.0;
/// Half the width of its diamond.
const DIAMOND_RADIUS: f32 = 4.5;
/// How thick the outline of a diamond is.
const OUTLINE_WIDTH: f32 = 1.2;

/// What the key buttons of one node show, and the properties whose button
/// was pressed. Rows only see it shared, so it keeps the presses inside.
#[derive(Debug, Default)]
pub(super) struct KeyButtons {
    /// The playhead, for the buttons' help.
    time: f32,
    /// The properties of the node with a track in the animation shown.
    tracked: Vec<String>,
    /// Those with a key at the playhead.
    keyed: Vec<String>,
    pressed: RefCell<Vec<String>>,
}

/// The name of a row, and its key button, if it has one.
#[derive(Clone, Copy)]
pub(super) struct RowName<'row> {
    pub(super) text: &'row str,
    key: Option<(&'row KeyButtons, &'row [&'row str])>,
}

impl<'row> From<&'row str> for RowName<'row> {
    fn from(text: &'row str) -> Self {
        Self { text, key: None }
    }
}

/// A row called `text` whose key button keys `properties`, if `keys` are
/// shown.
pub(super) fn keyed<'row>(
    keys: Option<&'row KeyButtons>,
    text: &'row str,
    properties: &'row [&'row str],
) -> RowName<'row> {
    RowName {
        text,
        key: keys.map(|keys| (keys, properties)),
    }
}

impl KeyButtons {
    /// The buttons of a node with tracks of `tracked`, and keys at `time`
    /// for `keyed`.
    pub(super) const fn new(time: f32, tracked: Vec<String>, keyed: Vec<String>) -> Self {
        Self {
            time,
            tracked,
            keyed,
            pressed: RefCell::new(Vec::new()),
        }
    }

    /// The properties whose button was pressed, each once.
    pub(super) fn into_pressed(self) -> Vec<String> {
        self.pressed.into_inner()
    }

    fn has_all(list: &[String], properties: &[&str]) -> bool {
        properties
            .iter()
            .all(|property| list.iter().any(|listed| listed == property))
    }

    /// The button that keys `properties`.
    pub(super) fn show(&self, ui: &mut egui::Ui, properties: &[&str]) {
        let (rect, response) =
            ui.allocate_exact_size(egui::Vec2::splat(BUTTON_SIZE), egui::Sense::click());
        let visuals = ui.visuals();
        let color = if response.hovered() {
            visuals.strong_text_color()
        } else if Self::has_all(&self.tracked, properties) {
            visuals.selection.stroke.color
        } else {
            visuals.weak_text_color()
        };
        let center = rect.center();
        let points = vec![
            center + egui::vec2(0.0, -DIAMOND_RADIUS),
            center + egui::vec2(DIAMOND_RADIUS, 0.0),
            center + egui::vec2(0.0, DIAMOND_RADIUS),
            center + egui::vec2(-DIAMOND_RADIUS, 0.0),
        ];
        let fill = if Self::has_all(&self.keyed, properties) {
            color
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter().add(egui::Shape::convex_polygon(
            points,
            fill,
            egui::Stroke::new(OUTLINE_WIDTH, color),
        ));
        let response = response.on_hover_text(texts::current().add_key_hint(self.time));
        if response.clicked() {
            let mut pressed = self.pressed.borrow_mut();
            for property in properties {
                if !pressed.iter().any(|done| done == property) {
                    pressed.push((*property).to_owned());
                }
            }
        }
    }
}

/// The faint name that starts a row, after its key button if it has one.
/// Returns the name.
pub(super) fn show_name<'row>(ui: &mut egui::Ui, name: impl Into<RowName<'row>>) -> &'row str {
    let name = name.into();
    match name.key {
        Some((keys, properties)) => {
            ui.horizontal(|ui| {
                keys.show(ui, properties);
                ui.weak(name.text);
            });
        }
        None => {
            ui.weak(name.text);
        }
    }
    name.text
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_know_which_properties_have_tracks_and_keys() {
        let keys = KeyButtons::new(
            0.5,
            vec!["x".to_owned(), "y".to_owned()],
            vec!["x".to_owned()],
        );

        assert!(KeyButtons::has_all(&keys.tracked, &["x", "y"]));
        assert!(!KeyButtons::has_all(&keys.keyed, &["x", "y"]));
        assert_eq!(keys.into_pressed(), Vec::<String>::new());
        let plain: RowName<'_> = "Name".into();
        assert!(plain.key.is_none());
        assert!(keyed(None, "Position", &["x"]).key.is_none());
    }
}
