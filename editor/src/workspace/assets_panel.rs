//! Browser of the files inside the project's `assets` folder: one folder
//! at a time, its contents as a grid of icons (images as small copies of
//! themselves), with the way back to the top above them. Right clicks
//! open menus to create files and folders, and to open, rename, delete or
//! copy the path of the one clicked.

use std::io;
use std::path::{Path, PathBuf};

use eframe::egui;
use star_engine::prelude::AssetPath;
use star_engine::star_assets::ImageFormat;

use super::asset_actions::{AssetActions, Choice, Outcome, Target};
use super::asset_files::NewAsset;
use super::asset_icons::{AssetKind, paint_icon};
use super::thumbnails::Thumbnails;
use crate::file_tree::{FileNode, file_paths, scan_directory};
use crate::project::{
    ASSETS_DIRECTORY, BLUEPRINT_EXTENSION, ProjectDimension, SCENE_EXTENSION, SCRIPT_EXTENSION,
};
use crate::scene_editor::{ClassDrop, ImageDrop};
use crate::style::ERROR_COLOR;
use crate::texts;

/// A file of the assets folder the user opened, as a path inside it.
pub(crate) enum OpenedAsset {
    Blueprint(String),
    Scene(String),
    Script(String),
}

impl OpenedAsset {
    /// What `name` opens as, if the editor opens files like it.
    fn of(path: String) -> Option<Self> {
        if path.ends_with(BLUEPRINT_EXTENSION) {
            Some(Self::Blueprint(path))
        } else if path.ends_with(SCENE_EXTENSION) {
            Some(Self::Scene(path))
        } else if path.ends_with(SCRIPT_EXTENSION) {
            Some(Self::Script(path))
        } else {
            None
        }
    }
}

/// What the user did in the panel that the rest of the editor must know.
pub(crate) enum AssetEvent {
    /// Opened a file.
    Opened(OpenedAsset),
    /// Renamed or deleted the file or folder at this path: what was open
    /// from it is gone.
    Gone(String),
}

/// What the user chose in a menu or with a key.
enum MenuChoice {
    Open(Entry),
    Act(Choice),
    Refresh,
    /// Show the file or folder at this path, inside the assets folder, in
    /// the system's file manager.
    ShowInFileExplorer(String),
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
/// The side of the copy of an image that follows the pointer while it is
/// dragged, and how far from the pointer it is.
const DRAGGED_SIDE: f32 = 40.0;
const DRAGGED_OFFSET: f32 = 12.0;
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
    /// Naming files and asking before deleting them.
    actions: AssetActions,
}

