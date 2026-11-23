//! Browser of the files inside the project's `assets` folder: one folder
//! at a time, its contents as a grid of icons (images as small copies of
//! themselves), with the way back to the top above them.

use std::io;
use std::path::{Path, PathBuf};

use eframe::egui;
use star_engine::prelude::AssetPath;
use star_engine::star_assets::ImageFormat;

use super::asset_icons::{AssetKind, paint_icon};
use super::thumbnails::Thumbnails;
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

/// A tile's size, and the size of the icon at its top.
const TILE_SIZE: egui::Vec2 = egui::vec2(84.0, 100.0);
const ICON_SIDE: f32 = 56.0;
/// Space between a tile's edges and its icon and name.
const TILE_PADDING: f32 = 6.0;
const TILE_ROUNDING: f32 = 4.0;
const NAME_SIZE: f32 = 12.0;
/// Lines a name takes at most under its icon.
const NAME_ROWS: usize = 2;
/// What separates folders in the way back to the top.
const PATH_SEPARATOR: &str = "›";

/// Contents of the assets folder, refreshed on demand.
pub(crate) struct AssetsPanel {
    directory: PathBuf,
    contents: io::Result<Vec<FileNode>>,
    /// The folder shown, by the names of the folders down to it.
    open_folder: Vec<String>,
    /// The file or folder clicked last, as a path inside the assets folder.
    selected: Option<String>,
    thumbnails: Thumbnails,
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
            thumbnails: Thumbnails::new(&directory),
            directory,
            contents,
            open_folder: Vec::new(),
            selected: None,
            new_blueprint: None,
        }
    }

    fn refresh(&mut self) {
        self.contents = scan_directory(&self.directory);
        self.images = image_paths(&self.contents);
        self.thumbnails.clear();
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

    /// Shows the folder open. Returns the blueprint or scene the user
    /// opened (double click), or the blueprint they created.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) -> Option<OpenedAsset> {
        let mut opened = None;
        ui.horizontal(|ui| {
            self.show_path(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button(texts::current().new_blueprint).clicked() {
                    self.new_blueprint = Some(NewBlueprintForm::default());
                }
                if ui.small_button(texts::current().refresh).clicked() {
                    self.refresh();
                }
            });
        });
        if let Some(created) = self.show_new_blueprint_form(ui) {
            self.refresh();
            opened = Some(OpenedAsset::Blueprint(created));
        }
        ui.separator();
        let entries = match &self.contents {
            Ok(nodes) => entries_of(nodes, &self.open_folder),
            Err(error) => {
                ui.colored_label(ERROR_COLOR, texts::current().assets_unreadable(error));
                return opened;
            }
        };
        let Some(entries) = entries else {
            // The folder shown is gone: back to the top.
            self.open_folder.clear();
            return opened;
        };
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                if entries.is_empty() {
                    ui.weak(texts::current().empty_assets);
                }
                ui.horizontal_wrapped(|ui| {
                    for entry in &entries {
                        if self.show_tile(ui, entry) {
                            opened = opened.take().or_else(|| self.open(entry));
                        }
                    }
                });
            });
        opened
    }

    /// The way from the assets folder down to the folder shown: each step
    /// goes back up to it when clicked.
    fn show_path(&mut self, ui: &mut egui::Ui) {
        if ui.small_button(ASSETS_DIRECTORY).clicked() {
            self.open_folder.clear();
        }
        let mut back_to = None;
        for (depth, name) in self.open_folder.iter().enumerate() {
            ui.weak(PATH_SEPARATOR);
            if ui.small_button(name).clicked() {
                back_to = Some(depth + 1);
            }
        }
        if let Some(depth) = back_to {
            self.open_folder.truncate(depth);
        }
    }

    /// One tile: the icon of `entry` and its name under it. A click
    /// selects it; returns whether it was double clicked.
    fn show_tile(&mut self, ui: &mut egui::Ui, entry: &Entry) -> bool {
        let (rect, response) = ui.allocate_exact_size(TILE_SIZE, egui::Sense::click());
        let hint =
            if OpenedAsset::of(entry.path.clone()).is_some() || entry.kind == AssetKind::Folder {
                format!("{}\n{}", entry.name, texts::current().open_file_hint)
            } else {
                entry.name.clone()
            };
        let response = response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(hint);
        if response.clicked() {
            self.selected = Some(entry.path.clone());
        }
        let selected = self.selected.as_deref() == Some(entry.path.as_str());
        let visuals = ui.visuals();
        let painter = ui.painter_at(rect);
        if selected {
            painter.rect_filled(rect, TILE_ROUNDING, visuals.selection.bg_fill);
        } else if response.hovered() {
            painter.rect_filled(rect, TILE_ROUNDING, visuals.widgets.hovered.weak_bg_fill);
        }
        let icon = egui::Rect::from_min_size(
            egui::pos2(rect.center().x - ICON_SIDE / 2.0, rect.top() + TILE_PADDING),
            egui::Vec2::splat(ICON_SIDE),
        );
        let thumbnail = (entry.kind == AssetKind::Image)
            .then(|| self.thumbnails.get(ui.ctx(), &entry.path))
            .flatten()
            .map(|texture| (texture.id(), texture.size_vec2()));
        match thumbnail {
            Some((texture, size)) => {
                painter.image(texture, fit(size, icon), FULL_TEXTURE, egui::Color32::WHITE);
            }
            None => paint_icon(&painter, icon, entry.kind, &entry.name),
        }
        let text_color = if selected {
            visuals.selection.stroke.color
        } else {
            visuals.text_color()
        };
        let mut name = egui::text::LayoutJob::simple(
            entry.name.clone(),
            egui::FontId::proportional(NAME_SIZE),
            text_color,
            TILE_SIZE.x - 2.0 * TILE_PADDING,
        );
        name.wrap.max_rows = NAME_ROWS;
        name.wrap.break_anywhere = true;
        name.halign = egui::Align::Center;
        let galley = painter.layout_job(name);
        painter.galley(
            egui::pos2(rect.center().x, icon.bottom() + TILE_PADDING),
            galley,
            text_color,
        );
        response.double_clicked()
    }

    /// Opens `entry`: goes into it if it is a folder, or returns it if the
    /// editor opens files like it.
    fn open(&mut self, entry: &Entry) -> Option<OpenedAsset> {
        if entry.kind == AssetKind::Folder {
            self.open_folder.push(entry.name.clone());
            self.selected = None;
            return None;
        }
        OpenedAsset::of(entry.path.clone())
    }
}

