//! List of recently opened projects.

use std::path::{Path, PathBuf};

use eframe::egui;

use crate::project::{Project, open_project};
use crate::texts;

const ROW_SPACING: f32 = 6.0;

/// What the user asked to do with an entry of the list.
pub(crate) enum RecentAction {
    Open(PathBuf),
    Remove(PathBuf),
}

/// A remembered project folder, with its settings if it can still be opened.
pub(crate) struct RecentEntry {
    directory: PathBuf,
    project: Option<Project>,
}

impl RecentEntry {
    pub(crate) fn load(directory: &Path) -> Self {
        Self {
            directory: directory.to_path_buf(),
            project: open_project(directory).ok(),
        }
    }
}

/// Draws `entries` and returns the action the user chose this frame, if any.
pub(crate) fn show_recent_list(ui: &mut egui::Ui, entries: &[RecentEntry]) -> Option<RecentAction> {
    if entries.is_empty() {
        ui.weak(texts::NO_RECENT_PROJECTS);
        return None;
    }
    let mut action = None;
    egui::ScrollArea::vertical()
        .auto_shrink([false, true])
        .show(ui, |ui| {
            for entry in entries {
                if let Some(chosen) = show_entry(ui, entry) {
                    action = Some(chosen);
                }
                ui.add_space(ROW_SPACING);
            }
        });
    action
}

fn show_entry(ui: &mut egui::Ui, entry: &RecentEntry) -> Option<RecentAction> {
    egui::Frame::group(ui.style())
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            match &entry.project {
                Some(project) => show_available(ui, project),
                None => show_unavailable(ui, &entry.directory),
            }
        })
        .inner
}

fn show_available(ui: &mut egui::Ui, project: &Project) -> Option<RecentAction> {
    let settings = project.settings();
    let mut action = None;
    ui.horizontal(|ui| {
        ui.strong(settings.name());
        ui.weak(settings.dimension().label());
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(texts::OPEN).clicked() {
                action = Some(RecentAction::Open(project.directory().to_path_buf()));
            }
        });
    });
    ui.small(project.directory().display().to_string());
    action
}

fn show_unavailable(ui: &mut egui::Ui, directory: &Path) -> Option<RecentAction> {
    let mut action = None;
    ui.horizontal(|ui| {
        ui.weak(texts::folder_name(directory));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let remove = ui
                .button(texts::REMOVE_FROM_LIST)
                .on_hover_text(texts::REMOVE_FROM_LIST_HINT);
            if remove.clicked() {
                action = Some(RecentAction::Remove(directory.to_path_buf()));
            }
        });
    });
    ui.small(texts::PROJECT_UNAVAILABLE);
    action
}
