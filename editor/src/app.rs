//! Top-level editor application: switches between the launcher and the editor.

use eframe::{egui, egui_wgpu};
use star_engine::prelude::ENGINE_NAME;

use crate::launcher::Launcher;
use crate::project::Project;
use crate::texts;
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
}

impl EditorApp {
    pub(crate) fn new() -> Self {
        Self {
            screen: Screen::Launcher(Launcher::new()),
        }
    }

    fn open_editor(
        context: &egui::Context,
        render_state: Option<&egui_wgpu::RenderState>,
        project: Project,
    ) -> Screen {
        let editor = EditorWindow::new(project, render_state);
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
            Screen::Launcher(launcher) => egui::CentralPanel::default_margins()
                .show(ui, |ui| launcher.show(ui))
                .inner
                .map(|project| Self::open_editor(&context, frame.wgpu_render_state(), project)),
            Screen::Editor(editor) => editor
                .show(ui)
                .map(|EditorAction::CloseProject| Self::open_launcher(&context)),
        };
        if let Some(screen) = next_screen {
            self.screen = screen;
        }
    }
}

pub(crate) fn launcher_title() -> String {
    format!("{ENGINE_NAME} — {}", texts::PROJECT_MANAGER)
}
