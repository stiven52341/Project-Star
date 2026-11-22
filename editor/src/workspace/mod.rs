//! Main editor window: menu, scene hierarchy, viewport, inspector and assets.

mod assets_panel;
mod documents;
mod editor_camera;
mod grid;
mod inspector_panel;
mod menu_bar;
mod problems_panel;
mod unsaved_dialog;
mod viewport;

use eframe::{egui, egui_wgpu};
use star_engine::prelude::{ENGINE_NAME, ENGINE_VERSION};

use self::assets_panel::AssetsPanel;
use self::documents::{ActiveDocument, Documents};
use self::inspector_panel::show_inspector;
use self::menu_bar::{MenuAction, SAVE_SHORTCUT, show_menu_bar};
use self::problems_panel::ProblemsPanel;
use self::unsaved_dialog::{UnsavedChoice, show_unsaved_dialog};
use self::viewport::SceneViewport;
use crate::project::{ASSETS_DIRECTORY, Project};
use crate::project_check::Problem;
use crate::style::ERROR_COLOR;
use crate::texts;

const SIDE_PANEL_WIDTH: f32 = 260.0;
const SIDE_PANEL_MIN_WIDTH: f32 = 180.0;
const BOTTOM_PANEL_HEIGHT: f32 = 200.0;
const BOTTOM_PANEL_MIN_HEIGHT: f32 = 100.0;

/// What the editor window asks the application to do.
pub(crate) enum EditorAction {
    CloseProject,
    /// Close the window: the user chose to lose unsaved changes.
    Quit,
}

/// Something that closes blueprints, waiting for the user to say what to do
/// with their unsaved changes.
#[derive(Clone, Copy)]
enum Leaving {
    CloseTab(usize),
    CloseProject,
    Quit,
}

/// What the panel below the view shows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BottomTab {
    Assets,
    Problems,
}

/// Editor for one open project.
pub(crate) struct EditorWindow {
    project: Project,
    assets: AssetsPanel,
    problems: ProblemsPanel,
    bottom_tab: BottomTab,
    documents: Documents,
    /// What is waiting for the answer about unsaved changes.
    leaving: Option<Leaving>,
    /// Whether the user already chose to quit without saving.
    quit_confirmed: bool,
    /// The engine view, or why it could not be created.
    viewport: Result<SceneViewport, String>,
    about_open: bool,
}

impl EditorWindow {
    /// Opens `project`, rendering its scene on the UI's GPU (`render_state`).
    pub(crate) fn new(project: Project, render_state: Option<&egui_wgpu::RenderState>) -> Self {
        let assets = AssetsPanel::new(project.directory());
        let dimension = project.settings().dimension();
        let viewport = render_state
            .ok_or_else(|| texts::NO_GPU_FOR_VIEWPORT.to_owned())
            .and_then(|state| {
                SceneViewport::new(state, dimension).map_err(|error| error.to_string())
            });
        if let Err(error) = &viewport {
            log::error!("Scene viewport unavailable: {error}");
        }
        Self {
            project,
            assets,
            problems: ProblemsPanel::new(),
            bottom_tab: BottomTab::Assets,
            documents: Documents::new(),
            leaving: None,
            quit_confirmed: false,
            viewport,
            about_open: false,
        }
    }

    /// Title for the native window while this project is open.
    pub(crate) fn title(&self) -> String {
        format!("{} — {ENGINE_NAME}", self.project.settings().name())
    }

    pub(crate) fn show(&mut self, ui: &mut egui::Ui) -> Option<EditorAction> {
        let mut action = None;
        if self.problems.update() {
            self.bottom_tab = BottomTab::Problems;
        }
        self.handle_quit_request(ui.ctx());
        if ui.input_mut(|input| input.consume_shortcut(&SAVE_SHORTCUT)) {
            self.documents.save_active();
        }
        let compiling = self.problems.is_compiling();
        let can_save = self.documents.active_is_unsaved();
        egui::Panel::top("menu_bar").show(ui, |ui| {
            match show_menu_bar(ui, can_save, compiling) {
                Some(MenuAction::Save) => self.documents.save_active(),
                Some(MenuAction::CloseProject) => action = self.leave(Leaving::CloseProject),
                Some(MenuAction::Compile) => {
                    // «Compilar» checks the files, so it checks what is
                    // shown.
                    self.documents.save_all();
                    self.problems.compile(self.project.directory(), ui.ctx());
                }
                Some(MenuAction::ShowAbout) => self.about_open = true,
                None => {}
            }
        });
        egui::Panel::bottom("status_bar").show(ui, |ui| self.show_status_bar(ui));
        egui::Panel::left("hierarchy")
            .resizable(true)
            .default_size(SIDE_PANEL_WIDTH)
            .min_size(SIDE_PANEL_MIN_WIDTH)
            .show(ui, show_hierarchy);
        egui::Panel::right("inspector")
            .resizable(true)
            .default_size(SIDE_PANEL_WIDTH)
            .min_size(SIDE_PANEL_MIN_WIDTH)
            .show(ui, |ui| match self.documents.active() {
                ActiveDocument::Scene => show_inspector(ui, &self.project),
                ActiveDocument::Blueprint(tab) => {
                    ui.strong(texts::INSPECTOR);
                    ui.separator();
                    tab.show_inspector(ui, self.problems.problems());
                }
            });
        egui::Panel::bottom("bottom_panel")
            .resizable(true)
            .default_size(BOTTOM_PANEL_HEIGHT)
            .min_size(BOTTOM_PANEL_MIN_HEIGHT)
            .show(ui, |ui| self.show_bottom_panel(ui));
        egui::CentralPanel::default().show(ui, |ui| self.show_central_area(ui));
        if action.is_none() {
            action = self.ask_about_unsaved_changes(ui.ctx());
        }
        self.show_about(ui.ctx());
        action
    }

