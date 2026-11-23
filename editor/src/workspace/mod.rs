//! Main editor window: menu, scene hierarchy, viewport, inspector and assets.

mod asset_actions;
mod asset_files;
mod asset_icons;
mod assets_panel;
mod console_panel;
mod documents;
mod editor_camera;
mod grid;
mod inspector_panel;
mod menu_bar;
mod output_panel;
mod problems_panel;
mod thumbnails;
mod unsaved_dialog;
mod viewport;
mod viewport_assets;
mod viewport_tools;

use eframe::{egui, egui_wgpu};
use star_engine::prelude::{ENGINE_NAME, ENGINE_VERSION};

pub(crate) use self::asset_files::{AssetFileError, NewAsset};
use self::assets_panel::{AssetEvent, AssetsPanel, OpenedAsset};
use self::console_panel::ConsolePanel;
use self::documents::{ActiveDocument, Documents};
use self::inspector_panel::show_inspector;
use self::menu_bar::{MenuAction, MenuState, SAVE_SHORTCUT, show_menu_bar};
use self::output_panel::OutputPanel;
use self::problems_panel::{CheckEnd, ProblemsPanel};
use self::unsaved_dialog::{UnsavedChoice, show_unsaved_dialog};
use self::viewport::SceneViewport;
use crate::editor_log::LogHistory;
use crate::game_run::PlayRequest;
use crate::project::{ASSETS_DIRECTORY, Project};
use crate::project_check::Problem;
use crate::style::ERROR_COLOR;
use crate::texts::{self, Language};

const SIDE_PANEL_WIDTH: f32 = 260.0;
const SIDE_PANEL_MIN_WIDTH: f32 = 180.0;
const BOTTOM_PANEL_HEIGHT: f32 = 200.0;
const BOTTOM_PANEL_MIN_HEIGHT: f32 = 100.0;

/// What the editor window asks the application to do.
pub(crate) enum EditorAction {
    CloseProject,
    /// Close the window: the user chose to lose unsaved changes.
    Quit,
    /// Show the editor in another language.
    ChangeLanguage(Language),
}

/// Something that closes blueprints, waiting for the user to say what to do
/// with their unsaved changes.
#[derive(Clone)]
enum Leaving {
    CloseTab(usize),
    /// Open the scene at this path of the assets folder in place of the
    /// one open.
    OpenScene(String),
    CloseProject,
    Quit,
}

/// What the panel below the view shows.
#[derive(Clone, Copy, PartialEq, Eq)]
enum BottomTab {
    Assets,
    Problems,
    Output,
    Console,
}

