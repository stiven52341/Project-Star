//! The tabs of the central area: the scene, always there (with the scene
//! file open in it, if any), and the blueprints, scripts and animations
//! (`.frames` files) the user opened.

use std::path::Path;

use eframe::egui;

use crate::blueprint_editor::BlueprintTab;
use crate::frames_editor::FramesTab;
use crate::project::ProjectDimension;
use crate::project_check::OpenTexts;
use crate::scene_editor::SceneDocument;
use crate::script_editor::ScriptTab;
use crate::texts;

/// The open scene, blueprints and scripts, and which tab is shown.
pub(crate) struct Documents {
    /// The scene of the “Scene” tab, once one is opened.
    scene: Option<SceneDocument>,
    tabs: Vec<Tab>,
    /// The tab shown, or `None` for the scene.
    active: Option<usize>,
}

/// A file open in a tab of its own.
enum Tab {
    Blueprint(BlueprintTab),
    Script(ScriptTab),
    Frames(FramesTab),
}

impl Tab {
    fn path(&self) -> &str {
        match self {
            Self::Blueprint(tab) => tab.path(),
            Self::Script(tab) => tab.path(),
            Self::Frames(tab) => tab.path(),
        }
    }

    fn title(&self) -> String {
        match self {
            Self::Blueprint(tab) => tab.title(),
            Self::Script(tab) => tab.title(),
            Self::Frames(tab) => tab.title(),
        }
    }

    const fn is_unsaved(&self) -> bool {
        match self {
            Self::Blueprint(tab) => tab.is_unsaved(),
            Self::Script(tab) => tab.is_unsaved(),
            Self::Frames(tab) => tab.is_unsaved(),
        }
    }

    fn save(&mut self) -> bool {
        match self {
            Self::Blueprint(tab) => tab.save(),
            Self::Script(tab) => tab.save(),
            Self::Frames(tab) => tab.save(),
        }
    }
}

