//! 2D physics test: nodes with bodies that fall, bounce and pile up.
//!
//! Space (or a click, or a gamepad's bottom button) drops a ball, C (or the
//! gamepad's left button) a crate. A kinematic paddle sweeps the floor, and
//! a sensor in the middle turns green what goes through it. The oldest
//! things disappear once there are too many. Run it with
//! `cargo run -p physics_2d`.

use std::error::Error;
use std::process::ExitCode;

use star_engine::prelude::*;

const DEFAULT_LOG_FILTER: &str = "info,wgpu_core=warn,wgpu_hal=warn";
const BACKGROUND: Color = Color::rgb(0.07, 0.08, 0.12);
/// World units visible from the bottom to the top of the window.
const VIEW_HEIGHT: f32 = 12.0;

/// Pixels per side of the generated round image.
const CIRCLE_PIXELS: u32 = 64;
const WHITE_PIXEL: [u8; 4] = [255, 255, 255, 255];

const FLOOR_Y: f32 = -5.5;
const FLOOR_SIZE: Vec2 = Vec2::new(16.0, 1.0);
const WALL_X: f32 = 8.0;
const WALL_SIZE: Vec2 = Vec2::new(1.0, 12.0);
const WALL_COLOR: Color = Color::rgb(0.3, 0.32, 0.4);

const PADDLE_SIZE: Vec2 = Vec2::new(2.5, 0.4);
const PADDLE_Y: f32 = -4.6;
/// World units per second.
const PADDLE_SPEED: f32 = 3.0;
/// The paddle turns back this far from the center.
const PADDLE_REACH: f32 = 5.5;
const PADDLE_COLOR: Color = Color::rgb(0.9, 0.6, 0.2);

const SENSOR_SIZE: Vec2 = Vec2::new(3.0, 1.5);
const SENSOR_Y: f32 = 0.5;
const SENSOR_COLOR: Color = Color::rgba(0.3, 0.9, 0.4, 0.25);
const TOUCHED_COLOR: Color = Color::rgb(0.4, 0.95, 0.5);

const BALL_RADIUS: f32 = 0.35;
const BALL_BOUNCINESS: f32 = 0.6;
const BALL_COLOR: Color = Color::rgb(0.5, 0.7, 1.0);
const CRATE_SIZE: Vec2 = Vec2::new(0.8, 0.8);
const CRATE_COLOR: Color = Color::rgb(0.85, 0.75, 0.55);
const DROP_Y: f32 = 5.0;
/// Each drop moves this much to the right of the previous one, wrapping
/// around `DROP_WIDTH`, so things do not stack in a single column.
const DROP_STEP: f32 = 1.7;
const DROP_WIDTH: f32 = 12.0;
/// Things on screen before the oldest disappear.
const MAX_THINGS: usize = 80;

const BALL: &str = "ball";
const CRATE: &str = "crate";

/// The images every sprite is drawn with.
struct Images {
    square: TextureId,
    circle: TextureId,
}

#[derive(Default)]
struct PhysicsGame {
    images: Option<Images>,
    paddle: Option<Entity>,
    sensor: Option<Entity>,
    /// Dropped things, oldest first.
    things: Vec<Entity>,
    /// Where the next thing drops.
    drop_x: f32,
}

impl PhysicsGame {
    fn spawn_level(&mut self, world: &mut World, images: &Images) -> EngineResult<()> {
        let floor = Shape2d::rectangle(FLOOR_SIZE)?;
        let wall = Shape2d::rectangle(WALL_SIZE)?;
        spawn_body(
            world,
            "Floor",
            Vec2::new(0.0, FLOOR_Y),
            BodySettings::new(BodyKind::Static, floor),
            SpriteNode::new(images.square, FLOOR_SIZE).with_color(WALL_COLOR),
        )?;
        for (name, x) in [("Left wall", -WALL_X), ("Right wall", WALL_X)] {
            spawn_body(
                world,
                name,
                Vec2::new(x, 0.0),
                BodySettings::new(BodyKind::Static, wall),
                SpriteNode::new(images.square, WALL_SIZE).with_color(WALL_COLOR),
            )?;
        }
        self.paddle = Some(spawn_body(
            world,
            "Paddle",
            Vec2::new(0.0, PADDLE_Y),
            BodySettings::new(BodyKind::Kinematic, Shape2d::rectangle(PADDLE_SIZE)?),
            SpriteNode::new(images.square, PADDLE_SIZE).with_color(PADDLE_COLOR),
        )?);
        let mut sensor = BodySettings::new(BodyKind::Static, Shape2d::rectangle(SENSOR_SIZE)?);
        sensor.is_sensor = true;
        self.sensor = Some(spawn_body(
            world,
            "Sensor",
            Vec2::new(0.0, SENSOR_Y),
            sensor,
            SpriteNode::new(images.square, SENSOR_SIZE).with_color(SENSOR_COLOR),
        )?);
        Ok(())
    }

