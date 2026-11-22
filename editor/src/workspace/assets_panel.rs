//! Browser of the files inside the project's `assets` folder.

use std::io;
use std::path::{Path, PathBuf};

use eframe::egui;

use crate::blueprint_editor::create_blueprint;
use crate::file_tree::{FileNode, scan_directory};
use crate::project::{ASSETS_DIRECTORY, BLUEPRINT_EXTENSION};
use crate::style::ERROR_COLOR;
use crate::texts;

/// Contents of the assets folder, refreshed on demand.
pub(crate) struct AssetsPanel {
    directory: PathBuf,
    contents: io::Result<Vec<FileNode>>,
    /// The form of «Nuevo blueprint…», while it is open.
    new_blueprint: Option<NewBlueprintForm>,
}

/// The class of the blueprint to create, and why the last try failed.
#[derive(Default)]
struct NewBlueprintForm {
    class: String,
    error: Option<String>,
}

impl AssetsPanel {
    pub(crate) fn new(project_directory: &Path) -> Self {
        let directory = project_directory.join(ASSETS_DIRECTORY);
        let contents = scan_directory(&directory);
        Self {
            directory,
            contents,
            new_blueprint: None,
        }
    }

    fn refresh(&mut self) {
        self.contents = scan_directory(&self.directory);
    }

    /// The folder it lists.
    pub(crate) fn directory(&self) -> &Path {
        &self.directory
    }

    /// Lists the files. Returns the blueprint the user opened (double
    /// click) or created, as a path inside the folder.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) -> Option<String> {
        let mut opened = None;
        ui.horizontal(|ui| {
            ui.weak(format!("{ASSETS_DIRECTORY}/"));
            if ui.small_button(texts::REFRESH).clicked() {
                self.refresh();
            }
            if ui.small_button(texts::NEW_BLUEPRINT).clicked() {
                self.new_blueprint = Some(NewBlueprintForm::default());
            }
        });
        if let Some(created) = self.show_new_blueprint_form(ui) {
            self.refresh();
            opened = Some(created);
        }
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| match &self.contents {
                Ok(nodes) if nodes.is_empty() => {
                    ui.weak(texts::EMPTY_ASSETS);
                }
                Ok(nodes) => show_nodes(ui, nodes, "", &mut opened),
                Err(error) => {
                    ui.colored_label(ERROR_COLOR, texts::assets_unreadable(error));
                }
            });
        opened
    }
}

/// Lists `nodes`, inside the folder `prefix` (`""` or `"enemies/"`); a
/// blueprint double clicked is put in `opened`.
impl AssetsPanel {
    /// The form to name a new blueprint; returns the one created.
    fn show_new_blueprint_form(&mut self, ui: &mut egui::Ui) -> Option<String> {
        let form = self.new_blueprint.as_mut()?;
        let mut created = None;
        let mut close = false;
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut form.class).hint_text(texts::CLASS_NAME_HINT));
            if ui.button(texts::CREATE).clicked() {
                match create_blueprint(&self.directory, &form.class) {
                    Ok(path) => {
                        created = Some(path);
                        close = true;
                    }
                    Err(error) => form.error = Some(texts::new_blueprint_error(&error)),
                }
            }
            if ui.button(texts::CANCEL).clicked() {
                close = true;
            }
        });
        if let Some(error) = &form.error {
            ui.colored_label(ERROR_COLOR, error);
        }
        if close {
            self.new_blueprint = None;
        }
        created
    }
}

fn show_nodes(ui: &mut egui::Ui, nodes: &[FileNode], prefix: &str, opened: &mut Option<String>) {
    for node in nodes {
        match node {
            FileNode::Folder {
                name,
                path,
                children,
            } => {
                egui::CollapsingHeader::new(name.as_str())
                    .id_salt(path)
                    .show(ui, |ui| {
                        show_nodes(ui, children, &format!("{prefix}{name}/"), opened);
                    });
            }
            FileNode::File { name } if name.ends_with(BLUEPRINT_EXTENSION) => {
                let response = ui
                    .add(egui::Label::new(name.as_str()).sense(egui::Sense::click()))
                    .on_hover_text(texts::OPEN_BLUEPRINT_HINT);
                if response.double_clicked() {
                    *opened = Some(format!("{prefix}{name}"));
                }
            }
            FileNode::File { name } => {
                ui.label(name.as_str());
            }
        }
    }
}
