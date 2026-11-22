//! The blueprint editor: a blueprint of the assets folder, open in a tab of
//! the central area, with its graphs drawn as nodes and wires.

mod canvas;
mod layout;
mod view;

use std::fs;
use std::path::Path;

use eframe::egui;
use star_blueprint::Blueprint;

use self::view::CanvasView;
use crate::style::ERROR_COLOR;
use crate::texts;

/// A blueprint open in the editor.
pub(crate) struct BlueprintTab {
    /// Where it is inside the assets folder: `enemies/rock.blueprint`.
    path: String,
    /// The blueprint, or why its file could not be read.
    content: Result<Blueprint, String>,
    /// Which of its graphs is shown.
    graph: usize,
    view: CanvasView,
}

impl BlueprintTab {
    /// Opens the blueprint at `path` inside the `assets` folder.
    pub(crate) fn open(assets: &Path, path: String) -> Self {
        let content = fs::read_to_string(assets.join(&path))
            .map_err(|error| error.to_string())
            .and_then(|text| Blueprint::from_text(&text).map_err(|error| error.to_string()));
        Self {
            path,
            content,
            graph: 0,
            view: CanvasView::default(),
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

    pub(crate) fn show(&mut self, ui: &mut egui::Ui) {
        let blueprint = match &self.content {
            Ok(blueprint) => blueprint,
            Err(error) => {
                ui.colored_label(ERROR_COLOR, texts::blueprint_unreadable(error));
                return;
            }
        };
        ui.horizontal(|ui| {
            ui.strong(texts::class_heading(blueprint));
            if !blueprint.graphs.is_empty() {
                ui.separator();
                show_graph_selector(ui, blueprint, &mut self.graph, &mut self.view);
                ui.separator();
                ui.weak(texts::CANVAS_CONTROLS);
            }
        });
        match blueprint.graphs.get(self.graph) {
            Some(graph) => canvas::show_graph(ui, graph, &mut self.view),
            None => {
                ui.weak(texts::NO_GRAPHS);
            }
        }
    }
}

/// The list of the blueprint's graphs; choosing another one shows it from
/// the start.
fn show_graph_selector(
    ui: &mut egui::Ui,
    blueprint: &Blueprint,
    selected: &mut usize,
    view: &mut CanvasView,
) {
    let current = blueprint
        .graphs
        .get(*selected)
        .map_or("", |graph| graph.name.as_str());
    egui::ComboBox::from_id_salt("blueprint_graph")
        .selected_text(current)
        .show_ui(ui, |ui| {
            for (index, graph) in blueprint.graphs.iter().enumerate() {
                if ui
                    .selectable_value(selected, index, graph.name.as_str())
                    .changed()
                {
                    *view = CanvasView::default();
                }
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::TestDirectory;

    const ROCK: &str = "format = 1\nclass = \"Rock\"\n\n\
                        [[graph]]\nname = \"start\"\n\n\
                        [[graph.node]]\nid = 1\nkind = \"entry\"\n";

    #[test]
    fn blueprints_open_from_the_assets_folder() {
        let assets = TestDirectory::new("blueprint_tab_open");
        fs::create_dir_all(assets.path().join("enemies")).expect("folder");
        fs::write(assets.path().join("enemies").join("rock.blueprint"), ROCK).expect("file");

        let tab = BlueprintTab::open(assets.path(), "enemies/rock.blueprint".to_owned());

        assert_eq!(tab.title(), "rock.blueprint");
        assert_eq!(tab.path(), "enemies/rock.blueprint");
        assert!(matches!(&tab.content, Ok(blueprint) if blueprint.class == "Rock"));
    }

    #[test]
    fn unreadable_blueprints_say_why() {
        let assets = TestDirectory::new("blueprint_tab_broken");
        fs::create_dir_all(assets.path()).expect("folder");
        fs::write(assets.path().join("rock.blueprint"), "class = \"Rock\"\n").expect("file");

        let tab = BlueprintTab::open(assets.path(), "rock.blueprint".to_owned());

        assert!(matches!(&tab.content, Err(reason) if reason.contains("format")));
    }

    #[test]
    fn missing_files_cannot_be_opened() {
        let assets = TestDirectory::new("blueprint_tab_missing");

        let tab = BlueprintTab::open(assets.path(), "ghost.blueprint".to_owned());

        assert!(tab.content.is_err());
    }
}
