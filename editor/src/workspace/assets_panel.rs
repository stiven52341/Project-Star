//! Browser of the files inside the project's `assets` folder.

use std::io;
use std::path::{Path, PathBuf};

use eframe::egui;
use star_engine::prelude::AssetPath;
use star_engine::star_assets::ImageFormat;

use crate::blueprint_editor::create_blueprint;
use crate::file_tree::{FileNode, file_paths, scan_directory};
use crate::project::{ASSETS_DIRECTORY, BLUEPRINT_EXTENSION, SCENE_EXTENSION};
use crate::style::ERROR_COLOR;
use crate::texts;

/// A file of the assets folder the user opened, as a path inside it.
pub(crate) enum OpenedAsset {
    Blueprint(String),
    Scene(String),
}

impl OpenedAsset {
    /// What `name` opens as, if the editor opens files like it.
    fn of(path: String) -> Option<Self> {
        if path.ends_with(BLUEPRINT_EXTENSION) {
            Some(Self::Blueprint(path))
        } else if path.ends_with(SCENE_EXTENSION) {
            Some(Self::Scene(path))
        } else {
            None
        }
    }
}

/// Contents of the assets folder, refreshed on demand.
pub(crate) struct AssetsPanel {
    directory: PathBuf,
    contents: io::Result<Vec<FileNode>>,
    /// The images among the contents, as paths inside the folder.
    images: Vec<String>,
    /// The form of “New blueprint…”, while it is open.
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
            images: image_paths(&contents),
            directory,
            contents,
            new_blueprint: None,
        }
    }

    fn refresh(&mut self) {
        self.contents = scan_directory(&self.directory);
        self.images = image_paths(&self.contents);
    }

    /// The images of the folder (those the engine can load), as paths
    /// inside it, as they were when it was last listed.
    pub(crate) fn images(&self) -> &[String] {
        &self.images
    }

    /// The folder it lists.
    pub(crate) fn directory(&self) -> &Path {
        &self.directory
    }

    /// Lists the files. Returns the blueprint or scene the user opened
    /// (double click), or the blueprint they created.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) -> Option<OpenedAsset> {
        let mut opened = None;
        ui.horizontal(|ui| {
            ui.weak(format!("{ASSETS_DIRECTORY}/"));
            if ui.small_button(texts::current().refresh).clicked() {
                self.refresh();
            }
            if ui.small_button(texts::current().new_blueprint).clicked() {
                self.new_blueprint = Some(NewBlueprintForm::default());
            }
        });
        if let Some(created) = self.show_new_blueprint_form(ui) {
            self.refresh();
            opened = Some(OpenedAsset::Blueprint(created));
        }
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| match &self.contents {
                Ok(nodes) if nodes.is_empty() => {
                    ui.weak(texts::current().empty_assets);
                }
                Ok(nodes) => show_nodes(ui, nodes, "", &mut opened),
                Err(error) => {
                    ui.colored_label(ERROR_COLOR, texts::current().assets_unreadable(error));
                }
            });
        opened
    }
}

impl AssetsPanel {
    /// The form to name a new blueprint; returns the one created.
    fn show_new_blueprint_form(&mut self, ui: &mut egui::Ui) -> Option<String> {
        let form = self.new_blueprint.as_mut()?;
        let mut created = None;
        let mut close = false;
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut form.class)
                    .hint_text(texts::current().class_name_hint),
            );
            if ui.button(texts::current().create).clicked() {
                match create_blueprint(&self.directory, &form.class) {
                    Ok(path) => {
                        created = Some(path);
                        close = true;
                    }
                    Err(error) => form.error = Some(texts::current().new_blueprint_error(&error)),
                }
            }
            if ui.button(texts::current().cancel).clicked() {
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

/// The paths of the images among `contents`.
fn image_paths(contents: &io::Result<Vec<FileNode>>) -> Vec<String> {
    let Ok(nodes) = contents else {
        return Vec::new();
    };
    file_paths(nodes)
        .into_iter()
        .filter(|path| {
            AssetPath::new(path)
                .and_then(|path| ImageFormat::of(&path))
                .is_ok()
        })
        .collect()
}

/// Lists `nodes`, inside the folder `prefix` (`""` or `"enemies/"`); a
/// blueprint or scene double clicked is put in `opened`.
fn show_nodes(
    ui: &mut egui::Ui,
    nodes: &[FileNode],
    prefix: &str,
    opened: &mut Option<OpenedAsset>,
) {
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
            FileNode::File { name } => match OpenedAsset::of(format!("{prefix}{name}")) {
                Some(asset) => {
                    let response = ui
                        .add(egui::Label::new(name.as_str()).sense(egui::Sense::click()))
                        .on_hover_text(texts::current().open_file_hint);
                    if response.double_clicked() {
                        *opened = Some(asset);
                    }
                }
                None => {
                    ui.label(name.as_str());
                }
            },
        }
    }
}