/// A texture's whole surface.
const FULL_TEXTURE: egui::Rect =
    egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));

/// The biggest rectangle of the shape of `size` that fits in `space`,
/// centered in it.
fn fit(size: egui::Vec2, space: egui::Rect) -> egui::Rect {
    let scale = (space.width() / size.x).min(space.height() / size.y);
    if !scale.is_finite() {
        return space;
    }
    egui::Rect::from_center_size(space.center(), size * scale)
}

/// A file or folder of the folder shown.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Entry {
    name: String,
    /// Where it is inside the assets folder: `enemies/boss.png`.
    path: String,
    kind: AssetKind,
}

/// What the folder down the names `open_folder` holds, or `None` if there
/// is no such folder.
fn entries_of(nodes: &[FileNode], open_folder: &[String]) -> Option<Vec<Entry>> {
    let mut shown = nodes;
    for name in open_folder {
        shown = shown.iter().find_map(|node| match node {
            FileNode::Folder {
                name: folder,
                children,
                ..
            } if folder == name => Some(children.as_slice()),
            _ => None,
        })?;
    }
    let prefix: String = open_folder
        .iter()
        .flat_map(|name| [name.as_str(), "/"])
        .collect();
    Some(
        shown
            .iter()
            .map(|node| {
                let name = node.name().to_owned();
                let kind = match node {
                    FileNode::Folder { .. } => AssetKind::Folder,
                    FileNode::File { .. } => AssetKind::of_file(&name),
                };
                Entry {
                    path: format!("{prefix}{name}"),
                    name,
                    kind,
                }
            })
            .collect(),
    )
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

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str) -> FileNode {
        FileNode::File {
            name: name.to_owned(),
        }
    }

    fn contents() -> Vec<FileNode> {
        vec![
            FileNode::Folder {
                name: "enemies".to_owned(),
                path: PathBuf::from("assets/enemies"),
                children: vec![file("boss.png")],
            },
            file("level.scene"),
        ]
    }

    #[test]
    fn the_top_folder_lists_its_files_and_folders() {
        let entries = entries_of(&contents(), &[]).unwrap();

        let kinds: Vec<_> = entries.iter().map(|entry| entry.kind).collect();
        assert_eq!(kinds, [AssetKind::Folder, AssetKind::Scene]);
        assert_eq!(entries[1].path, "level.scene");
    }

    #[test]
    fn inner_folders_give_paths_from_the_top() {
        let entries = entries_of(&contents(), &["enemies".to_owned()]).unwrap();

        assert_eq!(entries[0].path, "enemies/boss.png");
        assert_eq!(entries[0].kind, AssetKind::Image);
    }

    #[test]
    fn folders_that_are_gone_list_nothing() {
        assert_eq!(entries_of(&contents(), &["bosses".to_owned()]), None);
    }

    #[test]
    fn images_fit_their_icon_keeping_their_shape() {
        let space = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(50.0, 50.0));

        let fitted = fit(egui::vec2(200.0, 100.0), space);

        assert_eq!(fitted.size(), egui::vec2(50.0, 25.0));
        assert_eq!(fitted.center(), space.center());
    }
}
