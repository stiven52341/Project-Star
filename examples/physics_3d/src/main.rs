//! 3D physics test: nodes with bodies that fall, bounce and pile up in an
//! arena.
//!
//! Space (or a click, a tap, or a gamepad's bottom button) drops a ball, C
//! (or the gamepad's left button) a crate. A kinematic bar sweeps the
//! floor, and a sensor plate floating in the middle turns green what goes
//! through it. The oldest things disappear once there are too many. Run it
//! with `cargo run -p physics_3d`.

use std::error::Error;
use std::process::ExitCode;

use star_engine::prelude::*;

const DEFAULT_LOG_FILTER: &str = "info,wgpu_core=warn,wgpu_hal=warn";
const BACKGROUND: Color = Color::rgb(0.45, 0.6, 0.8);

const CAMERA_POSITION: Vec3 = Vec3::new(0.0, 10.0, 14.0);
const LOOK_AT: Vec3 = Vec3::new(0.0, 1.0, 0.0);
const AMBIENT: AmbientLight = AmbientLight::new(Color::rgb(0.6, 0.7, 1.0), 0.2);
/// The sun looks down this many radians, turned this many to the side.
const SUN_PITCH: f32 = -1.0;
const SUN_YAW: f32 = 0.5;
const SUN_INTENSITY: f32 = 2.0;
const SUN_COLOR: Color = Color::rgb(1.0, 0.96, 0.88);

const FLOOR_SIZE: Vec3 = Vec3::new(14.0, 1.0, 14.0);
const FLOOR_Y: f32 = -0.5;
const FLOOR_COLOR: Color = Color::rgb(0.35, 0.4, 0.3);
const WALL_HEIGHT: f32 = 1.0;
const WALL_THICKNESS: f32 = 0.5;
const WALL_COLOR: Color = Color::rgb(0.5, 0.52, 0.6);

const SWEEPER_SIZE: Vec3 = Vec3::new(0.6, 0.6, 10.0);
/// World units per second.
const SWEEPER_SPEED: f32 = 2.5;
/// The sweeper turns back this far from the center.
const SWEEPER_REACH: f32 = 5.5;
const SWEEPER_COLOR: Color = Color::rgb(0.9, 0.6, 0.2);

const SENSOR_SIZE: Vec3 = Vec3::new(4.0, 0.1, 4.0);
const SENSOR_Y: f32 = 3.0;
const SENSOR_COLOR: Color = Color::rgb(0.3, 0.8, 0.4);
const TOUCHED_COLOR: Color = Color::rgb(0.4, 0.95, 0.5);

const BALL_RADIUS: f32 = 0.4;
const BALL_BOUNCINESS: f32 = 0.6;
const BALL_COLOR: Color = Color::rgb(0.5, 0.7, 1.0);
const BALL_SEGMENTS: u32 = 32;
const BALL_RINGS: u32 = 16;
const CRATE_SIZE: Vec3 = Vec3::new(0.9, 0.9, 0.9);
const CRATE_COLOR: Color = Color::rgb(0.85, 0.75, 0.55);
const DROP_Y: f32 = 7.0;
/// Each drop moves this much from the previous one, wrapping around
/// `DROP_WIDTH`, so things do not stack in a single column.
const DROP_STEP: Vec2 = Vec2::new(1.3, 0.9);
const DROP_WIDTH: f32 = 8.0;
/// Things in the arena before the oldest disappear.
const MAX_THINGS: usize = 120;

const BALL: &str = "ball";
const CRATE: &str = "crate";

/// The meshes every node is drawn with: boxes are a unit cube scaled by
/// their node, which never scales their bodies.
struct Meshes {
    cube: Mesh3dId,
    ball: Mesh3dId,
}

#[derive(Default)]
struct Physics3dGame {
    meshes: Option<Meshes>,
    sweeper: Option<Entity>,
    sensor: Option<Entity>,
    /// Dropped things, oldest first.
    things: Vec<Entity>,
    /// Where the next thing drops, over the floor.
    drop_spot: Vec2,
}

