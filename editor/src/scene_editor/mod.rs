//! The scene editor: a `.scene` file of the assets folder, open in the
//! “Scene” tab. The “Scene” panel shows its nodes as a tree, with buttons
//! to add and delete nodes, and the inspector edits the node selected in
//! it. Changes are kept until the scene is saved.

mod components;
mod drawing;
mod fields;
mod picking;
mod properties;
mod tree;

use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui;
use star_engine::prelude::{Frame, SceneOutline};

use self::components::new_node;
pub(crate) use self::drawing::{Marks, SceneGpu, cross_lines, cube_lines, square_lines};
pub(crate) use self::picking::Ray;
use self::properties::{NodeContext, edit_node};
use self::tree::children_of;
use crate::project::ProjectDimension;
use crate::style::ERROR_COLOR;
use crate::texts;

/// A scene open in the editor.
pub(crate) struct SceneDocument {
    /// Where it is inside the assets folder: `levels/one.scene`.
    path: String,
    /// Its file on disk.
    file: PathBuf,
    /// The kind of project it belongs to, which new nodes and parts follow.
    dimension: ProjectDimension,
    /// Its nodes, or why its file could not be read.
    content: Result<SceneOutline, String>,
    /// The position of the node selected in the tree.
    selected: Option<usize>,
    /// Whether it changed since it was opened or saved.
    unsaved: bool,
    /// Why the last save failed, until one works.
    save_error: Option<String>,
}

impl SceneDocument {
    /// Opens the scene at `path` inside the `assets` folder of a project of
    /// `dimension`.
    pub(crate) fn open(assets: &Path, path: String, dimension: ProjectDimension) -> Self {
        let file = assets.join(&path);
        let content = fs::read_to_string(&file)
            .map_err(|error| error.to_string())
            .and_then(|text| SceneOutline::from_text(&text).map_err(|error| error.to_string()));
        Self {
            path,
            file,
            dimension,
            content,
            selected: None,
            unsaved: false,
            save_error: None,
        }
    }

    /// Where it is inside the assets folder.
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    /// The name of its file, for its tab, marked if it has unsaved changes.
    pub(crate) fn title(&self) -> String {
        let name = self.path.rsplit('/').next().unwrap_or(&self.path);
        if self.unsaved {
            texts::unsaved_title(name)
        } else {
            name.to_owned()
        }
    }

    /// Whether it changed since it was opened or saved.
    pub(crate) const fn is_unsaved(&self) -> bool {
        self.unsaved
    }

    /// Writes it to its file if it has unsaved changes. Returns whether it
    /// is saved now; if not, the “Scene” panel shows why.
    pub(crate) fn save(&mut self) -> bool {
        let Ok(outline) = &self.content else {
            return true;
        };
        if !self.unsaved {
            return true;
        }
        let written = outline
            .to_text()
            .map_err(|error| error.to_string())
            .and_then(|text| fs::write(&self.file, text).map_err(|error| error.to_string()));
        match written {
            Ok(()) => {
                self.unsaved = false;
                self.save_error = None;
                true
            }
            Err(error) => {
                self.save_error = Some(texts::current().scene_not_saved(&error));
                false
            }
        }
    }

    /// Adds to `frame` what its nodes show and the mark of the one
    /// selected, with the textures and meshes of `gpu`.
    pub(crate) fn draw(&self, gpu: &mut impl SceneGpu, marks: Marks, frame: &mut Frame) {
        if let Ok(outline) = &self.content {
            drawing::draw_scene(outline, self.selected, gpu, marks, frame);
        }
    }

    /// Selects the node `ray` meets first, or none if it meets nothing, as
    /// when the user clicks the view.
    pub(crate) fn select_at(&mut self, ray: Ray) {
        if let Ok(outline) = &self.content {
            self.selected = picking::node_at(outline, ray);
        }
    }