/// What the central area shows this frame.
pub(crate) enum ActiveDocument<'documents> {
    Scene,
    Blueprint(&'documents mut BlueprintTab),
    Script(&'documents mut ScriptTab),
    Frames(&'documents mut FramesTab),
}

impl Documents {
    pub(crate) const fn new() -> Self {
        Self {
            scene: None,
            tabs: Vec::new(),
            active: None,
        }
    }

    /// Shows the blueprint at `path` of the `assets` folder, opening it if
    /// it is not open yet.
    pub(crate) fn open_blueprint(&mut self, assets: &Path, path: String) {
        self.show_or_open(path, |path| {
            Tab::Blueprint(BlueprintTab::open(assets, path))
        });
    }

    /// Shows the script at `path` of the `assets` folder, opening it if it
    /// is not open yet.
    pub(crate) fn open_script(&mut self, assets: &Path, path: String) {
        self.show_or_open(path, |path| Tab::Script(ScriptTab::open(assets, path)));
    }

    /// Shows the `.frames` file at `path` of the `assets` folder, opening
    /// it if it is not open yet.
    pub(crate) fn open_frames(&mut self, assets: &Path, path: String) {
        self.show_or_open(path, |path| Tab::Frames(FramesTab::open(assets, path)));
    }

    /// The texts of the open scripts, by their path inside the assets
    /// folder, saved or not.
    pub(crate) fn open_texts(&self) -> OpenTexts {
        self.tabs
            .iter()
            .filter_map(|tab| match tab {
                Tab::Script(script) => script
                    .text()
                    .map(|text| (script.path().to_owned(), text.to_owned())),
                Tab::Blueprint(_) | Tab::Frames(_) => None,
            })
            .collect()
    }

    /// Shows the tab of `path`, opening it with `open` if there is none.
    fn show_or_open(&mut self, path: String, open: impl FnOnce(String) -> Tab) {
        let index = self.position_of(&path).unwrap_or_else(|| {
            self.tabs.push(open(path));
            self.tabs.len() - 1
        });
        self.active = Some(index);
    }

    /// Shows the scene at `path` of the `assets` folder of a project of
    /// `dimension` in the “Scene” tab, in place of the one open there,
    /// saved or not.
    pub(crate) fn open_scene(&mut self, assets: &Path, path: String, dimension: ProjectDimension) {
        if !self.is_scene_open(&path) {
            self.scene = Some(SceneDocument::open(assets, path, dimension));
        }
        self.active = None;
    }

    /// Whether the scene at `path` is the one open.
    pub(crate) fn is_scene_open(&self, path: &str) -> bool {
        self.scene
            .as_ref()
            .is_some_and(|scene| scene.path() == path)
    }

    /// Whether the open scene has unsaved changes.
    pub(crate) fn scene_is_unsaved(&self) -> bool {
        self.scene.as_ref().is_some_and(SceneDocument::is_unsaved)
    }

    /// Saves the open scene. Returns whether it is saved now.
    pub(crate) fn save_scene(&mut self) -> bool {
        self.scene.as_mut().is_none_or(SceneDocument::save)
    }

    /// The scene open in the “Scene” tab.
    pub(crate) fn scene_mut(&mut self) -> Option<&mut SceneDocument> {
        self.scene.as_mut()
    }

    /// Closes, saved or not, the scene and tabs at `path` or inside the
    /// folder at `path`: their files are gone or moved.
    pub(crate) fn forget(&mut self, path: &str) {
        while let Some(index) = self.tabs.iter().position(|tab| is_within(tab.path(), path)) {
            self.close(index);
        }
        if self
            .scene
            .as_ref()
            .is_some_and(|scene| is_within(scene.path(), path))
        {
            self.scene = None;
        }
    }

    /// The paths of the scene and tabs with unsaved changes.
    pub(crate) fn unsaved_paths(&self) -> Vec<String> {
        let scene = self
            .scene
            .as_ref()
            .filter(|scene| scene.is_unsaved())
            .map(|scene| scene.path().to_owned());
        let tabs = self
            .tabs
            .iter()
            .filter(|tab| tab.is_unsaved())
            .map(|tab| tab.path().to_owned());
        scene.into_iter().chain(tabs).collect()
    }

    /// Reads again from the `assets` folder of a project of `dimension`
    /// the scene and tabs at `paths`, whose files changed on disk, unless
    /// they have changes of their own.
    pub(crate) fn reload(&mut self, assets: &Path, paths: &[String], dimension: ProjectDimension) {
        let changed = |path: &str| paths.iter().any(|changed| changed == path);
        if let Some(scene) = &mut self.scene
            && changed(scene.path())
            && !scene.is_unsaved()
        {
            *scene = SceneDocument::open(assets, scene.path().to_owned(), dimension);
        }
        for tab in &mut self.tabs {
            if !changed(tab.path()) || tab.is_unsaved() {
                continue;
            }
            let path = tab.path().to_owned();
            *tab = match tab {
                Tab::Blueprint(_) => Tab::Blueprint(BlueprintTab::open(assets, path)),
                Tab::Script(_) => Tab::Script(ScriptTab::open(assets, path)),
                Tab::Frames(_) => Tab::Frames(FramesTab::open(assets, path)),
            };
        }
    }

    fn position_of(&self, path: &str) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.path() == path)
    }

    /// Closes the tab at `index`, saved or not; the tab shown stays shown,
    /// or the scene is shown if it was that one.
    pub(crate) fn close(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        self.active = match self.active {
            Some(active) if active == index => None,
            Some(active) if active > index => Some(active - 1),
            other => other,
        };
    }

    /// Whether the tab at `index` has unsaved changes.
    pub(crate) fn is_unsaved(&self, index: usize) -> bool {
        self.tabs.get(index).is_some_and(Tab::is_unsaved)
    }

    /// Whether the scene or any tab has unsaved changes.
    pub(crate) fn any_unsaved(&self) -> bool {
        self.scene_is_unsaved() || self.tabs.iter().any(Tab::is_unsaved)
    }

    /// Whether the tab shown has unsaved changes.
    pub(crate) fn active_is_unsaved(&self) -> bool {
        match self.active {
            Some(index) => self.is_unsaved(index),
            None => self.scene_is_unsaved(),
        }
    }

    /// Saves the tab at `index`. Returns whether it is saved now.
    pub(crate) fn save(&mut self, index: usize) -> bool {
        self.tabs.get_mut(index).is_none_or(Tab::save)
    }

    /// Saves the tab shown.
    pub(crate) fn save_active(&mut self) {
        match self.active {
            Some(index) => self.save(index),
            None => self.save_scene(),
        };
    }

    /// Saves the scene and every tab. Returns whether all of them are saved
    /// now; those that are not say why.
    pub(crate) fn save_all(&mut self) -> bool {
        // Every file is saved, even after one fails.
        let mut all_saved = self.save_scene();
        for tab in &mut self.tabs {
            all_saved &= tab.save();
        }
        all_saved
    }

    /// The tab shown.
    pub(crate) fn active(&mut self) -> ActiveDocument<'_> {
        match self.active.and_then(|index| self.tabs.get_mut(index)) {
            Some(Tab::Blueprint(tab)) => ActiveDocument::Blueprint(tab),
            Some(Tab::Script(tab)) => ActiveDocument::Script(tab),
            Some(Tab::Frames(tab)) => ActiveDocument::Frames(tab),
            None => ActiveDocument::Scene,
        }
    }

    /// The row of tabs, each file with a button to close it. Returns the
    /// tab the user asked to close.
    pub(crate) fn show_tabs(&mut self, ui: &mut egui::Ui) -> Option<usize> {
        let mut close = None;
        ui.horizontal(|ui| {
            let scene_title = self.scene.as_ref().map_or_else(
                || texts::current().scene_tab.to_owned(),
                SceneDocument::title,
            );
            let scene_tab = ui.selectable_value(&mut self.active, None, scene_title);
            if let Some(scene) = &self.scene {
                scene_tab.on_hover_text(scene.path());
            }
            for (index, tab) in self.tabs.iter().enumerate() {
                ui.separator();
                ui.selectable_value(&mut self.active, Some(index), tab.title())
                    .on_hover_text(tab.path());
                if ui
                    .small_button("×")
                    .on_hover_text(texts::current().close_tab)
                    .clicked()
                {
                    close = Some(index);
                }
            }
        });
        close
    }
}