impl Physics3dGame {
    fn spawn_level(&mut self, world: &mut World, meshes: &Meshes) -> EngineResult<()> {
        spawn_box(
            world,
            "Floor",
            Vec3::new(0.0, FLOOR_Y, 0.0),
            BodySettings::new(BodyKind::Static, Shape3d::cuboid(FLOOR_SIZE)?),
            FLOOR_SIZE,
            MeshNode::new(meshes.cube, Material::new(FLOOR_COLOR).with_roughness(0.9)),
        )?;
        let half = FLOOR_SIZE.x / 2.0;
        let side_wall = Vec3::new(WALL_THICKNESS, WALL_HEIGHT, FLOOR_SIZE.z);
        let end_wall = Vec3::new(FLOOR_SIZE.x, WALL_HEIGHT, WALL_THICKNESS);
        for (name, position, size) in [
            (
                "Left wall",
                Vec3::new(-half, WALL_HEIGHT / 2.0, 0.0),
                side_wall,
            ),
            (
                "Right wall",
                Vec3::new(half, WALL_HEIGHT / 2.0, 0.0),
                side_wall,
            ),
            (
                "Back wall",
                Vec3::new(0.0, WALL_HEIGHT / 2.0, -half),
                end_wall,
            ),
            (
                "Front wall",
                Vec3::new(0.0, WALL_HEIGHT / 2.0, half),
                end_wall,
            ),
        ] {
            spawn_box(
                world,
                name,
                position,
                BodySettings::new(BodyKind::Static, Shape3d::cuboid(size)?),
                size,
                MeshNode::new(meshes.cube, Material::new(WALL_COLOR)),
            )?;
        }
        self.sweeper = Some(spawn_box(
            world,
            "Sweeper",
            Vec3::new(0.0, SWEEPER_SIZE.y / 2.0, 0.0),
            BodySettings::new(BodyKind::Kinematic, Shape3d::cuboid(SWEEPER_SIZE)?),
            SWEEPER_SIZE,
            MeshNode::new(meshes.cube, Material::new(SWEEPER_COLOR).with_metallic(1.0)),
        )?);
        let mut sensor = BodySettings::new(BodyKind::Static, Shape3d::cuboid(SENSOR_SIZE)?);
        sensor.is_sensor = true;
        self.sensor = Some(spawn_box(
            world,
            "Sensor",
            Vec3::new(0.0, SENSOR_Y, 0.0),
            sensor,
            SENSOR_SIZE,
            MeshNode::new(meshes.cube, Material::new(SENSOR_COLOR)),
        )?);
        let sun = world.spawn_node("Sun", None)?;
        world.insert(
            sun,
            Transform3D::IDENTITY
                .with_rotation(Quat::from_euler(Vec3::new(SUN_PITCH, SUN_YAW, 0.0))),
        )?;
        world.insert(
            sun,
            LightNode::Directional {
                color: SUN_COLOR,
                intensity: SUN_INTENSITY,
            },
        )?;
        Ok(())
    }

    fn drop_thing(&mut self, world: &mut World, is_ball: bool) -> EngineResult<()> {
        let Some(meshes) = &self.meshes else {
            return Ok(());
        };
        let position = Vec3::new(self.drop_spot.x, DROP_Y, self.drop_spot.y);
        self.drop_spot = wrap(self.drop_spot + DROP_STEP);
        let thing = if is_ball {
            let mut ball = BodySettings::new(BodyKind::Dynamic, Shape3d::sphere(BALL_RADIUS)?);
            ball.bounciness = BALL_BOUNCINESS;
            let look = Material::new(BALL_COLOR).with_roughness(0.3);
            spawn_body(
                world,
                "Ball",
                Transform3D::from_position(position),
                ball,
                MeshNode::new(meshes.ball, look),
            )?
        } else {
            spawn_box(
                world,
                "Crate",
                position,
                BodySettings::new(BodyKind::Dynamic, Shape3d::cuboid(CRATE_SIZE)?),
                CRATE_SIZE,
                MeshNode::new(meshes.cube, Material::new(CRATE_COLOR).with_roughness(0.8)),
            )?
        };
        self.things.push(thing);
        if self.things.len() > MAX_THINGS {
            let oldest = self.things.remove(0);
            world.despawn_node(oldest);
        }
        Ok(())
    }

