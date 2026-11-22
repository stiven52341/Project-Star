//! Minimal ProjectStar application: opens a window filled with a color.
//!
//! Run it with an optional graphics backend:
//! `cargo run -p hello_star -- [auto|vulkan|dx12|metal|opengl]`

use std::error::Error;
use std::process::ExitCode;

use star_engine::prelude::*;

const BACKGROUND: Color = Color::rgb(0.1, 0.1, 0.25);
const DEFAULT_LOG_FILTER: &str = "info,wgpu_core=warn,wgpu_hal=warn";

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

fn run() -> Result<(), Box<dyn Error>> {
    let backend = match std::env::args().nth(1) {
        Some(name) => name.parse()?,
        None => GraphicsBackend::Auto,
    };

    App::new()
        .with_title(format!("Hello Star — {backend}"))
        .with_graphics_backend(backend)
        .with_clear_color(BACKGROUND)
        .run()?;
    Ok(())
}
