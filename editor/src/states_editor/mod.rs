//! The states editor: a `.states` file of the assets folder (an animator's
//! state machine, decision 81), open in a tab of the central area. The
//! middle shows its graph of states and transitions; the inspector edits
//! what is selected and the values scripts set. The names of the
//! animations come from an `.anim` file chosen above the graph.

mod edits;
mod graph;
mod inspector;

use std::fs;
use std::path::{Path, PathBuf};

use eframe::egui;
use star_engine::star_scene::{
    ANIMATIONS_EXTENSION, AnimationStates, STATES_EXTENSION, TrackAnimations,
};

use self::graph::{GraphState, Selected, show_graph};
use crate::style::ERROR_COLOR;
use crate::texts;

/// A `.states` file open in the editor.
pub(crate) struct StatesTab {
    /// Where it is inside the assets folder: `characters/hero.states`.
    path: String,
    /// The assets folder, where the `.anim` files are.
    assets: PathBuf,
    /// The machine, or why the file could not be read.
    content: Result<AnimationStates, String>,
    selected: Selected,
    graph: GraphState,
    /// The `.anim` file whose animations the states choose from, and their
    /// names.
    animations: Option<(String, Vec<String>)>,
    /// Whether it changed since it was opened or saved.
    unsaved: bool,
    /// Why the last save failed, until one works.
    save_error: Option<String>,
}

impl StatesTab {
    /// Opens the `.states` file at `path` inside the `assets` folder.
    pub(crate) fn open(assets: &Path, path: String) -> Self {
        let content = fs::read_to_string(assets.join(&path))
            .map_err(|error| error.to_string())
            .and_then(|text| AnimationStates::from_text(&text).map_err(|error| error.to_string()));
        Self {
            path,
            assets: assets.to_path_buf(),
            content,
            selected: Selected::Nothing,
            graph: GraphState::default(),
            animations: None,
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
    /// is saved now; if not, the tab shows why (a transition to a state
    /// that is gone, for one).
    pub(crate) fn save(&mut self) -> bool {
        let Ok(machine) = &self.content else {
            return true;
        };
        if !self.unsaved {
            return true;
        }
        let written = machine
            .to_text()
            .map_err(|error| error.to_string())
            .and_then(|text| {
                fs::write(self.assets.join(&self.path), text).map_err(|error| error.to_string())
            });
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

    /// The middle of the tab: the `.anim` file to take names from, what is
    /// wrong with the machine, and its graph. `keyframes` are the `.anim`
    /// files of the assets folder.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui, keyframes: &[String]) {
        let machine = match &mut self.content {
            Ok(machine) => machine,
            Err(error) => {
                ui.colored_label(ERROR_COLOR, texts::current().states_unreadable(error));
                return;
            }
        };
        if self.animations.is_none()
            && let Some(file) = matching_animations(&self.path, keyframes)
        {
            self.animations = Some(read_names(&self.assets, file));
        }
        if let Some(chosen) = animations_row(ui, self.animations.as_ref(), keyframes) {
            self.animations = Some(read_names(&self.assets, &chosen));
        }
        if let Some(error) = &self.save_error {
            ui.colored_label(ERROR_COLOR, texts::current().states_not_saved(error));
        }
        if let Err(error) = machine.validate() {
            ui.colored_label(ERROR_COLOR, error.to_string());
        }
        let hint = if self.graph.is_linking() {
            texts::current().linking_hint()
        } else {
            texts::current().states_graph_hint()
        };
        ui.weak(hint);
        ui.separator();
        let names = self.animations.as_ref().map(|(_, names)| names.as_slice());
        let first = names.and_then(<[String]>::first).map_or("", String::as_str);
        self.unsaved |= show_graph(ui, machine, &mut self.selected, &mut self.graph, first);
    }

    /// The inspector: what is selected on the graph, and the values.
    pub(crate) fn show_inspector(&mut self, ui: &mut egui::Ui) {
        let Ok(machine) = &mut self.content else {
            return;
        };
        let names = self
            .animations
            .as_ref()
            .map_or(&[][..], |(_, names)| names.as_slice());
        self.unsaved |= inspector::show_inspector(ui, machine, &mut self.selected, names);
    }
}

/// The row that chooses the `.anim` file the states take animations from.
/// Returns the file chosen, if one was.
fn animations_row(
    ui: &mut egui::Ui,
    current: Option<&(String, Vec<String>)>,
    keyframes: &[String],
) -> Option<String> {
    let texts = texts::current();
    let mut chosen = None;
    ui.horizontal(|ui| {
        ui.weak(texts.keyframes())
            .on_hover_text(texts.states_animations_hint());
        let shown = current.map_or(texts.no_states(), |(path, _)| path.as_str());
        egui::ComboBox::from_id_salt("states_animations")
            .selected_text(shown)
            .show_ui(ui, |ui| {
                for path in keyframes {
                    let selected = shown == path;
                    if ui.selectable_label(selected, path).clicked() && !selected {
                        chosen = Some(path.clone());
                    }
                }
            });
    });
    chosen
}

/// The `.anim` file a `.states` file at `path` most likely chooses from:
/// the one of the same name and folder, or else the first one.
fn matching_animations<'files>(path: &str, keyframes: &'files [String]) -> Option<&'files String> {
    let stem = path.strip_suffix(STATES_EXTENSION).unwrap_or(path);
    let twin = format!("{stem}{ANIMATIONS_EXTENSION}");
    keyframes
        .iter()
        .find(|file| **file == twin)
        .or_else(|| keyframes.first())
}

