//! Rendering test: a colored triangle spinning in 3D with a perspective camera.
//!
//! The side moving away from the camera must look smaller (perspective) and
//! the back face must stay visible when it turns around. The `pause` action
//! (Space, the left mouse button, a tap or a gamepad's South button) stops
//! and resumes the rotation.
//! Run it with an optional graphics backend:
//! `cargo run -p triangle_3d -- [auto|vulkan|dx12|metal|opengl]`

use std::error::Error;
use std::process::ExitCode;

use star_engine::prelude::*;

const BACKGROUND: Color = Color::rgb(0.05, 0.05, 0.1);
const DEFAULT_LOG_FILTER: &str = "info,wgpu_core=warn,wgpu_hal=warn";
/// Radians per second around the vertical axis.
const ROTATION_SPEED: f32 = 1.0;
const CAMERA_POSITION: Vec3 = Vec3::new(0.0, 0.5, 2.5);
const PAUSE: &str = "pause";

const TRIANGLE: [Vertex; 3] = [
    Vertex::new(Vec3::new(0.0, 0.8, 0.0), Color::rgb(1.0, 0.0, 0.0)),
    Vertex::new(Vec3::new(-0.8, -0.6, 0.0), Color::rgb(0.0, 1.0, 0.0)),
    Vertex::new(Vec3::new(0.8, -0.6, 0.0), Color::rgb(0.0, 0.0, 1.0)),
];

#[derive(Default)]
struct Triangle3d {
    mesh: Option<MeshId>,
    angle: f32,
    paused: bool,
}

impl Game for Triangle3d {
    fn start(&mut self, context: &mut GameContext<'_>) -> EngineResult<()> {
        self.mesh = Some(context.create_mesh(&TRIANGLE)?);
        Ok(())
    }

    fn update(&mut self, time: &Time, input: &Input) {
        if input.actions().just_pressed(PAUSE) {
            self.paused = !self.paused;
        }
        if self.paused {
            return;
        }
        self.angle = (self.angle + ROTATION_SPEED * time.delta_seconds()) % std::f32::consts::TAU;
    }

    fn draw(&self, frame: &mut Frame) {
        frame.set_camera(Camera::new_3d(CAMERA_POSITION, Vec3::ZERO));
        if let Some(mesh) = self.mesh {
            frame.draw(mesh, Mat4::from_rotation_y(self.angle));
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

    let mut actions = ActionMap::new();
    actions.add(
        PAUSE,
        &[
            Key::Space.into(),
            MouseButton::Left.into(),
            Binding::Touch,
            GamepadButton::South.into(),
        ],
    )?;

    App::new()
        .with_title(format!("Triangle 3D — {backend}"))
        .with_actions(actions)
        .with_graphics_backend(backend)
        .with_clear_color(BACKGROUND)
        .run_game(Triangle3d::default())?;
    Ok(())
}