    /// The tree of its nodes, where the user selects one, with buttons to
    /// add a node and delete the one selected.
    pub(crate) fn show_tree(&mut self, ui: &mut egui::Ui) {
        if let Some(error) = &self.save_error {
            ui.colored_label(ERROR_COLOR, error);
        }
        let outline = match &mut self.content {
            Ok(outline) => outline,
            Err(error) => {
                ui.colored_label(ERROR_COLOR, texts::current().scene_unreadable(error));
                return;
            }
        };
        let mut editor = TreeEditor {
            outline,
            selected: &mut self.selected,
            dimension: self.dimension,
        };
        self.unsaved |= editor.show_buttons(ui);
        ui.separator();
        let tree = SceneTree {
            outline: editor.outline,
            children: children_of(editor.outline.nodes()),
        };
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| tree.show_node(ui, ROOT, editor.selected));
    }

    /// The node selected, to edit, choosing images among `images`. Returns
    /// whether one is selected, so the inspector can show something else
    /// when none is.
    pub(crate) fn edit_selected_node(&mut self, ui: &mut egui::Ui, images: &[String]) -> bool {
        let node = self.content.as_mut().ok().and_then(|outline| {
            self.selected
                .and_then(|selected| outline.node_mut(selected))
        });
        let Some(node) = node else {
            return false;
        };
        let context = NodeContext {
            dimension: self.dimension,
            images,
        };
        self.unsaved |= edit_node(ui, node, &context);
        true
    }
}

/// Every scene has its root first.
const ROOT: usize = 0;

/// What changes the tree: adding and deleting nodes.
struct TreeEditor<'scene> {
    outline: &'scene mut SceneOutline,
    selected: &'scene mut Option<usize>,
    dimension: ProjectDimension,
}

impl TreeEditor<'_> {
    /// “Add node” and “Delete node”. Returns whether the tree changed.
    fn show_buttons(&mut self, ui: &mut egui::Ui) -> bool {
        let texts = texts::current();
        let mut changed = false;
        ui.horizontal(|ui| {
            if ui
                .button(texts.add_node)
                .on_hover_text(texts.add_child_node_hint)
                .clicked()
            {
                changed |= self.add_child(self.selected.unwrap_or(ROOT));
            }
            let deletable = self.selected.is_some_and(|selected| selected != ROOT);
            let delete = ui
                .add_enabled(deletable, egui::Button::new(texts.delete_node))
                .on_disabled_hover_text(texts.root_cannot_be_deleted);
            if delete.clicked() {
                changed |= self.delete_selected();
            }
        });
        changed
    }

    /// Adds a node under `parent` and selects it.
    fn add_child(&mut self, parent: usize) -> bool {
        let name = unique_child_name(self.outline, parent, texts::current().new_node_name);
        match self
            .outline
            .add_node(parent, new_node(name, self.dimension))
        {
            Ok(index) => {
                *self.selected = Some(index);
                true
            }
            Err(error) => {
                log::warn!("{error}");
                false
            }
        }
    }

    /// Deletes the node selected with its descendants, and selects its
    /// parent.
    fn delete_selected(&mut self) -> bool {
        let Some(selected) = *self.selected else {
            return false;
        };
        let parent = self
            .outline
            .nodes()
            .get(selected)
            .and_then(|node| node.parent);
        match self.outline.remove_node(selected) {
            Ok(()) => {
                // Nodes before the one removed keep their place.
                *self.selected = parent;
                true
            }
            Err(error) => {
                log::warn!("{error}");
                false
            }
        }
    }
}

/// `base`, or `base2`, `base3`… : the first that no child of `parent` has.
fn unique_child_name(outline: &SceneOutline, parent: usize, base: &str) -> String {
    let taken = |name: &str| {
        outline
            .nodes()
            .iter()
            .any(|node| node.parent == Some(parent) && node.name == name)
    };
    if !taken(base) {
        return base.to_owned();
    }
    // Among one more name than there are nodes, one is free.
    (2..=outline.nodes().len() + 1)
        .map(|number| format!("{base}{number}"))
        .find(|name| !taken(name))
        .unwrap_or_else(|| base.to_owned())
}

/// The nodes of an outline with the children of each, to draw them as a
/// tree.
struct SceneTree<'scene> {
    outline: &'scene SceneOutline,
    children: Vec<Vec<usize>>,
}

