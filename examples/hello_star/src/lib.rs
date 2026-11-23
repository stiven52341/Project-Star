//! Minimal ProjectStar application: opens a window filled with a color.
//!
//! Run it with an optional graphics backend:
//! `cargo run -p hello_star -- [auto|vulkan|dx12|metal|opengl]`, or on Android
//! with `cargo apk2 run --lib --release` from its folder.

use std::error::Error;

use star_engine::prelude::*;

const BACKGROUND: Color = Color::rgb(0.1, 0.1, 0.25);

/// Runs the example with the graphics `backend` until its window is closed (on phones it fills the
/// screen).
///
/// # Errors
///
/// Returns an error if the actions are invalid or the engine cannot run.
pub fn run_with(backend: GraphicsBackend) -> Result<(), Box<dyn Error>> {
    App::new()
        .with_title(format!("Hello Star — {backend}"))
        .with_graphics_backend(backend)
        .with_clear_color(BACKGROUND)
        .run()?;
    Ok(())
}

/// Runs the example with the default backend until its window is closed (on phones it fills the
/// screen).
///
/// # Errors
///
/// Returns an error if the actions are invalid or the engine cannot run.
pub fn run() -> Result<(), Box<dyn Error>> {
    run_with(GraphicsBackend::Auto)
}

star_engine::android_main!(run);
