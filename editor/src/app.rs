//! Top-level editor application: switches between the launcher and the editor.

use eframe::{egui, egui_wgpu};
use star_engine::prelude::ENGINE_NAME;

use crate::editor_log::LogHistory;
use crate::launcher::{Launcher, LauncherAction};
use crate::preferences::PreferencesStore;
use crate::project::Project;
use crate::texts::{self, Language};
use crate::workspace::{EditorAction, EditorWindow};

/// Size of the native window while the launcher is shown.
pub(crate) const LAUNCHER_WINDOW_SIZE: [f32; 2] = [980.0, 620.0];

enum Screen {
    Launcher(Launcher),
    Editor(Box<EditorWindow>),
}

/// The editor process: one screen at a time in a single native window.
pub(crate) struct EditorApp {
    screen: Screen,
    preferences: PreferencesStore,
    /// The log's messages, for the console of each project opened.
    log_history: LogHistory,
}

impl EditorApp {
    /// Starts on the launcher, shown as `preferences` say; the editor's
    /// console shows the messages of `log_history`.
    pub(crate) fn new(preferences: PreferencesStore, log_history: LogHistory) -> Self {
        Self {
            screen: Screen::Launcher(Launcher::new()),
            preferences,
            log_history,
        }
    }

    /// Shows the editor in `language`, window title included.
    fn change_language(&mut self, context: &egui::Context, language: Language) {
        self.preferences.set_language(language);
        if matches!(self.screen, Screen::Launcher(_)) {
            context.send_viewport_cmd(egui::ViewportCommand::Title(launcher_title()));
        }
    }

    fn open_editor(
        context: &egui::Context,
        render_state: Option<&egui_wgpu::RenderState>,
        project: Project,
        log_history: LogHistory,
        icon_size: u16,
    ) -> Screen {
        let editor = EditorWindow::new(project, context, render_state, log_history, icon_size);
        context.send_viewport_cmd(egui::ViewportCommand::Title(editor.title()));
        context.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
        Screen::Editor(Box::new(editor))
    }

    fn open_launcher(context: &egui::Context) -> Screen {
        context.send_viewport_cmd(egui::ViewportCommand::Maximized(false));
        context.send_viewport_cmd(egui::ViewportCommand::InnerSize(
            LAUNCHER_WINDOW_SIZE.into(),
        ));
        context.send_viewport_cmd(egui::ViewportCommand::Title(launcher_title()));
        Screen::Launcher(Launcher::new())
    }
}

impl eframe::App for EditorApp {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        let context = ui.ctx().clone();
        let next_screen = match &mut self.screen {
            Screen::Launcher(launcher) => match egui::CentralPanel::default_margins()
                .show(ui, |ui| launcher.show(ui))
                .inner
            {
                Some(LauncherAction::Open(project)) => Some(Self::open_editor(
                    &context,
                    frame.wgpu_render_state(),
                    project,
                    self.log_history.clone(),
                    self.preferences.asset_icon_size(),
                )),
                Some(LauncherAction::ChangeLanguage(language)) => {
                    self.change_language(&context, language);
                    None
                }
                None => None,
            },
            Screen::Editor(editor) => match editor.show(ui) {
                Some(EditorAction::CloseProject) => Some(Self::open_launcher(&context)),
                Some(EditorAction::Quit) => {
                    context.send_viewport_cmd(egui::ViewportCommand::Close);
                    None
                }
                Some(EditorAction::ChangeLanguage(language)) => {
                    self.change_language(&context, language);
                    None
                }
                Some(EditorAction::ChangeIconSize(size)) => {
                    self.preferences.set_asset_icon_size(size);
                    None
                }
                None => None,
            },
        };
        if let Some(screen) = next_screen {
            self.screen = screen;
        }
    }
}

pub(crate) fn launcher_title() -> String {
    format!("{ENGINE_NAME} — {}", texts::current().project_manager)
}
