//! Rendering test: a single colored triangle seen through a 2D camera.
//!
//! The triangle must keep its shape when the window is resized.
//! Run it with an optional graphics backend:
//! `cargo run -p triangle_2d -- [auto|vulkan|dx12|metal|opengl]`, or on Android
//! with `cargo apk2 run --lib --release` from its folder.

use std::error::Error;

use star_engine::prelude::*;

const BACKGROUND: Color = Color::rgb(0.1, 0.1, 0.25);
/// World units visible from the bottom to the top of the window.
const VIEW_HEIGHT: f32 = 2.0;

const TRIANGLE: [Vertex; 3] = [
    Vertex::new(Vec3::new(0.0, 0.6, 0.0), Color::rgb(1.0, 0.0, 0.0)),
    Vertex::new(Vec3::new(-0.7, -0.6, 0.0), Color::rgb(0.0, 1.0, 0.0)),
    Vertex::new(Vec3::new(0.7, -0.6, 0.0), Color::rgb(0.0, 0.0, 1.0)),
];

#[derive(Default)]
struct Triangle2d {
    mesh: Option<MeshId>,
}

impl Game for Triangle2d {
    fn start(&mut self, context: &mut GameContext<'_>) -> EngineResult<()> {
        self.mesh = Some(context.create_mesh(&TRIANGLE)?);
        Ok(())
    }

    fn draw(&self, _world: &World, frame: &mut Frame) {
        frame.set_camera(Camera::new_2d(VIEW_HEIGHT));
        if let Some(mesh) = self.mesh {
            frame.draw(mesh, Mat4::IDENTITY);
        }
    }
}

/// Runs the example with the graphics `backend` until its window is closed (on phones it fills the
/// screen).
///
/// # Errors
///
/// Returns an error if the actions are invalid or the engine cannot run.
pub fn run_with(backend: GraphicsBackend) -> Result<(), Box<dyn Error>> {
    App::new()
        .with_title(format!("Triangle 2D — {backend}"))
        .with_graphics_backend(backend)
        .with_clear_color(BACKGROUND)
        .run_game(Triangle2d::default())?;
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
