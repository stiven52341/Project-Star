//! Properties of the selected element: the node selected in the scene,
//! or the project itself when there is none.

use eframe::egui;
use star_engine::prelude::{ENGINE_NAME, ENGINE_VERSION};

use crate::project::Project;
use crate::scene_editor::SceneDocument;
use crate::style::{PROPERTY_SPACING, property_row};
use crate::texts;

/// The node selected in `scene`, to edit with the `images` of the assets
/// folder, or the project if none is.
pub(crate) fn show_inspector(
    ui: &mut egui::Ui,
    project: &Project,
    scene: Option<&mut SceneDocument>,
    images: &[String],
) {
    ui.strong(texts::current().inspector);
    ui.separator();
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            let edited = scene.is_some_and(|scene| scene.edit_selected_node(ui, images));
            if !edited {
                egui::CollapsingHeader::new(texts::current().project)
                    .default_open(true)
                    .show(ui, |ui| show_project_properties(ui, project));
            }
        });
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
            property_row(ui, texts::current().project_name, settings.name());
            property_row(ui, texts::current().dimension, settings.dimension().label());
            property_row(ui, texts::current().platforms, &platforms);
            property_row(
                ui,
                texts::current().engine,
                &format!("{ENGINE_NAME} {ENGINE_VERSION}"),
            );
            property_row(
                ui,
                texts::current().folder,
                &project.directory().display().to_string(),
            );
        });
}