impl SceneTree<'_> {
    /// Draws the node at `index` and, under it, its children; a click
    /// selects it.
    fn show_node(&self, ui: &mut egui::Ui, index: usize, selected: &mut Option<usize>) {
        let Some(node) = self.outline.nodes().get(index) else {
            return;
        };
        let children = self.children.get(index).map_or(&[][..], Vec::as_slice);
        let mut label = |ui: &mut egui::Ui| {
            let response = ui.selectable_label(*selected == Some(index), node.name.as_str());
            let response = match &node.class {
                Some(class) => response.on_hover_text(class.as_str()),
                None => response,
            };
            if response.clicked() {
                *selected = Some(index);
            }
        };
        if children.is_empty() {
            // Leaves line up with the names of the nodes that can open.
            ui.horizontal(|ui| {
                ui.add_space(ui.spacing().indent);
                label(ui);
            });
            return;
        }
        egui::collapsing_header::CollapsingState::load_with_default_open(
            ui.ctx(),
            ui.make_persistent_id(("scene_node", index)),
            true,
        )
        .show_header(ui, label)
        .body(|ui| {
            for &child in children {
                self.show_node(ui, child, selected);
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::TestDirectory;

    const LEVEL: &str = "format = 1\n[[node]]\nname = \"One\"";

    fn open_level(name: &str) -> (TestDirectory, SceneDocument) {
        let assets = TestDirectory::new(name);
        fs::create_dir_all(assets.path().join("levels")).unwrap();
        fs::write(assets.path().join("levels/one.scene"), LEVEL).unwrap();
        let scene = SceneDocument::open(
            assets.path(),
            "levels/one.scene".to_owned(),
            ProjectDimension::TwoD,
        );
        (assets, scene)
    }

    fn tree_editor(scene: &mut SceneDocument) -> TreeEditor<'_> {
        TreeEditor {
            outline: scene.content.as_mut().unwrap(),
            selected: &mut scene.selected,
            dimension: scene.dimension,
        }
    }

    #[test]
    fn scenes_are_read_from_the_assets_folder() {
        let (_assets, scene) = open_level("scene_document_read");

        assert_eq!(scene.title(), "one.scene");
        assert_eq!(scene.content.unwrap().nodes()[0].name, "One");
    }

    #[test]
    fn scenes_that_cannot_be_read_say_why() {
        let assets = TestDirectory::new("scene_document_broken");
        fs::create_dir_all(assets.path()).unwrap();
        fs::write(assets.path().join("bad.scene"), "format = 2").unwrap();

        let broken = SceneDocument::open(
            assets.path(),
            "bad.scene".to_owned(),
            ProjectDimension::TwoD,
        );
        let missing = SceneDocument::open(
            assets.path(),
            "missing.scene".to_owned(),
            ProjectDimension::TwoD,
        );

        assert!(broken.content.unwrap_err().contains("format"));
        assert!(missing.content.is_err());
    }

    #[test]
    fn added_nodes_get_free_names_and_are_selected() {
        let (_assets, mut scene) = open_level("scene_document_add");
        let mut editor = tree_editor(&mut scene);

        assert!(editor.add_child(ROOT));
        assert!(editor.add_child(ROOT));

        let names: Vec<_> = editor
            .outline
            .nodes()
            .iter()
            .map(|node| node.name.clone())
            .collect();
        assert_eq!(names, ["One", "Node", "Node2"]);
        assert_eq!(scene.selected, Some(2));
    }

    #[test]
    fn deleting_a_node_selects_its_parent() {
        let (_assets, mut scene) = open_level("scene_document_delete");
        let mut editor = tree_editor(&mut scene);
        editor.add_child(ROOT);

        assert!(editor.delete_selected());

        assert_eq!(editor.outline.nodes().len(), 1);
        assert_eq!(scene.selected, Some(ROOT));
    }

    #[test]
    fn saving_writes_the_changes_and_clears_the_mark() {
        let (assets, mut scene) = open_level("scene_document_save");
        tree_editor(&mut scene).add_child(ROOT);
        scene.unsaved = true;

        assert!(scene.save());

        assert!(!scene.is_unsaved());
        assert_eq!(scene.title(), "one.scene");
        let text = fs::read_to_string(assets.path().join("levels/one.scene")).unwrap();
        let saved = SceneOutline::from_text(&text).unwrap();
        assert_eq!(saved.nodes()[1].name, "Node");
    }
}
