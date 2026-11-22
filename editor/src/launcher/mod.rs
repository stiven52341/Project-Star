//! Start-up screen: open a recent project or create a new one.

mod new_project_form;
mod recent_list;

use std::path::Path;

use eframe::egui;
use star_engine::prelude::ENGINE_NAME;

use self::new_project_form::NewProjectForm;
use self::recent_list::{RecentAction, RecentEntry, show_recent_list};
use crate::folder_dialog::pick_folder;
use crate::language_menu::show_language_menu;
use crate::project::{Project, default_projects_location, open_project};
use crate::recent_projects::RecentProjectsStore;
use crate::style::{ERROR_COLOR, SECTION_SPACING, TITLE_SIZE};
use crate::texts::{self, Language};

/// What the user did in the launcher this frame.
pub(crate) enum LauncherAction {
    /// Opened (or created) a project.
    Open(Project),
    /// Chose to see the editor in another language.
    ChangeLanguage(Language),
}

/// Project manager shown when the editor starts or a project is closed.
pub(crate) struct Launcher {
    recent: RecentProjectsStore,
    entries: Vec<RecentEntry>,
    form: NewProjectForm,
    open_error: Option<String>,
}

impl Launcher {
    pub(crate) fn new() -> Self {
        let recent = RecentProjectsStore::load();
        let entries = load_entries(&recent);
        Self {
            recent,
            entries,
            form: NewProjectForm::new(),
            open_error: None,
        }
    }

    /// Draws the launcher. Returns what the user did this frame.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui) -> Option<LauncherAction> {
        let language = show_header(ui);
        let mut opened = None;
        // Columns stretch their widgets by default; `vertical` keeps natural widths.
        ui.columns(2, |columns| {
            let recent_opened = columns[0].vertical(|ui| self.show_recent_column(ui)).inner;
            let created = columns[1].vertical(|ui| self.form.show(ui)).inner;
            opened = recent_opened.or_else(|| created.and_then(|directory| self.open(&directory)));
        });
        language
            .map(LauncherAction::ChangeLanguage)
            .or_else(|| opened.map(LauncherAction::Open))
    }

    fn show_recent_column(&mut self, ui: &mut egui::Ui) -> Option<Project> {
        ui.heading(texts::current().recent_projects);
        ui.add_space(SECTION_SPACING);
        let mut opened = match show_recent_list(ui, &self.entries) {
            Some(RecentAction::Open(directory)) => self.open(&directory),
            Some(RecentAction::Remove(directory)) => {
                self.forget(&directory);
                None
            }
            None => None,
        };
        ui.add_space(SECTION_SPACING);
        if ui.button(texts::current().open_other_project).clicked()
            && let Some(directory) = pick_folder(
                texts::current().choose_project_folder,
                &default_projects_location(),
            )
        {
            opened = self.open(&directory);
        }
        if let Some(error) = &self.open_error {
            ui.colored_label(ERROR_COLOR, error);
        }
        opened
    }

    fn open(&mut self, directory: &Path) -> Option<Project> {
        match open_project(directory) {
            Ok(project) => {
                log::info!("Opening project {}", project.directory().display());
                self.recent.add(project.directory());
                Some(project)
            }
            Err(error) => {
                log::error!("{error}");
                self.open_error = Some(texts::current().open_error(&error));
                self.entries = load_entries(&self.recent);
                None
            }
        }
    }

    fn forget(&mut self, directory: &Path) {
        self.recent.remove(directory);
        self.entries = load_entries(&self.recent);
    }
}

fn load_entries(recent: &RecentProjectsStore) -> Vec<RecentEntry> {
    recent
        .paths()
        .iter()
        .map(|path| RecentEntry::load(path))
        .collect()
}

/// The engine's name, with the language menu on its right. Returns the
/// language chosen.
fn show_header(ui: &mut egui::Ui) -> Option<Language> {
    let chosen = ui
        .horizontal(|ui| {
            ui.label(egui::RichText::new(ENGINE_NAME).size(TITLE_SIZE).strong());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                show_language_menu(ui)
            })
            .inner
        })
        .inner;
    ui.weak(texts::current().project_manager);
    ui.add_space(SECTION_SPACING);
    ui.separator();
    ui.add_space(SECTION_SPACING);
    chosen
}
