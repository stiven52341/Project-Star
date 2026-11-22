//! The field of a constant: its kind and its value, side by side.

use eframe::egui;
use star_blueprint::Literal;

use super::names::LiteralKind;

/// How much a decimal changes per point dragged.
const DECIMAL_DRAG_SPEED: f64 = 0.1;

/// Edits `value`; `id_salt` tells it apart from other constants shown
/// together. Returns whether it changed.
pub(super) fn literal_field(
    ui: &mut egui::Ui,
    id_salt: impl egui::AsIdSalt,
    value: &mut Literal,
) -> bool {
    let current = LiteralKind::of(value);
    let mut kind = current;
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt(id_salt)
            .selected_text(kind.name())
            .show_ui(ui, |ui| {
                for option in LiteralKind::ALL {
                    ui.selectable_value(&mut kind, option, option.name());
                }
            });
        if kind != current {
            *value = kind.default_value();
            return true;
        }
        let response = match value {
            Literal::Bool(value) => ui.checkbox(value, ""),
            Literal::Int(value) => ui.add(egui::DragValue::new(value)),
            Literal::Float(value) => ui.add(egui::DragValue::new(value).speed(DECIMAL_DRAG_SPEED)),
            Literal::String(text) => ui.text_edit_singleline(text),
        };
        response.changed()
    })
    .inner
}
