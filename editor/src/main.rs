//! ProjectStar visual editor. Starts on the project manager.

mod app;
mod blueprint_editor;
mod config_file;
mod drag_drop;
mod editor_log;
mod file_tree;
mod folder_dialog;
mod frames_editor;
mod game_run;
mod language_menu;
mod last_scenes;
mod launcher;
mod preferences;
mod project;
mod project_check;
mod recent_projects;
mod scene_editor;
mod script_editor;
mod style;
mod texts;
mod workspace;

use std::process::ExitCode;

use eframe::egui;
use star_engine::prelude::ENGINE_NAME;

use crate::app::{EditorApp, LAUNCHER_WINDOW_SIZE, launcher_title};
use crate::editor_log::LogHistory;
use crate::preferences::PreferencesStore;

const DEFAULT_LOG_FILTER: &str = "info,wgpu_core=warn,wgpu_hal=warn";
const MIN_WINDOW_SIZE: [f32; 2] = [800.0, 520.0];

fn main() -> ExitCode {
    let log_history = LogHistory::new();
    if let Err(error) = editor_log::install(DEFAULT_LOG_FILTER, log_history.clone()) {
        // Only possible if something installed a logger before: the editor
        // still works, without its messages.
        eprintln!("The editor's log is unavailable: {error}");
    }

    match run(log_history) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log::error!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run(log_history: LogHistory) -> eframe::Result {
    // Before the first text is shown, so it is already in the chosen language.
    let preferences = PreferencesStore::load();
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default()
            .with_title(launcher_title())
            .with_inner_size(LAUNCHER_WINDOW_SIZE)
            .with_min_inner_size(MIN_WINDOW_SIZE),
        ..Default::default()
    };
    eframe::run_native(
        ENGINE_NAME,
        options,
        Box::new(|_creation_context| Ok(Box::new(EditorApp::new(preferences, log_history)))),
    )
}
