//! The blueprint editor: a blueprint of the assets folder, open in a tab of
//! the central area, with its graphs drawn as nodes and wires to edit; the
//! inspector shows the selected node, or the class and the graph shown.
//! The mistakes the last “Compile” found in it are shown on its nodes.

mod canvas;
mod class_properties;
mod creation;
mod edit;
mod layout;
mod literal_field;
mod menus;
mod names;
mod painting;
mod properties;
mod selection;
mod view;

use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui;
use star_blueprint::{Blueprint, Origin};

use self::canvas::{CanvasState, NodeMistakes};
pub(crate) use self::creation::{NewBlueprintError, create_blueprint};
use self::names::BlueprintNames;
use crate::project::ASSETS_DIRECTORY;
use crate::project_check::Problem;
use crate::style::ERROR_COLOR;
use crate::texts;

/// Names `Nuevo gráfico` gives: `method1`, `method2`…
const NEW_GRAPH_PREFIX: &str = "method";

/// A blueprint open in the editor.
pub(crate) struct BlueprintTab {
    /// Where it is inside the assets folder: `enemies/rock.blueprint`.
    path: String,
    /// Its file on disk.
    file: PathBuf,
    /// The blueprint, or why its file could not be read.
    content: Result<Blueprint, String>,
    /// Which of its graphs is shown.
    graph: usize,
    canvas: CanvasState,
    /// Whether it changed since it was opened or saved.
    unsaved: bool,
    /// Why the last save failed, until one works.
    save_error: Option<String>,
}

impl BlueprintTab {
    /// Opens the blueprint at `path` inside the `assets` folder.
    pub(crate) fn open(assets: &Path, path: String) -> Self {
        let file = assets.join(&path);
        let content = fs::read_to_string(&file)
            .map_err(|error| error.to_string())
            .and_then(|text| Blueprint::from_text(&text).map_err(|error| error.to_string()));
        Self {
            path,
            file,
            content,
            graph: 0,
            canvas: CanvasState::default(),
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
    /// is saved now; if not, the tab shows why.
    pub(crate) fn save(&mut self) -> bool {
        let Ok(blueprint) = &self.content else {
            return true;
        };
        if !self.unsaved {
            return true;
        }
        let written = blueprint
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
                self.save_error = Some(error);
                false
            }
        }
    }

    /// Shows the blueprint's part `origin`: the node selected in the middle
    /// of its graph, or the inspector of its class and that graph.
    pub(crate) fn reveal(&mut self, origin: &Origin) {
        let Ok(blueprint) = &self.content else {
            return;
        };
        let graph_named = |name: &str| blueprint.graphs.iter().position(|graph| graph.name == name);
        match origin {
            Origin::Node { graph, node } => {
                if let Some(index) = graph_named(graph) {
                    self.show_graph(index);
                    self.canvas.focus_on(*node);
                }
            }
            Origin::Graph(graph) => {
                if let Some(index) = graph_named(graph) {
                    self.show_graph(index);
                }
            }
            Origin::Class | Origin::Variable(_) => self.canvas.selection.clear(),
        }
    }

    fn show_graph(&mut self, index: usize) {
        if index != self.graph {
            self.graph = index;
            self.canvas.show_other_graph();
        }
        self.canvas.selection.clear();
    }

    /// How “Compile” names its file.
    fn problem_file(&self) -> String {
        format!("{ASSETS_DIRECTORY}/{}", self.path)
    }

    /// The `problems` of the last “Compile” in this blueprint.
    fn own_problems<'problems>(
        &self,
        problems: &'problems [Problem],
    ) -> impl Iterator<Item = &'problems Problem> {
        let file = self.problem_file();
        problems.iter().filter(move |problem| problem.file == file)
    }

    /// The tab: its class, the graph shown and the mistakes of the last
    /// “Compile” (`problems`) that are not on a node of it.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui, problems: &[Problem]) {
        let own_problems: Vec<&Problem> = self.own_problems(problems).collect();
        let blueprint = match &mut self.content {
            Ok(blueprint) => blueprint,
            Err(error) => {
                ui.colored_label(ERROR_COLOR, texts::current().blueprint_unreadable(error));
                return;
            }
        };
        if let Some(error) = &self.save_error {
            ui.colored_label(ERROR_COLOR, texts::current().blueprint_not_saved(error));
        }
        ui.horizontal(|ui| {
            ui.strong(texts::class_heading(blueprint));
            ui.separator();
            if !blueprint.graphs.is_empty() {
                show_graph_selector(ui, blueprint, &mut self.graph, &mut self.canvas);
            }
            if show_graphs_menu(ui, blueprint, &mut self.graph, &mut self.canvas) {
                self.unsaved = true;
            }
            ui.separator();
            ui.weak(texts::current().canvas_controls);
        });
        let current_graph = blueprint
            .graphs
            .get(self.graph)
            .map(|graph| graph.name.clone());
        let mut mistakes = NodeMistakes::new();
        for problem in own_problems {
            match &problem.origin {
                Some(Origin::Node { graph, node }) if Some(graph) == current_graph.as_ref() => {
                    mistakes.entry(*node).or_default().push(&problem.message);
                }
                Some(Origin::Node { .. }) => {}
                _ => {
                    ui.colored_label(ERROR_COLOR, &problem.message);
                }
            }
        }
        let names = BlueprintNames::of(blueprint);
        match blueprint.graphs.get_mut(self.graph) {
            Some(graph) => {
                self.unsaved |= canvas::show_graph(ui, graph, &mut self.canvas, &names, &mistakes);
            }
            None => {
                ui.weak(texts::current().no_graphs);
            }
        }
    }

    /// The inspector while this tab is shown: the selected node's
    /// properties and mistakes, or the class's and the graph's.
    pub(crate) fn show_inspector(&mut self, ui: &mut egui::Ui, problems: &[Problem]) {
        let own_problems: Vec<&Problem> = self.own_problems(problems).collect();
        let Ok(blueprint) = &mut self.content else {
            return;
        };
        let names = BlueprintNames::of(blueprint);
        let current_graph = blueprint
            .graphs
            .get(self.graph)
            .map(|graph| graph.name.clone());
        let selected = self.canvas.selection.len();
        match (self.canvas.selection.single(), current_graph) {
            (Some(id), Some(graph_name)) => {
                for problem in &own_problems {
                    if problem.origin.as_ref()
                        == Some(&Origin::Node {
                            graph: graph_name.clone(),
                            node: id,
                        })
                    {
                        ui.colored_label(ERROR_COLOR, &problem.message);
                    }
                }
                if let Some(graph) = blueprint.graphs.get_mut(self.graph) {
                    self.unsaved |= properties::show_node_properties(ui, graph, id, &names);
                }
            }
            _ if selected > 1 => {
                ui.weak(texts::current().nodes_selected(selected));
            }
            _ => {
                self.unsaved |= class_properties::show_class_properties(ui, blueprint, self.graph);
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
    canvas: &mut CanvasState,
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
                    canvas.show_other_graph();
                }
            }
        });
}

