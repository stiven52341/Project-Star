//! Rendering test: a colored triangle spinning in 3D with a perspective camera.
//!
//! The side moving away from the camera must look smaller (perspective) and
//! the back face must stay visible when it turns around. The `pause` action
//! (Space, the left mouse button, a tap or a gamepad's South button) stops
//! and resumes the rotation.
//! Run it with an optional graphics backend:
//! `cargo run -p triangle_3d -- [auto|vulkan|dx12|metal|opengl]`, or on Android
//! with `cargo apk2 run --lib --release` from its folder.

use std::error::Error;

use star_engine::prelude::*;

const BACKGROUND: Color = Color::rgb(0.05, 0.05, 0.1);
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

    fn update(&mut self, world: &mut World) {
        if world
            .input()
            .is_some_and(|input| input.actions().just_pressed(PAUSE))
        {
            self.paused = !self.paused;
        }
        if self.paused {
            return;
        }
        let seconds = world.time().delta_seconds();
        self.angle = (self.angle + ROTATION_SPEED * seconds) % std::f32::consts::TAU;
    }

    fn draw(&self, _world: &World, frame: &mut Frame) {
        frame.set_camera(Camera::new_3d(CAMERA_POSITION, Vec3::ZERO));
        if let Some(mesh) = self.mesh {
            frame.draw(mesh, Mat4::from_rotation_y(self.angle));
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
