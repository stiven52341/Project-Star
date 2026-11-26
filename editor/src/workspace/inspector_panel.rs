//! Properties of the selected element: the node selected in the scene,
//! or the project itself when there is none. A script or blueprint
//! dropped on it gives the node selected its class.

use eframe::egui;
use star_engine::prelude::{ENGINE_NAME, ENGINE_VERSION};

use crate::drag_drop::released;
use star_script::semantic::ClassTable;

use crate::project::Project;
use crate::scene_editor::{AnimationFiles, ClassDrop, SceneDocument};
use crate::style::{PROPERTY_SPACING, property_row};
use crate::texts;

/// The width of the outline of the inspector while a script would be
/// dropped on the node selected.
const DROP_TARGET_WIDTH: f32 = 2.0;

/// The node selected in `scene`, to edit with the `images` of the assets
/// folder and the `classes` of the scripts (for its `export` fields), or
/// the project if none is.
pub(crate) fn show_inspector(
    ui: &mut egui::Ui,
    project: &Project,
    mut scene: Option<&mut SceneDocument>,
    images: &[String],
    animations: AnimationFiles<'_>,
    classes: Option<&ClassTable>,
) {
    let area = ui.max_rect();
    ui.strong(texts::current().inspector);
    ui.separator();
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            let edited = scene
                .as_deref_mut()
                .is_some_and(|scene| scene.edit_selected_node(ui, images, animations, classes));
            if !edited {
                egui::CollapsingHeader::new(texts::current().project)
                    .default_open(true)
                    .show(ui, |ui| show_project_properties(ui, project));
            }
        });
    if let Some(scene) = scene.filter(|scene| scene.has_selection()) {
        take_dropped_class(ui, area, scene);
    }
}

/// Outlines `area` while a script or blueprint with classes is held over
/// it, and gives the node selected in `scene` the class of one dropped.
fn take_dropped_class(ui: &egui::Ui, area: egui::Rect, scene: &mut SceneDocument) {
    let response = ui.interact(
        area,
        egui::Id::unique("inspector_class_drop"),
        egui::Sense::hover(),
    );
    let held = response
        .dnd_hover_payload::<ClassDrop>()
        .is_some_and(|script| !script.classes.is_empty());
    if held {
        ui.painter().rect_stroke(
            area,
            0.0,
            egui::Stroke::new(DROP_TARGET_WIDTH, ui.visuals().selection.stroke.color),
            egui::StrokeKind::Inside,
        );
    }
    if let Some(script) = released::<ClassDrop>(&response) {
        let at = response.hover_pos().unwrap_or_else(|| area.center());
        scene.drop_class_on_selected(&script, at);
    }
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
