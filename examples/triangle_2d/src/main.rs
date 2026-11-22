//! Rendering test: a single colored triangle seen through a 2D camera.
//!
//! The triangle must keep its shape when the window is resized.
//! Run it with an optional graphics backend:
//! `cargo run -p triangle_2d -- [auto|vulkan|dx12|metal|opengl]`

use std::error::Error;
use std::process::ExitCode;

use star_engine::prelude::*;

const BACKGROUND: Color = Color::rgb(0.1, 0.1, 0.25);
const DEFAULT_LOG_FILTER: &str = "info,wgpu_core=warn,wgpu_hal=warn";
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

    fn draw(&self, frame: &mut Frame) {
        frame.set_camera(Camera::new_2d(VIEW_HEIGHT));
        if let Some(mesh) = self.mesh {
            frame.draw(mesh, Mat4::IDENTITY);
        }
    }
}

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
        .with_title(format!("Triangle 2D — {backend}"))
        .with_graphics_backend(backend)
        .with_clear_color(BACKGROUND)
        .run_game(Triangle2d::default())?;
    Ok(())
}
