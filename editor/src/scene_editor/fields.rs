//! Rows of the node inspector that edit one value: numbers, vectors,
//! colors and choices. Each returns whether the user changed the value.

use eframe::egui;
use star_engine::prelude::{Color, Vec2, Vec3};

use super::key_buttons::{self, RowName};

/// How much a number changes per pixel dragged.
pub(super) const DRAG_SPEED: f32 = 0.01;
/// Angles change faster: one degree per pixel dragged.
const DEGREES_SPEED: f32 = 1.0;
/// The smallest size a shape can have; shapes need positive sizes.
pub(super) const MIN_SIZE: f32 = 0.01;
/// The largest value of a color channel, in bytes.
const CHANNEL_MAX: f32 = 255.0;

/// The faint name that starts a row, after its key button if it has one.
/// Returns the name.
fn label<'row>(ui: &mut egui::Ui, name: impl Into<RowName<'row>>) -> &'row str {
    key_buttons::show_name(ui, name)
}

/// A number that can be any value.
pub(super) fn number_row<'row>(
    ui: &mut egui::Ui,
    name: impl Into<RowName<'row>>,
    value: &mut f32,
) -> bool {
    label(ui, name);
    let changed = number(ui, value);
    ui.end_row();
    changed
}

/// A number from `min` up.
pub(super) fn number_from_row<'row>(
    ui: &mut egui::Ui,
    name: impl Into<RowName<'row>>,
    value: &mut f32,
    min: f32,
) -> bool {
    label(ui, name);
    let changed = ui
        .add(
            egui::DragValue::new(value)
                .speed(DRAG_SPEED)
                .range(min..=f32::MAX),
        )
        .changed();
    ui.end_row();
    changed
}

/// A whole number between `min` and `max`.
pub(super) fn whole_row<'row, Number: egui::emath::Numeric>(
    ui: &mut egui::Ui,
    name: impl Into<RowName<'row>>,
    value: &mut Number,
    range: std::ops::RangeInclusive<Number>,
) -> bool {
    label(ui, name);
    let changed = ui.add(egui::DragValue::new(value).range(range)).changed();
    ui.end_row();
    changed
}

/// A number from 0 to 1, as a slider.
pub(super) fn fraction_row<'row>(
    ui: &mut egui::Ui,
    name: impl Into<RowName<'row>>,
    value: &mut f32,
) -> bool {
    label(ui, name);
    let changed = ui.add(egui::Slider::new(value, 0.0..=1.0)).changed();
    ui.end_row();
    changed
}

/// An angle kept in radians, edited in degrees, which people read more
/// easily.
pub(super) fn angle_row<'row>(
    ui: &mut egui::Ui,
    name: impl Into<RowName<'row>>,
    radians: &mut f32,
) -> bool {
    label(ui, name);
    let changed = angle(ui, radians);
    ui.end_row();
    changed
}

/// Three angles kept in radians, edited in degrees.
pub(super) fn angles_row<'row>(
    ui: &mut egui::Ui,
    name: impl Into<RowName<'row>>,
    radians: &mut Vec3,
) -> bool {
    label(ui, name);
    let changed = ui
        .horizontal(|ui| {
            angle(ui, &mut radians.x) | angle(ui, &mut radians.y) | angle(ui, &mut radians.z)
        })
        .inner;
    ui.end_row();
    changed
}

pub(super) fn vec2_row<'row>(
    ui: &mut egui::Ui,
    name: impl Into<RowName<'row>>,
    value: &mut Vec2,
) -> bool {
    label(ui, name);
    let changed = ui
        .horizontal(|ui| number(ui, &mut value.x) | number(ui, &mut value.y))
        .inner;
    ui.end_row();
    changed
}

pub(super) fn vec3_row<'row>(
    ui: &mut egui::Ui,
    name: impl Into<RowName<'row>>,
    value: &mut Vec3,
) -> bool {
    label(ui, name);
    let changed = ui
        .horizontal(|ui| {
            number(ui, &mut value.x) | number(ui, &mut value.y) | number(ui, &mut value.z)
        })
        .inner;
    ui.end_row();
    changed
}

/// A size along each axis, every one at least [`MIN_SIZE`].
pub(super) fn size2_row<'row>(
    ui: &mut egui::Ui,
    name: impl Into<RowName<'row>>,
    value: &mut Vec2,
) -> bool {
    label(ui, name);
    let changed = ui
        .horizontal(|ui| size(ui, &mut value.x) | size(ui, &mut value.y))
        .inner;
    ui.end_row();
    changed
}

/// A size along each axis, every one at least [`MIN_SIZE`].
pub(super) fn size3_row<'row>(
    ui: &mut egui::Ui,
    name: impl Into<RowName<'row>>,
    value: &mut Vec3,
) -> bool {
    label(ui, name);
    let changed = ui
        .horizontal(|ui| size(ui, &mut value.x) | size(ui, &mut value.y) | size(ui, &mut value.z))
        .inner;
    ui.end_row();
    changed
}

/// A color with its opacity, picked with egui's color picker. Colors are
/// sRGB in scenes and in the picker, so only their range changes.
pub(super) fn color_row<'row>(
    ui: &mut egui::Ui,
    name: impl Into<RowName<'row>>,
    color: &mut Color,
) -> bool {
    label(ui, name);
    let mut bytes = [color.r, color.g, color.b, color.a].map(to_byte);
    let changed = ui
        .color_edit_button_srgba_unmultiplied(&mut bytes)
        .changed();
    if changed {
        let [r, g, b, a] = bytes.map(from_byte);
        *color = Color::rgba(r, g, b, a);
    }
    ui.end_row();
    changed
}

