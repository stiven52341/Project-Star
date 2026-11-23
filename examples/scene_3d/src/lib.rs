//! 3D rendering test: meshes with PBR materials, lit by the sun, a moving
//! lamp and a dim ambient light.
//!
//! A row of spheres goes from rough (left) to polished (right), the front
//! ones plastic and the back ones metal; a checkered cube spins in the
//! middle, and an orange lamp circles above them. The camera orbits the
//! scene; the `pause` action (Space, the left mouse button, a tap or a
//! gamepad's South button) stops and resumes it. Run it with
//! `cargo run -p scene_3d`, or on Android with `cargo apk2 run -p scene_3d`.

use std::error::Error;
use std::f32::consts::TAU;

use star_engine::prelude::*;

const BACKGROUND: Color = Color::rgb(0.45, 0.6, 0.8);
const PAUSE: &str = "pause";

/// Radians per second of the camera around the scene.
const ORBIT_SPEED: f32 = 0.25;
const ORBIT_RADIUS: f32 = 9.0;
const CAMERA_HEIGHT: f32 = 4.0;
const LOOK_AT: Vec3 = Vec3::new(0.0, 0.5, 0.0);

const GROUND_SIZE: Vec2 = Vec2::new(16.0, 16.0);
const GROUND: Material = Material::new(Color::rgb(0.35, 0.4, 0.3)).with_roughness(0.9);

const SPHERE_RADIUS: f32 = 0.5;
const SPHERE_SEGMENTS: u32 = 48;
const SPHERE_RINGS: u32 = 24;
/// Spheres in each row, from roughest to smoothest.
const SPHERES_PER_ROW: u16 = 5;
const SPHERE_SPACING: f32 = 1.3;
const PLASTIC_ROW_Z: f32 = 2.0;
const METAL_ROW_Z: f32 = 3.3;
const PLASTIC_COLOR: Color = Color::rgb(0.8, 0.1, 0.1);
const METAL_COLOR: Color = Color::rgb(0.95, 0.75, 0.35);
/// The smoothest sphere is not a perfect mirror, which only reflects lights
/// as points.
const SMOOTHEST: f32 = 0.1;

const CUBE_SIZE: f32 = 1.4;
const CUBE_HEIGHT: f32 = 1.0;
/// Radians per second of the cube around itself.
const CUBE_SPIN: f32 = 0.6;
const CHECKER_CELLS: u32 = 8;
const CHECKER_LIGHT: [u8; 4] = [235, 235, 235, 255];
const CHECKER_DARK: [u8; 4] = [40, 90, 160, 255];

const SUN: DirectionalLight = DirectionalLight {
    direction: Vec3::new(-0.4, -1.0, -0.6),
    color: Color::rgb(1.0, 0.96, 0.88),
    intensity: 2.0,
};
const AMBIENT: AmbientLight = AmbientLight::new(Color::rgb(0.6, 0.7, 1.0), 0.15);
const LAMP_COLOR: Color = Color::rgb(1.0, 0.55, 0.2);
const LAMP_INTENSITY: f32 = 4.0;
const LAMP_RANGE: f32 = 6.0;
const LAMP_ORBIT_RADIUS: f32 = 3.0;
const LAMP_HEIGHT: f32 = 1.5;
/// Radians per second of the lamp around the cube.
const LAMP_SPEED: f32 = 0.9;

struct Meshes {
    ground: Mesh3dId,
    sphere: Mesh3dId,
    cube: Mesh3dId,
    checker: TextureId,
}

#[derive(Default)]
struct Scene3d {
    meshes: Option<Meshes>,
    /// Seconds the scene has been moving.
    time: f32,
    paused: bool,
}

/// A square image of `CHECKER_CELLS` by `CHECKER_CELLS` cells.
fn checker_image() -> EngineResult<Image> {
    let pixels = (0..CHECKER_CELLS * CHECKER_CELLS)
        .flat_map(|cell| {
            let (column, row) = (cell % CHECKER_CELLS, cell / CHECKER_CELLS);
            if (column + row) % 2 == 0 {
                CHECKER_LIGHT
            } else {
                CHECKER_DARK
            }
        })
        .collect();
    Image::from_rgba8(CHECKER_CELLS, CHECKER_CELLS, pixels)
}

