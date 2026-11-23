//! Visual constants and small widgets shared by every editor screen.

use eframe::egui;

pub(crate) const TITLE_SIZE: f32 = 28.0;
pub(crate) const SECTION_SPACING: f32 = 12.0;
pub(crate) const ERROR_COLOR: egui::Color32 = egui::Color32::from_rgb(235, 100, 90);
pub(crate) const SUCCESS_COLOR: egui::Color32 = egui::Color32::from_rgb(110, 200, 120);
/// Space between the columns and the rows of a grid of properties.
pub(crate) const PROPERTY_SPACING: [f32; 2] = [12.0, 8.0];

/// A row of a grid of properties that are only shown: its name, faint, and
/// its value.
pub(crate) fn property_row(ui: &mut egui::Ui, name: &str, value: &str) {
    ui.weak(name);
    ui.add(egui::Label::new(value).wrap());
    ui.end_row();
}
