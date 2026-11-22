//! The tabs of the central area: the scene, always there, and the
//! blueprints the user opened.

use std::path::Path;

use eframe::egui;

use crate::blueprint_editor::BlueprintTab;
use crate::texts;

/// The open blueprints, and which tab is shown.
pub(crate) struct Documents {
    blueprints: Vec<BlueprintTab>,
    /// The blueprint shown, or `None` for the scene.
    active: Option<usize>,
}

/// What the central area shows this frame.
pub(crate) enum ActiveDocument<'documents> {
    Scene,
    Blueprint(&'documents mut BlueprintTab),
}

impl Documents {
    pub(crate) const fn new() -> Self {
        Self {
            blueprints: Vec::new(),
            active: None,
        }
    }

    /// Shows the blueprint at `path` of the `assets` folder, opening it if
    /// it is not open yet.
    pub(crate) fn open_blueprint(&mut self, assets: &Path, path: String) {
        let index = self.position_of(&path).unwrap_or_else(|| {
            self.blueprints.push(BlueprintTab::open(assets, path));
            self.blueprints.len() - 1
        });
        self.active = Some(index);
    }

    fn position_of(&self, path: &str) -> Option<usize> {
        self.blueprints.iter().position(|tab| tab.path() == path)
    }

    /// Closes the blueprint at `index`, saved or not; the tab shown stays
    /// shown, or the scene is shown if it was that one.
    pub(crate) fn close(&mut self, index: usize) {
        if index >= self.blueprints.len() {
            return;
        }
        self.blueprints.remove(index);
        self.active = match self.active {
            Some(active) if active == index => None,
            Some(active) if active > index => Some(active - 1),
            other => other,
        };
    }

    /// Whether the blueprint at `index` has unsaved changes.
    pub(crate) fn is_unsaved(&self, index: usize) -> bool {
        self.blueprints
            .get(index)
            .is_some_and(BlueprintTab::is_unsaved)
    }

    /// Whether any blueprint has unsaved changes.
    pub(crate) fn any_unsaved(&self) -> bool {
        self.blueprints.iter().any(BlueprintTab::is_unsaved)
    }

    /// Whether the tab shown is a blueprint with unsaved changes.
    pub(crate) fn active_is_unsaved(&self) -> bool {
        self.active.is_some_and(|index| self.is_unsaved(index))
    }

    /// Saves the blueprint at `index`. Returns whether it is saved now.
    pub(crate) fn save(&mut self, index: usize) -> bool {
        self.blueprints
            .get_mut(index)
            .is_none_or(BlueprintTab::save)
    }

    /// Saves the blueprint shown, if one is.
    pub(crate) fn save_active(&mut self) {
        if let Some(index) = self.active {
            self.save(index);
        }
    }

    /// Saves every blueprint. Returns whether all of them are saved now;
    /// the tabs of those that are not say why.
    pub(crate) fn save_all(&mut self) -> bool {
        // Every tab is saved, even after one fails.
        let mut all_saved = true;
        for tab in &mut self.blueprints {
            all_saved &= tab.save();
        }
        all_saved
    }

    /// The tab shown.
    pub(crate) fn active(&mut self) -> ActiveDocument<'_> {
        match self.active.and_then(|index| self.blueprints.get_mut(index)) {
            Some(tab) => ActiveDocument::Blueprint(tab),
            None => ActiveDocument::Scene,
        }
    }

    /// The row of tabs, each blueprint with a button to close it. Returns
    /// the blueprint the user asked to close.
    pub(crate) fn show_tabs(&mut self, ui: &mut egui::Ui) -> Option<usize> {
        let mut close = None;
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.active, None, texts::SCENE_TAB);
            for (index, tab) in self.blueprints.iter().enumerate() {
                ui.separator();
                ui.selectable_value(&mut self.active, Some(index), tab.title())
                    .on_hover_text(tab.path());
                if ui
                    .small_button("×")
                    .on_hover_text(texts::CLOSE_TAB)
                    .clicked()
                {
                    close = Some(index);
                }
            }
        });
        close
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::TestDirectory;

    fn active_path(documents: &mut Documents) -> Option<String> {
        match documents.active() {
            ActiveDocument::Scene => None,
            ActiveDocument::Blueprint(tab) => Some(tab.path().to_owned()),
        }
    }

    fn documents_with(paths: &[&str]) -> (TestDirectory, Documents) {
        let assets = TestDirectory::new(&format!("documents_{}", paths.join("_")));
        let mut documents = Documents::new();
        for path in paths {
            documents.open_blueprint(assets.path(), (*path).to_owned());
        }
        (assets, documents)
    }

    #[test]
    fn blueprints_open_saved() {
        let (_assets, documents) = documents_with(&["a.blueprint"]);

        assert!(!documents.any_unsaved());
        assert!(!documents.active_is_unsaved());
    }

    #[test]
    fn the_scene_is_shown_at_first() {
        let mut documents = Documents::new();

        assert_eq!(active_path(&mut documents), None);
    }

    #[test]
    fn opened_blueprints_are_shown() {
        let (_assets, mut documents) = documents_with(&["a.blueprint", "b.blueprint"]);

        assert_eq!(active_path(&mut documents).as_deref(), Some("b.blueprint"));
    }

    #[test]
    fn opening_an_open_blueprint_shows_its_tab() {
        let (assets, mut documents) = documents_with(&["a.blueprint", "b.blueprint"]);

        documents.open_blueprint(assets.path(), "a.blueprint".to_owned());

        assert_eq!(documents.blueprints.len(), 2);
        assert_eq!(active_path(&mut documents).as_deref(), Some("a.blueprint"));
    }

    #[test]
    fn closing_the_tab_shown_shows_the_scene() {
        let (_assets, mut documents) = documents_with(&["a.blueprint", "b.blueprint"]);

        documents.close(1);

        assert_eq!(active_path(&mut documents), None);
    }

    #[test]
    fn closing_a_tab_before_the_one_shown_keeps_it_shown() {
        let (_assets, mut documents) =
            documents_with(&["a.blueprint", "b.blueprint", "c.blueprint"]);
        documents.active = Some(2);

        documents.close(0);

        assert_eq!(active_path(&mut documents).as_deref(), Some("c.blueprint"));
    }
}
