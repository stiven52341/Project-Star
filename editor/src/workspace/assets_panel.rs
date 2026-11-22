//! Browser of the files inside the project's `assets` folder.

use std::io;
use std::path::{Path, PathBuf};

use eframe::egui;

use crate::file_tree::{FileNode, scan_directory};
use crate::project::ASSETS_DIRECTORY;
use crate::style::ERROR_COLOR;
use crate::texts;

/// Contents of the assets folder, refreshed on demand.
pub(crate) struct AssetsPanel {
    directory: PathBuf,
    contents: io::Result<Vec<FileNode>>,
}

impl AssetsPanel {
    pub(crate) fn new(project_directory: &Path) -> Self {
        let directory = project_directory.join(ASSETS_DIRECTORY);
        let contents = scan_directory(&directory);
        Self {
            directory,
            contents,
        }
    }

    fn refresh(&mut self) {
        self.contents = scan_directory(&self.directory);
    }

    pub(crate) fn show(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.weak(format!("{ASSETS_DIRECTORY}/"));
            if ui.small_button(texts::REFRESH).clicked() {
                self.refresh();
            }
        });
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| match &self.contents {
                Ok(nodes) if nodes.is_empty() => {
                    ui.weak(texts::EMPTY_ASSETS);
                }
                Ok(nodes) => show_nodes(ui, nodes),
                Err(error) => {
                    ui.colored_label(ERROR_COLOR, texts::assets_unreadable(error));
                }
            });
    }
}

fn show_nodes(ui: &mut egui::Ui, nodes: &[FileNode]) {
    for node in nodes {
        match node {
            FileNode::Folder {
                name,
                path,
                children,
            } => {
                egui::CollapsingHeader::new(name.as_str())
                    .id_salt(path)
                    .show(ui, |ui| show_nodes(ui, children));
            }
            FileNode::File { name } => {
                ui.label(name.as_str());
            }
        }
    }
}
