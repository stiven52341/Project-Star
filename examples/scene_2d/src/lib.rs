//! Scene test: a solar system read from a `.scene` file and instanced
//! several times.
//!
//! The file describes the nodes (names, parents, transforms and sprites);
//! the game only loads it, adds copies of it to the world and makes the
//! orbits turn. Every planet moves with its orbit node and the moon with the
//! Earth, because each node is placed relative to its parent. Press Space,
//! Enter, a gamepad's south button or tap the screen to add another,
//! smaller copy while the game runs.
//! Run it with an optional graphics backend:
//! `cargo run -p scene_2d -- [auto|vulkan|dx12|metal|opengl]`, or on
//! Android with `cargo apk2 run --lib --release` from its folder.

use std::error::Error;
use std::f32::consts::TAU;

use star_engine::prelude::*;

/// The example's assets, found wherever it is run from.
const ASSET_FOLDER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets");
const SOLAR_SYSTEM: &str = "scenes/solar_system.scene";
const BACKGROUND: Color = Color::rgb(0.02, 0.02, 0.06);
/// World units visible from the bottom to the top of the window.
const VIEW_HEIGHT: f32 = 16.0;
const ADD_SYSTEM: &str = "add_system";
/// Turning speed of each orbit node of the scene, in radians per second.
const ORBIT_SPEEDS: [(&str, f32); 4] = [
    ("EarthOrbit", 0.8),
    ("MoonOrbit", 3.0),
    ("MarsOrbit", 0.45),
    ("Glow", -0.5),
];
/// Where the copies added while playing go, one after another.
const EXTRA_SYSTEMS: [(f32, f32); 6] = [
    (-10.0, 5.0),
    (10.0, 5.0),
    (-10.0, -5.0),
    (10.0, -5.0),
    (0.0, 6.5),
    (0.0, -6.5),
];
const EXTRA_SYSTEM_SCALE: f32 = 0.35;

/// Makes a node turn on itself, in radians per second.
struct Spin(f32);

#[derive(Default)]
struct SceneDemo {
    solar_system: Option<Scene>,
    extra_systems: usize,
}

impl Game for SceneDemo {
    fn start(&mut self, context: &mut GameContext<'_>) -> EngineResult<()> {
        let scene = context.load_scene(SOLAR_SYSTEM)?;
        log::info!("Loaded \"{SOLAR_SYSTEM}\": {} nodes", scene.node_count());
        add_system(context.world(), &scene, Vec2::ZERO, 1.0)?;
        self.solar_system = Some(scene);
        Ok(())
    }

    fn update(&mut self, world: &mut World) {
        let wants_another = world
            .input()
            .is_some_and(|input| input.actions().just_pressed(ADD_SYSTEM));
        if wants_another {
            self.add_extra_system(world);
        }
    }

    fn draw(&self, _world: &World, frame: &mut Frame) {
        frame.set_camera(Camera::new_2d(VIEW_HEIGHT));
    }
}

impl SceneDemo {
    fn add_extra_system(&mut self, world: &mut World) {
        let (Some(scene), Some(&(x, y))) =
            (&self.solar_system, EXTRA_SYSTEMS.get(self.extra_systems))
        else {
            log::info!("There is no room for more solar systems");
            return;
        };
        match add_system(world, scene, Vec2::new(x, y), EXTRA_SYSTEM_SCALE) {
            Ok(()) => {
                self.extra_systems += 1;
                log::info!("Added solar system {}", self.extra_systems + 1);
            }
            Err(error) => log::warn!("{error}"),
        }
    }
}

/// Adds a copy of `scene` at `position`, `scale` times its size, with its
/// orbits turning.
fn add_system(world: &mut World, scene: &Scene, position: Vec2, scale: f32) -> EngineResult<()> {
    let root = scene.instantiate(world, None)?;
    world.insert(
        root,
        Transform2D::from_position(position).with_scale(Vec2::new(scale, scale)),
    )?;
    let mut pending = vec![root];
    while let Some(node) = pending.pop() {
        if let Some(speed) = orbit_speed(world, node) {
            world.insert(node, Spin(speed))?;
        }
        pending.extend(world.children(node));
    }
    Ok(())
}

fn orbit_speed(world: &World, node: Entity) -> Option<f32> {
    let name = world.get::<Name>(node)?;
    ORBIT_SPEEDS
        .iter()
        .find_map(|&(orbit, speed)| (orbit == name.as_str()).then_some(speed))
}

/// Turns every node with a [`Spin`].
fn spin(world: &mut World) -> EngineResult<()> {
    let seconds = world.time().delta_seconds();
    world.each::<(&mut Transform2D, &Spin)>(|_, (transform, spin)| {
        transform.rotation = (transform.rotation + spin.0 * seconds) % TAU;
    })
}

/// Runs the scene with the default backend.
///
/// # Errors
///
/// Returns an error if the actions are invalid or the engine cannot run.
pub fn run() -> Result<(), Box<dyn Error>> {
    run_with(GraphicsBackend::Auto)
}

/// Runs the scene with `backend`, reading its assets from the example's
/// folder (on Android, from its APK).
///
/// # Errors
///
/// Returns an error if the actions are invalid or the engine cannot run.
pub fn run_with(backend: GraphicsBackend) -> Result<(), Box<dyn Error>> {
    let mut actions = ActionMap::new();
    actions.add(
        ADD_SYSTEM,
        &[
            Key::Space.into(),
            Key::Enter.into(),
            GamepadButton::South.into(),
            Binding::Touch,
        ],
    )?;

    App::new()
        .with_title(format!("Scene 2D — {backend}"))
        .with_actions(actions)
        .with_development_assets(ASSET_FOLDER)
        .with_system(Stage::Update, spin)
        .with_graphics_backend(backend)
        .with_clear_color(BACKGROUND)
        .run_game(SceneDemo::default())?;
    Ok(())
}

star_engine::android_main!(run);