    /// Closing the window waits for an answer if blueprints are unsaved.
    fn handle_quit_request(&mut self, context: &egui::Context) {
        let requested = context.input(|input| input.viewport().close_requested());
        if requested && !self.quit_confirmed && self.documents.any_unsaved() {
            context.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.leaving = Some(Leaving::Quit);
        }
    }

    /// Does what `leaving` does, or asks first if it would lose unsaved
    /// changes. Returns what the application must do.
    fn leave(&mut self, leaving: Leaving) -> Option<EditorAction> {
        let unsaved = match leaving {
            Leaving::CloseTab(index) => self.documents.is_unsaved(index),
            Leaving::CloseProject | Leaving::Quit => self.documents.any_unsaved(),
        };
        if unsaved {
            self.leaving = Some(leaving);
            None
        } else {
            self.finish_leaving(leaving)
        }
    }

    fn finish_leaving(&mut self, leaving: Leaving) -> Option<EditorAction> {
        match leaving {
            Leaving::CloseTab(index) => {
                self.documents.close(index);
                None
            }
            Leaving::CloseProject => Some(EditorAction::CloseProject),
            Leaving::Quit => {
                self.quit_confirmed = true;
                Some(EditorAction::Quit)
            }
        }
    }

    /// The question about unsaved changes, while something waits for it.
    fn ask_about_unsaved_changes(&mut self, context: &egui::Context) -> Option<EditorAction> {
        let leaving = self.leaving?;
        let question = match leaving {
            Leaving::CloseTab(_) => texts::UNSAVED_BLUEPRINT_QUESTION,
            Leaving::CloseProject | Leaving::Quit => texts::UNSAVED_BLUEPRINTS_QUESTION,
        };
        let choice = show_unsaved_dialog(context, question)?;
        self.leaving = None;
        match choice {
            UnsavedChoice::Cancel => None,
            UnsavedChoice::Discard => self.finish_leaving(leaving),
            UnsavedChoice::Save => {
                let saved = match leaving {
                    Leaving::CloseTab(index) => self.documents.save(index),
                    Leaving::CloseProject | Leaving::Quit => self.documents.save_all(),
                };
                // If a save fails, its tab says why and nothing is closed.
                if saved {
                    self.finish_leaving(leaving)
                } else {
                    None
                }
            }
        }
    }

    /// The tabs of the scene and the open blueprints, and the one shown.
    fn show_central_area(&mut self, ui: &mut egui::Ui) {
        if let Some(index) = self.documents.show_tabs(ui) {
            self.leave(Leaving::CloseTab(index));
        }
        match self.documents.active() {
            ActiveDocument::Scene => match &mut self.viewport {
                Ok(viewport) => viewport.show(ui),
                Err(error) => {
                    ui.colored_label(ERROR_COLOR, texts::viewport_error(error));
                }
            },
            ActiveDocument::Blueprint(tab) => tab.show(ui, self.problems.problems()),
        }
    }

    fn show_bottom_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.bottom_tab, BottomTab::Assets, texts::ASSETS);
            ui.selectable_value(
                &mut self.bottom_tab,
                BottomTab::Problems,
                self.problems.title(),
            );
        });
        ui.separator();
        match self.bottom_tab {
            BottomTab::Assets => {
                if let Some(path) = self.assets.show(ui) {
                    self.documents.open_blueprint(self.assets.directory(), path);
                }
            }
            BottomTab::Problems => {
                if let Some(problem) = self.problems.show(ui) {
                    self.show_problem_in_blueprint(&problem);
                }
            }
        }
    }

    /// Opens the blueprint of `problem` where the problem is.
    fn show_problem_in_blueprint(&mut self, problem: &Problem) {
        let (Some(origin), Some(path)) = (
            &problem.origin,
            problem
                .file
                .strip_prefix(ASSETS_DIRECTORY)
                .and_then(|path| path.strip_prefix('/')),
        ) else {
            return;
        };
        self.documents
            .open_blueprint(self.assets.directory(), path.to_owned());
        if let ActiveDocument::Blueprint(tab) = self.documents.active() {
            tab.reveal(origin);
        }
    }

    fn show_status_bar(&self, ui: &mut egui::Ui) {
        let settings = self.project.settings();
        ui.horizontal(|ui| {
            ui.label(settings.name());
            ui.weak(settings.dimension().label());
            ui.weak(self.project.directory().display().to_string());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.weak(format!("{ENGINE_NAME} {ENGINE_VERSION}"));
                ui.separator();
                self.problems.show_summary(ui);
            });
        });
    }

    fn show_about(&mut self, context: &egui::Context) {
        egui::Window::new(format!("{} {ENGINE_NAME}", texts::ABOUT))
            .open(&mut self.about_open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(context, |ui| {
                ui.strong(format!("{ENGINE_NAME} {ENGINE_VERSION}"));
                ui.label(texts::ABOUT_DESCRIPTION);
                ui.separator();
                ui.label(texts::LICENSE_NOTICE);
                ui.label(texts::GAME_EXCEPTION_NOTICE);
                ui.weak(texts::NO_WARRANTY);
                ui.hyperlink_to(texts::SOURCE_CODE, REPOSITORY_URL);
            });
    }
}

/// Where the source code is published, as the GPL asks programs to tell users.
const REPOSITORY_URL: &str = env!("CARGO_PKG_REPOSITORY");

fn show_hierarchy(ui: &mut egui::Ui) {
    ui.strong(texts::HIERARCHY);
    ui.separator();
    ui.weak(texts::EMPTY_SCENE);
}
