//! The scene editor: a `.scene` file of the assets folder, open in the
//! “Scene” tab. The “Scene” panel shows its nodes as a tree, and the
//! inspector the parts of the node selected in it.

mod properties;
mod tree;

use std::fs;
use std::path::Path;

use eframe::egui;
use star_engine::prelude::SceneOutline;

use self::properties::show_node_properties;
use self::tree::children_of;
use crate::style::ERROR_COLOR;
use crate::texts;

/// A scene open in the editor.
pub(crate) struct SceneDocument {
    /// Where it is inside the assets folder: `levels/one.scene`.
    path: String,
    /// Its nodes, or why its file could not be read.
    content: Result<SceneOutline, String>,
    /// The position of the node selected in the tree.
    selected: Option<usize>,
}

impl SceneDocument {
    /// Opens the scene at `path` inside the `assets` folder.
    pub(crate) fn open(assets: &Path, path: String) -> Self {
        let content = fs::read_to_string(assets.join(&path))
            .map_err(|error| error.to_string())
            .and_then(|text| SceneOutline::from_text(&text).map_err(|error| error.to_string()));
        Self {
            path,
            content,
            selected: None,
        }
    }

    /// Where it is inside the assets folder.
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    /// The name of its file, for its tab.
    pub(crate) fn title(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// The tree of its nodes, where the user selects one.
    pub(crate) fn show_tree(&mut self, ui: &mut egui::Ui) {
        let outline = match &self.content {
            Ok(outline) => outline,
            Err(error) => {
                ui.colored_label(ERROR_COLOR, texts::current().scene_unreadable(error));
                return;
            }
        };
        let tree = SceneTree {
            outline,
            children: children_of(outline.nodes()),
        };
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| tree.show_node(ui, ROOT, &mut self.selected));
    }

    /// The parts of the node selected. Returns whether one is selected, so
    /// the inspector can show something else when none is.
    pub(crate) fn show_selected_node(&self, ui: &mut egui::Ui) -> bool {
        let node = self.content.as_ref().ok().and_then(|outline| {
            self.selected
                .and_then(|selected| outline.nodes().get(selected))
        });
        if let Some(node) = node {
            show_node_properties(ui, node);
        }
        node.is_some()
    }
}

/// Every scene has its root first.
const ROOT: usize = 0;

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

    #[test]
    fn scenes_are_read_from_the_assets_folder() {
        let assets = TestDirectory::new("scene_document_read");
        fs::create_dir_all(assets.path().join("levels")).unwrap();
        fs::write(
            assets.path().join("levels/one.scene"),
            "format = 1\n[[node]]\nname = \"One\"",
        )
        .unwrap();

        let scene = SceneDocument::open(assets.path(), "levels/one.scene".to_owned());

        assert_eq!(scene.title(), "one.scene");
        assert_eq!(scene.content.unwrap().nodes()[0].name, "One");
    }

    #[test]
    fn scenes_that_cannot_be_read_say_why() {
        let assets = TestDirectory::new("scene_document_broken");
        fs::create_dir_all(assets.path()).unwrap();
        fs::write(assets.path().join("bad.scene"), "format = 2").unwrap();

        let broken = SceneDocument::open(assets.path(), "bad.scene".to_owned());
        let missing = SceneDocument::open(assets.path(), "missing.scene".to_owned());

        assert!(broken.content.unwrap_err().contains("format"));
        assert!(missing.content.is_err());
    }
}