impl Scene3d {
    fn draw_spheres(meshes: &Meshes, frame: &mut Frame) {
        let first_x = -SPHERE_SPACING * f32::from(SPHERES_PER_ROW - 1) / 2.0;
        for index in 0..SPHERES_PER_ROW {
            let along = f32::from(index) / f32::from(SPHERES_PER_ROW - 1);
            let roughness = 1.0 - along * (1.0 - SMOOTHEST);
            let x = first_x + SPHERE_SPACING * f32::from(index);
            for (z, color, metallic) in [
                (PLASTIC_ROW_Z, PLASTIC_COLOR, 0.0),
                (METAL_ROW_Z, METAL_COLOR, 1.0),
            ] {
                let material = Material::new(color)
                    .with_metallic(metallic)
                    .with_roughness(roughness);
                let place = Mat4::from_translation(Vec3::new(x, SPHERE_RADIUS, -z));
                frame.draw_mesh_3d(meshes.sphere, material, place);
            }
        }
    }

    fn camera(&self) -> Camera {
        let angle = self.time * ORBIT_SPEED;
        let position = Vec3::new(
            angle.sin() * ORBIT_RADIUS,
            CAMERA_HEIGHT,
            angle.cos() * ORBIT_RADIUS,
        );
        Camera::new_3d(position, LOOK_AT)
    }

    fn lamp(&self) -> PointLight {
        let angle = (self.time * LAMP_SPEED) % TAU;
        PointLight {
            position: Vec3::new(
                angle.cos() * LAMP_ORBIT_RADIUS,
                LAMP_HEIGHT,
                angle.sin() * LAMP_ORBIT_RADIUS,
            ),
            color: LAMP_COLOR,
            intensity: LAMP_INTENSITY,
            range: LAMP_RANGE,
        }
    }
}

impl Game for Scene3d {
    fn start(&mut self, context: &mut GameContext<'_>) -> EngineResult<()> {
        let checker = context.create_texture(&checker_image()?, TextureFilter::Pixelated)?;
        self.meshes = Some(Meshes {
            ground: context.create_mesh_3d(&MeshData::plane(GROUND_SIZE)?)?,
            sphere: context.create_mesh_3d(&MeshData::sphere(
                SPHERE_RADIUS,
                SPHERE_SEGMENTS,
                SPHERE_RINGS,
            )?)?,
            cube: context.create_mesh_3d(&MeshData::cube(Vec3::ONE * CUBE_SIZE)?)?,
            checker,
        });
        Ok(())
    }

    fn update(&mut self, world: &mut World) {
        if world
            .input()
            .is_some_and(|input| input.actions().just_pressed(PAUSE))
        {
            self.paused = !self.paused;
        }
        if !self.paused {
            self.time += world.time().delta_seconds();
        }
    }

    fn draw(&self, _world: &World, frame: &mut Frame) {
        frame.set_camera(self.camera());
        frame.set_ambient_light(AMBIENT);
        frame.add_light(SUN);
        frame.add_light(self.lamp());
        let Some(meshes) = &self.meshes else {
            return;
        };
        frame.draw_mesh_3d(meshes.ground, GROUND, Mat4::IDENTITY);
        Self::draw_spheres(meshes, frame);
        let spin = Mat4::from_translation(Vec3::new(0.0, CUBE_HEIGHT, 0.0))
            * Mat4::from_rotation_y(self.time * CUBE_SPIN)
            * Mat4::from_rotation_x(self.time * CUBE_SPIN / 2.0);
        let checkered = Material::default()
            .with_texture(meshes.checker)
            .with_roughness(0.4);
        frame.draw_mesh_3d(meshes.cube, checkered, spin);
    }
}

/// Runs the scene in a window (the whole screen on phones) until it is
/// closed.
///
/// # Errors
///
/// Returns an error if the actions are invalid or the engine cannot run.
pub fn run() -> Result<(), Box<dyn Error>> {
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
        .with_title("Scene 3D")
        .with_actions(actions)
        .with_clear_color(BACKGROUND)
        .run_game(Scene3d::default())?;
    Ok(())
}

star_engine::android_main!(run);