    /// Sweeps the bar from side to side, turning at the ends.
    fn move_sweeper(&self, world: &World) {
        let Some(sweeper) = self.sweeper else {
            return;
        };
        let Some(x) = world
            .get::<Transform3D>(sweeper)
            .map(|transform| transform.position.x)
        else {
            return;
        };
        let Some(mut body) = world.get_mut::<Body3D>(sweeper) else {
            return;
        };
        let going_right = if x > SWEEPER_REACH {
            false
        } else if x < -SWEEPER_REACH {
            true
        } else {
            body.velocity().x >= 0.0
        };
        let speed = if going_right {
            SWEEPER_SPEED
        } else {
            -SWEEPER_SPEED
        };
        body.set_velocity(Vec3::new(speed, 0.0, 0.0));
    }

    /// Paints green what went through the sensor in the last frame.
    fn paint_sensed(&self, world: &World) {
        let Some(sensor) = self.sensor else {
            return;
        };
        let Some(nodes) = world.physics_nodes_3d() else {
            return;
        };
        for contact in nodes.contacts() {
            if contact.phase == ContactPhase::Started
                && let Some(thing) = contact.other(sensor)
                && let Some(mut mesh) = world.get_mut::<MeshNode>(thing)
            {
                mesh.material.base_color = TOUCHED_COLOR;
            }
        }
    }
}

/// `spot` brought back inside the square of side `DROP_WIDTH`.
fn wrap(spot: Vec2) -> Vec2 {
    let half = DROP_WIDTH / 2.0;
    let wrap_one = |value: f32| {
        if value > half {
            value - DROP_WIDTH
        } else {
            value
        }
    };
    Vec2::new(wrap_one(spot.x), wrap_one(spot.y))
}

/// Spawns a node placed by `transform` with a body and a mesh.
fn spawn_body(
    world: &mut World,
    name: &str,
    transform: Transform3D,
    body: BodySettings<Shape3d>,
    mesh: MeshNode,
) -> EngineResult<Entity> {
    let node = world.spawn_node(name, None)?;
    world.insert(node, transform)?;
    world.insert(node, Body3D::new(body))?;
    world.insert(node, mesh)?;
    Ok(node)
}

/// Spawns a box at `position`: a unit cube mesh scaled to `size`, with a
/// body of the same size.
fn spawn_box(
    world: &mut World,
    name: &str,
    position: Vec3,
    body: BodySettings<Shape3d>,
    size: Vec3,
    mesh: MeshNode,
) -> EngineResult<Entity> {
    let transform = Transform3D::from_position(position).with_scale(size);
    spawn_body(world, name, transform, body, mesh)
}

impl Game for Physics3dGame {
    fn start(&mut self, context: &mut GameContext<'_>) -> EngineResult<()> {
        let meshes = Meshes {
            cube: context.create_mesh_3d(&MeshData::cube(Vec3::ONE)?)?,
            ball: context.create_mesh_3d(&MeshData::sphere(
                BALL_RADIUS,
                BALL_SEGMENTS,
                BALL_RINGS,
            )?)?,
        };
        self.spawn_level(context.world(), &meshes)?;
        self.meshes = Some(meshes);
        Ok(())
    }

    fn update(&mut self, world: &mut World) {
        let (drop_ball, drop_crate) = world.input().map_or((false, false), |input| {
            let actions = input.actions();
            (actions.just_pressed(BALL), actions.just_pressed(CRATE))
        });
        for (wanted, is_ball) in [(drop_ball, true), (drop_crate, false)] {
            if wanted && let Err(error) = self.drop_thing(world, is_ball) {
                log::warn!("{error}");
            }
        }
        self.move_sweeper(world);
        self.paint_sensed(world);
    }

    fn draw(&self, _world: &World, frame: &mut Frame) {
        frame.set_camera(Camera::new_3d(CAMERA_POSITION, LOOK_AT));
        frame.set_ambient_light(AMBIENT);
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
    let mut actions = ActionMap::new();
    actions.add(
        BALL,
        &[
            Key::Space.into(),
            MouseButton::Left.into(),
            Binding::Touch,
            GamepadButton::South.into(),
        ],
    )?;
    actions.add(CRATE, &[Key::C.into(), GamepadButton::West.into()])?;

    App::new()
        .with_title("Physics 3D")
        .with_actions(actions)
        .with_clear_color(BACKGROUND)
        .run_game(Physics3dGame::default())?;
    Ok(())
}