/// Whether `file` is `path` or inside the folder at `path`.
fn is_within(file: &str, path: &str) -> bool {
    file.strip_prefix(path)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::TestDirectory;

    fn active_path(documents: &mut Documents) -> Option<String> {
        match documents.active() {
            ActiveDocument::Scene => None,
            ActiveDocument::Blueprint(tab) => Some(tab.path().to_owned()),
            ActiveDocument::Script(tab) => Some(tab.path().to_owned()),
            ActiveDocument::Frames(tab) => Some(tab.path().to_owned()),
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
    fn opened_animations_are_shown_in_a_tab_of_their_own() {
        let (assets, mut documents) = documents_with(&["a.blueprint"]);

        documents.open_frames(assets.path(), "hero.frames".to_owned());
        documents.open_frames(assets.path(), "hero.frames".to_owned());

        assert_eq!(active_path(&mut documents).as_deref(), Some("hero.frames"));
        assert_eq!(documents.tabs.len(), 2);
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

        assert_eq!(documents.tabs.len(), 2);
        assert_eq!(active_path(&mut documents).as_deref(), Some("a.blueprint"));
    }

    #[test]
    fn opening_a_scene_shows_the_scene_tab_with_it() {
        let (assets, mut documents) = documents_with(&["a.blueprint"]);

        documents.open_scene(
            assets.path(),
            "level.scene".to_owned(),
            ProjectDimension::TwoD,
        );

        assert_eq!(active_path(&mut documents), None);
        assert!(documents.is_scene_open("level.scene"));
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

    #[test]
    fn scripts_open_in_tabs_beside_blueprints() {
        let (assets, mut documents) = documents_with(&["a.blueprint"]);

        documents.open_script(assets.path(), "a.star".to_owned());
        documents.open_blueprint(assets.path(), "a.blueprint".to_owned());
        documents.open_script(assets.path(), "a.star".to_owned());

        assert_eq!(documents.tabs.len(), 2);
        assert!(matches!(documents.active(), ActiveDocument::Script(_)));
    }

    #[test]
    fn files_are_within_themselves_and_their_folders() {
        assert!(is_within("enemies/boss.blueprint", "enemies"));
        assert!(is_within(
            "enemies/boss.blueprint",
            "enemies/boss.blueprint"
        ));
        assert!(!is_within("enemies2/boss.blueprint", "enemies"));
    }
}