impl AssetsPanel {
    /// Lists the assets folder of the project in `project_directory`, of
    /// `dimension`.
    pub(crate) fn new(project_directory: &Path, dimension: ProjectDimension) -> Self {
        let directory = project_directory.join(ASSETS_DIRECTORY);
        let contents = scan_directory(&directory);
        Self {
            images: image_paths(&contents),
            thumbnails: Thumbnails::new(&directory),
            directory,
            contents,
            open_folder: Vec::new(),
            selected: None,
            actions: AssetActions::new(dimension),
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

    /// Shows the folder open. Returns what the user opened (double click
    /// or menu), renamed or deleted.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) -> Option<AssetEvent> {
        ui.horizontal(|ui| {
            self.show_path(ui);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button(texts::current().refresh).clicked() {
                    self.refresh();
                }
            });
        });
        let folder = self.open_folder.join("/");
        let mut event = self
            .actions
            .show(ui, &self.directory, &folder)
            .and_then(|outcome| self.apply(outcome));
        ui.separator();
        let entries = match &self.contents {
            Ok(nodes) => entries_of(nodes, &self.open_folder),
            Err(error) => {
                ui.colored_label(ERROR_COLOR, texts::current().assets_unreadable(error));
                return event;
            }
        };
        let Some(entries) = entries else {
            // The folder shown is gone: back to the top.
            self.open_folder.clear();
            return event;
        };
        let mut choice = None;
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| {
                let background = ui.interact(
                    ui.clip_rect(),
                    ui.make_persistent_id("assets_background"),
                    egui::Sense::click(),
                );
                if background.clicked() {
                    self.selected = None;
                }
                background.context_menu(|ui| {
                    create_menu(ui, &mut choice);
                    if ui.button(texts::current().refresh).clicked() {
                        choice = Some(MenuChoice::Refresh);
                    }
                    if ui.button(texts::current().show_in_file_explorer).clicked() {
                        choice = Some(MenuChoice::ShowInFileExplorer(self.open_folder.join("/")));
                    }
                });
                if entries.is_empty() {
                    ui.weak(texts::current().empty_assets);
                }
                ui.horizontal_wrapped(|ui| {
                    for entry in &entries {
                        if let Some(chosen) = self.show_tile(ui, entry) {
                            choice = Some(chosen);
                        }
                    }
                });
            });
        if ui.ui_contains_pointer() {
            choice = choice.or_else(|| self.choice_from_keys(ui, &entries));
        }
        if let Some(choice) = choice {
            event = event.or_else(|| self.carry_out(choice));
        }
        self.show_dragged_image(ui);
        show_dragged_class(ui);
        event
    }

    /// `F2` renames the file selected and `Delete` deletes it, unless a
    /// text field is being typed in.
    fn choice_from_keys(&self, ui: &egui::Ui, entries: &[Entry]) -> Option<MenuChoice> {
        if ui.ctx().egui_wants_keyboard_input() {
            return None;
        }
        let entry = entries
            .iter()
            .find(|entry| self.selected.as_deref() == Some(entry.path.as_str()))?;
        let (rename, delete) = ui.input(|input| {
            (
                input.key_pressed(egui::Key::F2),
                input.key_pressed(egui::Key::Delete),
            )
        });
        if rename {
            Some(MenuChoice::Act(Choice::Rename(entry.target())))
        } else if delete {
            Some(MenuChoice::Act(Choice::Delete(entry.target())))
        } else {
            None
        }
    }

    fn carry_out(&mut self, choice: MenuChoice) -> Option<AssetEvent> {
        match choice {
            MenuChoice::Open(entry) => self.open(&entry).map(AssetEvent::Opened),
            MenuChoice::Act(choice) => {
                self.actions.choose(choice);
                None
            }
            MenuChoice::Refresh => {
                self.refresh();
                None
            }
            MenuChoice::ShowInFileExplorer(path) => {
                show_in_file_explorer(self.directory.join(path));
                None
            }
        }
    }

    /// Lists the folder again after `outcome` and selects what it made.
    fn apply(&mut self, outcome: Outcome) -> Option<AssetEvent> {
        self.refresh();
        match outcome {
            Outcome::Created(path) => {
                self.selected = Some(path);
                None
            }
            Outcome::Renamed { from, to } => {
                self.selected = Some(to);
                Some(AssetEvent::Gone(from))
            }
            Outcome::Deleted(path) => {
                self.selected = None;
                Some(AssetEvent::Gone(path))
            }
        }
    }

    /// While an image of the panel is dragged, a small copy of it and its
    /// name follow the pointer.
    fn show_dragged_image(&mut self, ui: &egui::Ui) {
        let ctx = ui.ctx();
        let Some(image) = egui::DragAndDrop::payload::<ImageDrop>(ctx) else {
            return;
        };
        let Some(cursor) = ctx.pointer_latest_pos() else {
            return;
        };
        ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Tooltip,
            egui::Id::unique("dragged_asset"),
        ));
        let space = egui::Rect::from_min_size(
            cursor + egui::Vec2::splat(DRAGGED_OFFSET),
            egui::Vec2::splat(DRAGGED_SIDE),
        );
        if let Some(texture) = self.thumbnails.get(ctx, &image.path) {
            let place = fit(texture.size_vec2(), space);
            painter.image(texture.id(), place, FULL_TEXTURE, egui::Color32::WHITE);
        } else {
            let name = image.path.rsplit('/').next().unwrap_or(&image.path);
            paint_icon(&painter, space, AssetKind::Image, name);
        }
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
    /// selects it; returns what a double click or its menu chose.
    fn show_tile(&mut self, ui: &mut egui::Ui, entry: &Entry) -> Option<MenuChoice> {
        let (rect, response) = ui.allocate_exact_size(TILE_SIZE, egui::Sense::click_and_drag());
        let hint =
            if OpenedAsset::of(entry.path.clone()).is_some() || entry.kind == AssetKind::Folder {
                format!("{}\n{}", entry.name, texts::current().open_file_hint)
            } else {
                entry.name.clone()
            };
        let response = response
            .on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(hint);
        if response.clicked() || response.secondary_clicked() {
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
        if entry.kind == AssetKind::Image {
            response.dnd_set_drag_payload(ImageDrop {
                path: entry.path.clone(),
                aspect: thumbnail.map(|(_, size)| size.x / size.y),
            });
        }
        let carries_classes = matches!(entry.kind, AssetKind::Script | AssetKind::Blueprint);
        if carries_classes && response.drag_started() {
            // Read once, when the drag starts, not every frame it lasts.
            if let Some(classes) = ClassDrop::read(&self.directory, entry.path.clone()) {
                response.dnd_set_drag_payload(classes);
            }
        }
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
        let mut choice = response
            .double_clicked()
            .then(|| MenuChoice::Open(entry.clone()));
        response.context_menu(|ui| tile_menu(ui, entry, &mut choice));
        choice
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

impl Entry {
    /// What a menu acting on it needs.
    fn target(&self) -> Target {
        Target {
            path: self.path.clone(),
            name: self.name.clone(),
            is_folder: self.kind == AssetKind::Folder,
        }
    }
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

/// The menu of a tile: open it, rename it, delete it, copy its path, show
/// it in the file manager, or create something next to it.
fn tile_menu(ui: &mut egui::Ui, entry: &Entry, choice: &mut Option<MenuChoice>) {
    let texts = texts::current();
    let opens = entry.kind == AssetKind::Folder || OpenedAsset::of(entry.path.clone()).is_some();
    if opens && ui.button(texts.open).clicked() {
        *choice = Some(MenuChoice::Open(entry.clone()));
    }
    if ui.button(texts.rename).clicked() {
        *choice = Some(MenuChoice::Act(Choice::Rename(entry.target())));
    }
    if ui.button(texts.delete).clicked() {
        *choice = Some(MenuChoice::Act(Choice::Delete(entry.target())));
    }
    if ui.button(texts.copy_path).clicked() {
        ui.ctx().copy_text(entry.path.clone());
    }
    if ui.button(texts.show_in_file_explorer).clicked() {
        *choice = Some(MenuChoice::ShowInFileExplorer(entry.path.clone()));
    }
    ui.separator();
    create_menu(ui, choice);
}

/// Opens the system's file manager at `path` on a thread of its own, so
/// the editor never waits for it; a failure is only logged.
fn show_in_file_explorer(path: PathBuf) {
    std::thread::spawn(move || {
        if let Err(error) = star_engine::star_platform::show_in_file_explorer(&path) {
            log::warn!("{error}");
        }
    });
}

/// “Create”, with a kind of file or folder to create in the folder shown.
fn create_menu(ui: &mut egui::Ui, choice: &mut Option<MenuChoice>) {
    let texts = texts::current();
    ui.menu_button(texts.create, |ui| {
        for kind in NewAsset::ALL {
            if ui.button(texts.new_asset_name(kind)).clicked() {
                *choice = Some(MenuChoice::Act(Choice::Create(kind)));
            }
        }
    });
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

/// While a script or a blueprint is dragged, its icon and name follow
/// the pointer, with a warning if it has no class a node can be built as.
fn show_dragged_class(ui: &egui::Ui) {
    let ctx = ui.ctx();
    let Some(script) = egui::DragAndDrop::payload::<ClassDrop>(ctx) else {
        return;
    };
    let Some(cursor) = ctx.pointer_latest_pos() else {
        return;
    };
    ctx.set_cursor_icon(egui::CursorIcon::Grabbing);
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Tooltip,
        egui::Id::unique("dragged_class"),
    ));
    let space = egui::Rect::from_min_size(
        cursor + egui::Vec2::splat(DRAGGED_OFFSET),
        egui::Vec2::splat(DRAGGED_SIDE),
    );
    let name = script.path.rsplit('/').next().unwrap_or(&script.path);
    paint_icon(&painter, space, AssetKind::of_file(name), name);
    if script.classes.is_empty() {
        painter.text(
            space.left_bottom() + egui::vec2(0.0, DRAGGED_OFFSET),
            egui::Align2::LEFT_TOP,
            texts::current().no_node_class,
            egui::TextStyle::Body.resolve(ui.style()),
            ERROR_COLOR,
        );
    }
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
