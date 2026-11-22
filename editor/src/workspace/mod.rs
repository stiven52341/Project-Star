//! Main editor window: menu, scene hierarchy, viewport, inspector and assets.

mod assets_panel;
mod editor_camera;
mod grid;
mod inspector_panel;
mod menu_bar;
mod problems_panel;
mod viewport;

use eframe::{egui, egui_wgpu};
use star_engine::prelude::{ENGINE_NAME, ENGINE_VERSION};

use self::assets_panel::AssetsPanel;
use self::inspector_panel::show_inspector;
use self::menu_bar::{MenuAction, show_menu_bar};
use self::problems_panel::ProblemsPanel;
use self::viewport::SceneViewport;
use crate::project::Project;
use crate::style::ERROR_COLOR;
use crate::texts;

const SIDE_PANEL_WIDTH: f32 = 260.0;
const SIDE_PANEL_MIN_WIDTH: f32 = 180.0;
const BOTTOM_PANEL_HEIGHT: f32 = 200.0;
const BOTTOM_PANEL_MIN_HEIGHT: f32 = 100.0;

/// What the editor window asks the application to do.
pub(crate) enum EditorAction {
    CloseProject,
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
        let compiling = self.problems.is_compiling();
        egui::Panel::top("menu_bar").show(ui, |ui| match show_menu_bar(ui, compiling) {
            Some(MenuAction::CloseProject) => action = Some(EditorAction::CloseProject),
            Some(MenuAction::Compile) => {
                self.problems.compile(self.project.directory(), ui.ctx());
            }
            Some(MenuAction::ShowAbout) => self.about_open = true,
            None => {}
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
            .show(ui, |ui| show_inspector(ui, &self.project));
        egui::Panel::bottom("bottom_panel")
            .resizable(true)
            .default_size(BOTTOM_PANEL_HEIGHT)
            .min_size(BOTTOM_PANEL_MIN_HEIGHT)
            .show(ui, |ui| self.show_bottom_panel(ui));
        egui::CentralPanel::no_frame().show(ui, |ui| match &mut self.viewport {
            Ok(viewport) => viewport.show(ui),
            Err(error) => {
                ui.colored_label(ERROR_COLOR, texts::viewport_error(error));
            }
        });
        self.show_about(ui.ctx());
        action
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
            BottomTab::Assets => self.assets.show(ui),
            BottomTab::Problems => self.problems.show(ui),
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
