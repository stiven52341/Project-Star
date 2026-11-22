//! Properties of the selected element; for now, of the project itself.

use eframe::egui;
use star_engine::prelude::{ENGINE_NAME, ENGINE_VERSION};

use crate::project::Project;
use crate::texts;

const PROPERTY_SPACING: [f32; 2] = [12.0, 8.0];

pub(crate) fn show_inspector(ui: &mut egui::Ui, project: &Project) {
    ui.strong(texts::INSPECTOR);
    ui.separator();
    egui::CollapsingHeader::new(texts::PROJECT)
        .default_open(true)
        .show(ui, |ui| show_project_properties(ui, project));
}

fn show_project_properties(ui: &mut egui::Ui, project: &Project) {
    let settings = project.settings();
    let platforms = settings
        .platforms()
        .iter()
        .map(|platform| platform.name())
        .collect::<Vec<_>>()
        .join(", ");
    egui::Grid::new("project_properties")
        .num_columns(2)
        .spacing(PROPERTY_SPACING)
        .show(ui, |ui| {
            property(ui, texts::PROJECT_NAME, settings.name());
            property(ui, texts::DIMENSION, settings.dimension().label());
            property(ui, texts::PLATFORMS, &platforms);
            property(
                ui,
                texts::ENGINE,
                &format!("{ENGINE_NAME} {ENGINE_VERSION}"),
            );
            property(
                ui,
                texts::FOLDER,
                &project.directory().display().to_string(),
            );
        });
}

fn property(ui: &mut egui::Ui, name: &str, value: &str) {
    ui.weak(name);
    ui.add(egui::Label::new(value).wrap());
    ui.end_row();
}