    fn drop_thing(&mut self, world: &mut World, is_ball: bool) -> EngineResult<()> {
        let Some(images) = &self.images else {
            return Ok(());
        };
        let position = Vec2::new(self.drop_x, DROP_Y);
        self.drop_x += DROP_STEP;
        if self.drop_x > DROP_WIDTH / 2.0 {
            self.drop_x -= DROP_WIDTH;
        }
        let thing = if is_ball {
            let mut ball = BodySettings::new(BodyKind::Dynamic, Shape2d::circle(BALL_RADIUS)?);
            ball.bounciness = BALL_BOUNCINESS;
            let size = Vec2::new(BALL_RADIUS * 2.0, BALL_RADIUS * 2.0);
            let sprite = SpriteNode::new(images.circle, size).with_color(BALL_COLOR);
            spawn_body(world, "Ball", position, ball, sprite)?
        } else {
            let box_shape = BodySettings::new(BodyKind::Dynamic, Shape2d::rectangle(CRATE_SIZE)?);
            let sprite = SpriteNode::new(images.square, CRATE_SIZE).with_color(CRATE_COLOR);
            spawn_body(world, "Crate", position, box_shape, sprite)?
        };
        self.things.push(thing);
        if self.things.len() > MAX_THINGS {
            let oldest = self.things.remove(0);
            world.despawn_node(oldest);
        }
        Ok(())
    }

    /// Sweeps the paddle from side to side, turning at the ends.
    fn move_paddle(&self, world: &World) {
        let Some(paddle) = self.paddle else {
            return;
        };
        let Some(body) = world.get::<Body2D>(paddle).and_then(|body| body.id()) else {
            return;
        };
        let Some(mut physics) = world.physics_mut() else {
            return;
        };
        let (Some(position), Some(velocity)) = (physics.position(body), physics.velocity(body))
        else {
            return;
        };
        let going_right = if position.x > PADDLE_REACH {
            false
        } else if position.x < -PADDLE_REACH {
            true
        } else {
            velocity.x >= 0.0
        };
        let speed = if going_right {
            PADDLE_SPEED
        } else {
            -PADDLE_SPEED
        };
        physics.set_velocity(body, Vec2::new(speed, 0.0));
    }

    /// Paints green what went through the sensor in the last frame.
    fn paint_sensed(&self, world: &World) {
        let Some(sensor) = self.sensor else {
            return;
        };
        let Some(nodes) = world.physics_nodes() else {
            return;
        };
        for contact in nodes.contacts() {
            if contact.phase == ContactPhase::Started
                && let Some(thing) = contact.other(sensor)
                && let Some(mut sprite) = world.get_mut::<SpriteNode>(thing)
            {
                sprite.color = TOUCHED_COLOR;
            }
        }
    }
}

/// Spawns a node at `position` with a body and a sprite.
fn spawn_body(
    world: &mut World,
    name: &str,
    position: Vec2,
    body: BodySettings,
    sprite: SpriteNode,
) -> EngineResult<Entity> {
    let node = world.spawn_node(name, None)?;
    world.insert(node, Transform2D::from_position(position))?;
    world.insert(node, Body2D::new(body))?;
    world.insert(node, sprite)?;
    Ok(node)
}

/// A white disc on a transparent background.
fn circle_image() -> EngineResult<Image> {
    let radius = f64::from(CIRCLE_PIXELS) / 2.0;
    let mut pixels = Vec::new();
    for y in 0..CIRCLE_PIXELS {
        for x in 0..CIRCLE_PIXELS {
            let dx = f64::from(x) + 0.5 - radius;
            let dy = f64::from(y) + 0.5 - radius;
            let inside = dx.hypot(dy) <= radius;
            pixels.extend_from_slice(&[255, 255, 255, if inside { 255 } else { 0 }]);
        }
    }
    Image::from_rgba8(CIRCLE_PIXELS, CIRCLE_PIXELS, pixels)
}

impl Game for PhysicsGame {
    fn start(&mut self, context: &mut GameContext<'_>) -> EngineResult<()> {
        let square = Image::from_rgba8(1, 1, WHITE_PIXEL.to_vec())?;
        let images = Images {
            square: context.create_texture(&square, TextureFilter::Pixelated)?,
            circle: context.create_texture(&circle_image()?, TextureFilter::Smooth)?,
        };
        self.spawn_level(context.world(), &images)?;
        self.images = Some(images);
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
        self.move_paddle(world);
        self.paint_sensed(world);
    }

    fn draw(&self, _world: &World, frame: &mut Frame) {
        frame.set_camera(Camera::new_2d(VIEW_HEIGHT));
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
            GamepadButton::South.into(),
        ],
    )?;
    actions.add(CRATE, &[Key::C.into(), GamepadButton::West.into()])?;

    App::new()
        .with_title("Physics 2D")
        .with_actions(actions)
        .with_clear_color(BACKGROUND)
        .run_game(PhysicsGame::default())?;
    Ok(())
}