/// Editor for one open project.
pub(crate) struct EditorWindow {
    project: Project,
    assets: AssetsPanel,
    problems: ProblemsPanel,
    output: OutputPanel,
    console: ConsolePanel,
    /// “Play” waits for its check: the game is played if it finds nothing.
    play_after_check: bool,
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
    /// Opens `project`, rendering its scene on the UI's GPU (`render_state`)
    /// and showing the messages of `log_history` in its console.
    pub(crate) fn new(
        project: Project,
        render_state: Option<&egui_wgpu::RenderState>,
        log_history: LogHistory,
    ) -> Self {
        let dimension = project.settings().dimension();
        let assets = AssetsPanel::new(project.directory(), dimension);
        let viewport = render_state
            .ok_or_else(|| texts::current().no_gpu_for_viewport.to_owned())
            .and_then(|state| {
                SceneViewport::new(state, dimension, assets.directory())
                    .map_err(|error| error.to_string())
            });
        if let Err(error) = &viewport {
            log::error!("Scene viewport unavailable: {error}");
        }
        Self {
            project,
            assets,
            problems: ProblemsPanel::new(),
            output: OutputPanel::new(),
            console: ConsolePanel::new(log_history),
            play_after_check: false,
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
        self.follow_check(ui.ctx());
        self.output.update();
        self.handle_quit_request(ui.ctx());
        if ui.input_mut(|input| input.consume_shortcut(&SAVE_SHORTCUT)) {
            self.documents.save_active();
        }
        let state = MenuState {
            can_save: self.documents.active_is_unsaved(),
            compiling: self.problems.is_compiling(),
            playing: self.output.is_playing(),
        };
        egui::Panel::top("menu_bar").show(ui, |ui| {
            match show_menu_bar(ui, state) {
                Some(MenuAction::Save) => self.documents.save_active(),
                Some(MenuAction::CloseProject) => action = self.leave(Leaving::CloseProject),
                Some(MenuAction::Compile) => {
                    // “Compile” checks the files, so it checks what is
                    // shown.
                    self.documents.save_all();
                    self.problems.compile(self.project.directory(), ui.ctx());
                }
                Some(MenuAction::Play) => {
                    // The game is built from what is shown, once checked.
                    self.documents.save_all();
                    self.problems.compile(self.project.directory(), ui.ctx());
                    self.play_after_check = true;
                    self.bottom_tab = BottomTab::Output;
                }
                Some(MenuAction::Stop) => self.output.stop(),
                Some(MenuAction::ShowAbout) => self.about_open = true,
                Some(MenuAction::ChangeLanguage(language)) => {
                    action = Some(EditorAction::ChangeLanguage(language));
                }
                None => {}
            }
        });
        egui::Panel::bottom("status_bar").show(ui, |ui| self.show_status_bar(ui));
        egui::Panel::left("hierarchy")
            .resizable(true)
            .default_size(SIDE_PANEL_WIDTH)
            .min_size(SIDE_PANEL_MIN_WIDTH)
            .show(ui, |ui| self.show_hierarchy(ui));
        egui::Panel::right("inspector")
            .resizable(true)
            .default_size(SIDE_PANEL_WIDTH)
            .min_size(SIDE_PANEL_MIN_WIDTH)
            .show(ui, |ui| match self.documents.active() {
                ActiveDocument::Scene => show_inspector(
                    ui,
                    &self.project,
                    self.documents.scene_mut(),
                    self.assets.images(),
                ),
                ActiveDocument::Blueprint(tab) => {
                    ui.strong(texts::current().inspector);
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

    /// Takes the result of a check that has just finished: its problems are
    /// shown, and “Play” plays the game if there are none.
    fn follow_check(&mut self, context: &egui::Context) {
        let Some(end) = self.problems.update() else {
            return;
        };
        let play = std::mem::take(&mut self.play_after_check);
        match (end, play) {
            (CheckEnd::WithProblems, true) => {
                self.output.refuse(texts::current().play_blocked);
                self.bottom_tab = BottomTab::Problems;
            }
            (CheckEnd::WithProblems, false) => self.bottom_tab = BottomTab::Problems,
            (CheckEnd::Clean, true) => self.play(context),
            (CheckEnd::Clean, false) => {}
        }
    }

    /// Plays the game the last check translated.
    fn play(&mut self, context: &egui::Context) {
        match self.problems.program() {
            Some(program) => self.output.play(
                PlayRequest {
                    name: self.project.settings().name().to_owned(),
                    directory: self.project.directory().to_path_buf(),
                    program: program.clone(),
                },
                context,
            ),
            None => self.output.refuse(texts::current().nothing_to_play),
        }
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
        let unsaved = match &leaving {
            Leaving::CloseTab(index) => self.documents.is_unsaved(*index),
            Leaving::OpenScene(path) => {
                self.documents.scene_is_unsaved() && !self.documents.is_scene_open(path)
            }
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
            Leaving::OpenScene(path) => {
                self.documents.open_scene(
                    self.assets.directory(),
                    path,
                    self.project.settings().dimension(),
                );
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
        let leaving = self.leaving.clone()?;
        let question = match leaving {
            Leaving::CloseTab(_) => texts::current().unsaved_blueprint_question,
            Leaving::OpenScene(_) => texts::current().unsaved_scene_question,
            Leaving::CloseProject | Leaving::Quit => texts::current().unsaved_files_question,
        };
        let choice = show_unsaved_dialog(context, question)?;
        self.leaving = None;
        match choice {
            UnsavedChoice::Cancel => None,
            UnsavedChoice::Discard => self.finish_leaving(leaving),
            UnsavedChoice::Save => {
                let saved = match leaving {
                    Leaving::CloseTab(index) => self.documents.save(index),
                    Leaving::OpenScene(_) => self.documents.save_scene(),
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
                Ok(viewport) => viewport.show(ui, self.documents.scene_mut()),
                Err(error) => {
                    ui.colored_label(ERROR_COLOR, texts::current().viewport_error(error));
                }
            },
            ActiveDocument::Blueprint(tab) => tab.show(ui, self.problems.problems()),
        }
    }

    fn show_bottom_panel(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.bottom_tab,
                BottomTab::Assets,
                texts::current().assets,
            );
            ui.selectable_value(
                &mut self.bottom_tab,
                BottomTab::Problems,
                self.problems.title(),
            );
            ui.selectable_value(
                &mut self.bottom_tab,
                BottomTab::Output,
                texts::current().output,
            );
            ui.selectable_value(
                &mut self.bottom_tab,
                BottomTab::Console,
                texts::current().console,
            );
        });
        ui.separator();
        match self.bottom_tab {
            BottomTab::Assets => match self.assets.show(ui) {
                Some(AssetEvent::Opened(OpenedAsset::Blueprint(path))) => {
                    self.documents.open_blueprint(self.assets.directory(), path);
                }
                Some(AssetEvent::Opened(OpenedAsset::Scene(path))) => {
                    self.leave(Leaving::OpenScene(path));
                }
                Some(AssetEvent::Gone(path)) => self.documents.forget(&path),
                None => {}
            },
            BottomTab::Problems => {
                if let Some(problem) = self.problems.show(ui) {
                    self.show_problem_in_blueprint(&problem);
                }
            }
            BottomTab::Output => self.output.show(ui),
            BottomTab::Console => self.console.show(ui),
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

    /// The “Scene” panel: the tree of the open scene's nodes.
    fn show_hierarchy(&mut self, ui: &mut egui::Ui) {
        ui.strong(texts::current().hierarchy);
        ui.separator();
        match self.documents.scene_mut() {
            Some(scene) => scene.show_tree(ui, self.assets.images()),
            None => {
                ui.weak(texts::current().no_scene_open);
            }
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
                self.output.show_summary(ui);
            });
        });
    }

    fn show_about(&mut self, context: &egui::Context) {
        egui::Window::new(format!("{} {ENGINE_NAME}", texts::current().about))
            .open(&mut self.about_open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(context, |ui| {
                ui.strong(format!("{ENGINE_NAME} {ENGINE_VERSION}"));
                ui.label(texts::current().about_description);
                ui.separator();
                ui.label(texts::current().license_notice);
                ui.label(texts::current().game_exception_notice);
                ui.weak(texts::current().no_warranty);
                ui.hyperlink_to(texts::current().source_code, REPOSITORY_URL);
            });
    }
}

/// Where the source code is published, as the GPL asks programs to tell users.
const REPOSITORY_URL: &str = env!("CARGO_PKG_REPOSITORY");