pub(super) fn checkbox_row<'row>(
    ui: &mut egui::Ui,
    name: impl Into<RowName<'row>>,
    value: &mut bool,
) -> bool {
    label(ui, name);
    let changed = ui.checkbox(value, "").changed();
    ui.end_row();
    changed
}

/// A line of text.
pub(super) fn text_row<'row>(
    ui: &mut egui::Ui,
    name: impl Into<RowName<'row>>,
    text: &mut String,
    hint: &str,
) -> bool {
    label(ui, name);
    let changed = ui
        .add(egui::TextEdit::singleline(text).hint_text(hint))
        .changed();
    ui.end_row();
    changed
}

/// One of `choices`, each with its name, in a drop-down list.
pub(super) fn choice_row<'row, Choice: Copy + PartialEq>(
    ui: &mut egui::Ui,
    name: impl Into<RowName<'row>>,
    value: &mut Choice,
    choices: &[(Choice, &str)],
) -> bool {
    let id = label(ui, name);
    let before = *value;
    let shown = choices
        .iter()
        .find_map(|(choice, text)| (*choice == before).then_some(*text))
        .unwrap_or_default();
    egui::ComboBox::from_id_salt(id)
        .selected_text(shown)
        .show_ui(ui, |ui| {
            for (choice, text) in choices {
                ui.selectable_value(value, *choice, *text);
            }
        });
    ui.end_row();
    *value != before
}

/// A row with an angle shown in degrees, on a slider between `degrees`,
/// kept in radians.
pub(super) fn angle_slider_row<'row>(
    ui: &mut egui::Ui,
    name: impl Into<RowName<'row>>,
    radians: &mut f32,
    degrees: std::ops::RangeInclusive<f32>,
) -> bool {
    label(ui, name);
    let mut shown = radians.to_degrees();
    let changed = ui
        .add(egui::Slider::new(&mut shown, degrees).suffix("°"))
        .changed();
    if changed {
        *radians = shown.to_radians();
    }
    ui.end_row();
    changed
}

/// A row with a checkbox that explains itself with `hint`.
pub(super) fn checkbox_hint_row<'row>(
    ui: &mut egui::Ui,
    name: impl Into<RowName<'row>>,
    value: &mut bool,
    hint: &str,
) -> bool {
    label(ui, name);
    let changed = ui.checkbox(value, "").on_hover_text(hint).changed();
    ui.end_row();
    changed
}

/// A row with the tags of a node, each with a button that removes it, and
/// a field where a new one is written; Enter adds it.
pub(super) fn tags_row<'row>(
    ui: &mut egui::Ui,
    name: impl Into<RowName<'row>>,
    tags: &mut Vec<String>,
    hint: &str,
) -> bool {
    label(ui, name);
    let mut changed = false;
    ui.vertical(|ui| {
        ui.horizontal_wrapped(|ui| {
            let mut removed = None;
            for (index, tag) in tags.iter().enumerate() {
                if ui.small_button(format!("{tag} ×")).clicked() {
                    removed = Some(index);
                }
            }
            if let Some(index) = removed {
                tags.remove(index);
                changed = true;
            }
        });
        let id = ui.scope_id().with("new_tag");
        let mut new_tag: String = ui.data_mut(|data| data.get_temp(id).unwrap_or_default());
        let response = ui.add(egui::TextEdit::singleline(&mut new_tag).hint_text(hint));
        let entered =
            response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        if entered {
            if let Some(tag) = add_tag(tags, &new_tag) {
                tags.push(tag);
                changed = true;
            }
            new_tag.clear();
        }
        ui.data_mut(|data| data.insert_temp(id, new_tag));
    });
    ui.end_row();
    changed
}

/// The tag `written` adds to `tags`: without the blanks around it, and
/// none if it is empty or already there.
fn add_tag(tags: &[String], written: &str) -> Option<String> {
    let tag = written.trim();
    (!tag.is_empty() && !tags.iter().any(|own| own == tag)).then(|| tag.to_owned())
}

fn number(ui: &mut egui::Ui, value: &mut f32) -> bool {
    ui.add(egui::DragValue::new(value).speed(DRAG_SPEED))
        .changed()
}

fn size(ui: &mut egui::Ui, value: &mut f32) -> bool {
    ui.add(
        egui::DragValue::new(value)
            .speed(DRAG_SPEED)
            .range(MIN_SIZE..=f32::MAX),
    )
    .changed()
}

fn angle(ui: &mut egui::Ui, radians: &mut f32) -> bool {
    let mut degrees = radians.to_degrees();
    let changed = ui
        .add(
            egui::DragValue::new(&mut degrees)
                .speed(DEGREES_SPEED)
                .suffix("°"),
        )
        .changed();
    if changed {
        *radians = degrees.to_radians();
    }
    changed
}

/// A color channel from 0 to 1 as a byte.
// The value is clamped to 0–255 first, so it fits a byte with no sign.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn to_byte(channel: f32) -> u8 {
    (channel.clamp(0.0, 1.0) * CHANNEL_MAX).round() as u8
}

fn from_byte(byte: u8) -> f32 {
    f32::from(byte) / CHANNEL_MAX
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tags_are_trimmed_and_never_repeated() {
        let tags = vec!["enemy".to_owned()];

        assert_eq!(add_tag(&tags, "  boss "), Some("boss".to_owned()));
        assert_eq!(add_tag(&tags, "enemy"), None);
        assert_eq!(add_tag(&tags, "   "), None);
    }

    #[test]
    fn channels_go_to_bytes_and_back() {
        assert_eq!(to_byte(1.0), 255);
        assert_eq!(to_byte(2.0), 255);
        assert_eq!(to_byte(-1.0), 0);
        assert!((from_byte(to_byte(0.5)) - 0.5).abs() < 1.0 / CHANNEL_MAX);
    }
}