/// “New graph” and “Delete this graph”. Returns whether the
/// blueprint changed.
fn show_graphs_menu(
    ui: &mut egui::Ui,
    blueprint: &mut Blueprint,
    selected: &mut usize,
    canvas: &mut CanvasState,
) -> bool {
    let mut changed = false;
    ui.menu_button(texts::current().graphs_menu, |ui| {
        if ui.button(texts::current().new_graph).clicked() {
            let taken: Vec<&str> = blueprint
                .graphs
                .iter()
                .map(|graph| graph.name.as_str())
                .collect();
            let name = edit::unused_name(NEW_GRAPH_PREFIX, &taken);
            blueprint.graphs.push(edit::new_graph(name));
            *selected = blueprint.graphs.len() - 1;
            canvas.show_other_graph();
            changed = true;
        }
        let can_delete = *selected < blueprint.graphs.len();
        if ui
            .add_enabled(can_delete, egui::Button::new(texts::current().delete_graph))
            .clicked()
        {
            blueprint.graphs.remove(*selected);
            *selected = selected.saturating_sub(1);
            canvas.show_other_graph();
            changed = true;
        }
    });
    changed
}

#[cfg(test)]
mod tests {
    use eframe::egui::Pos2;
    use star_blueprint::NodeKind;

    use super::*;
    use crate::project::TestDirectory;

    const ROCK: &str = "format = 1\nclass = \"Rock\"\n\n\
                        [[graph]]\nname = \"start\"\n\n\
                        [[graph.node]]\nid = 1\nkind = \"entry\"\n";

    fn assets_with_rock(test_name: &str) -> TestDirectory {
        let assets = TestDirectory::new(test_name);
        fs::create_dir_all(assets.path().join("enemies")).expect("folder");
        fs::write(assets.path().join("enemies").join("rock.blueprint"), ROCK).expect("file");
        assets
    }

    #[test]
    fn blueprints_open_from_the_assets_folder() {
        let assets = assets_with_rock("blueprint_tab_open");

        let tab = BlueprintTab::open(assets.path(), "enemies/rock.blueprint".to_owned());

        assert_eq!(tab.title(), "rock.blueprint");
        assert_eq!(tab.path(), "enemies/rock.blueprint");
        assert!(matches!(&tab.content, Ok(blueprint) if blueprint.class == "Rock"));
        assert!(!tab.is_unsaved());
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

    #[test]
    fn saved_changes_are_in_the_file() {
        let assets = assets_with_rock("blueprint_tab_save");
        let mut tab = BlueprintTab::open(assets.path(), "enemies/rock.blueprint".to_owned());
        if let Ok(blueprint) = &mut tab.content {
            edit::add_node(&mut blueprint.graphs[0], NodeKind::Return, Pos2::ZERO);
        }
        tab.unsaved = true;
        assert_eq!(tab.title(), texts::unsaved_title("rock.blueprint"));

        assert!(tab.save());

        let reopened = BlueprintTab::open(assets.path(), "enemies/rock.blueprint".to_owned());
        assert!(!tab.is_unsaved());
        assert_eq!(reopened.content, tab.content);
    }

    #[test]
    fn failed_saves_keep_the_changes_and_say_why() {
        let assets = assets_with_rock("blueprint_tab_save_failed");
        let mut tab = BlueprintTab::open(assets.path(), "enemies/rock.blueprint".to_owned());
        tab.unsaved = true;
        fs::remove_dir_all(assets.path().join("enemies")).expect("removed");

        assert!(!tab.save());

        assert!(tab.is_unsaved());
        assert!(tab.save_error.is_some());
    }
}