/// The file at `path` of the assets folder and the names of its
/// animations; none if it cannot be read (the reason is logged).
fn read_names(assets: &Path, path: &str) -> (String, Vec<String>) {
    let names = fs::read_to_string(assets.join(path))
        .map_err(|error| error.to_string())
        .and_then(|text| TrackAnimations::from_text(&text).map_err(|error| error.to_string()))
        .inspect_err(|error| log::warn!("\"{path}\": {error}"))
        .map(|file| {
            file.animations
                .into_iter()
                .map(|animation| animation.name)
                .collect()
        })
        .unwrap_or_default();
    (path.to_owned(), names)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_animations_of_the_same_name_are_chosen_first() {
        let keyframes = ["door.anim".to_owned(), "chars/hero.anim".to_owned()];

        assert_eq!(
            matching_animations("chars/hero.states", &keyframes).map(String::as_str),
            Some("chars/hero.anim")
        );
        assert_eq!(
            matching_animations("guard.states", &keyframes).map(String::as_str),
            Some("door.anim")
        );
        assert_eq!(matching_animations("guard.states", &[]), None);
    }

    #[test]
    fn tabs_save_what_changed_and_say_why_a_file_is_unreadable() {
        let assets = std::env::temp_dir().join("states_editor_tab");
        fs::create_dir_all(&assets).unwrap();
        let text =
            "format = 1\nstart = \"idle\"\n[[state]]\nname = \"idle\"\nanimation = \"idle\"\n";
        fs::write(assets.join("hero.states"), text).unwrap();
        fs::write(assets.join("broken.states"), "format = 1").unwrap();

        let mut tab = StatesTab::open(&assets, "hero.states".to_owned());
        if let Ok(machine) = &mut tab.content {
            edits::add_state(machine, [0.0, 0.0], "run");
        }
        tab.unsaved = true;

        assert_eq!(tab.title(), texts::unsaved_title("hero.states"));
        assert!(tab.save());
        let saved =
            AnimationStates::from_text(&fs::read_to_string(assets.join("hero.states")).unwrap());
        assert_eq!(saved.unwrap().states.len(), 2);
        assert!(
            StatesTab::open(&assets, "broken.states".to_owned())
                .content
                .is_err()
        );
    }
}
