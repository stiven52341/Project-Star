//! ProjectStar visual editor. Starts on the project manager.

mod app;
mod folder_dialog;
mod launcher;
mod project;
mod recent_projects;
mod style;
mod texts;
mod workspace;

use std::process::ExitCode;

use eframe::egui;
use star_engine::prelude::ENGINE_NAME;

use crate::app::{EditorApp, LAUNCHER_WINDOW_SIZE, launcher_title};

const DEFAULT_LOG_FILTER: &str = "info,wgpu_core=warn,wgpu_hal=warn";
const MIN_WINDOW_SIZE: [f32; 2] = [800.0, 520.0];

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(DEFAULT_LOG_FILTER))
        .init();

    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            log::error!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> eframe::Result {
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
        Box::new(|_creation_context| Ok(Box::new(EditorApp::new()))),
    )
}
