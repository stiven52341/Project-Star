//! Rendering test: hundreds of sprites drawn in a few batches.
//!
//! A tiled floor (pixelated texture) lies under hundreds of tinted,
//! translucent balls (smooth texture) that bounce around. On top, a hero
//! animated from a sprite sheet walks with the arrow keys, WASD or a
//! gamepad's directional pad, and turns to face where it goes. Every
//! texture is generated in code, pixel by pixel.
//! Run it with an optional graphics backend:
//! `cargo run -p sprites_2d -- [auto|vulkan|dx12|metal|opengl]`

use std::error::Error;
use std::f32::consts::TAU;
use std::process::ExitCode;

use star_engine::prelude::*;

const BACKGROUND: Color = Color::rgb(0.05, 0.05, 0.1);
const DEFAULT_LOG_FILTER: &str = "info,wgpu_core=warn,wgpu_hal=warn";
/// World units visible from the bottom to the top of the window.
const VIEW_HEIGHT: f32 = 10.0;
/// Where balls bounce, around the origin; wide enough for 16:9 windows.
const ARENA: Vec2 = Vec2::new(8.5, 4.8);

const FLOOR_LAYER: i32 = -1;
const BALL_LAYER: i32 = 0;
const HERO_LAYER: i32 = 1;

/// Floor tiles in each direction from the origin.
const FLOOR_TILES: i16 = 9;
const FLOOR_LIGHT: [u8; 4] = [60, 60, 80, 255];
const FLOOR_DARK: [u8; 4] = [40, 40, 55, 255];

const BALL_COUNT: usize = 600;
const BALL_PIXELS: u16 = 32;
/// Balls are between this size and twice it, in world units.
const BALL_MIN_SIZE: f32 = 0.2;
/// Fastest speed of a ball, in world units per second.
const BALL_MAX_SPEED: f32 = 3.0;
const BALL_OPACITY: f32 = 0.7;
/// Fixed seed: every run looks the same.
const SEED: u32 = 0x5EED;

/// The hero's sheet: 4 frames in a row, each a square of this many pixels.
const HERO_FRAMES: u16 = 4;
const HERO_FRAME_PIXELS: u16 = 8;
const HERO_FRAMES_PER_SECOND: f32 = 8.0;
const HERO_SIZE: f32 = 1.2;
/// World units per second.
const HERO_SPEED: f32 = 4.0;
const HERO_BODY: [u8; 4] = [250, 200, 80, 255];
const HERO_EYE: [u8; 4] = [30, 30, 30, 255];

const LEFT: &str = "left";
const RIGHT: &str = "right";
const UP: &str = "up";
const DOWN: &str = "down";

struct Textures {
    floor: TextureId,
    ball: TextureId,
    hero: TextureId,
}

struct Ball {
    position: Vec2,
    velocity: Vec2,
    size: f32,
    color: Color,
}

struct Hero {
    position: Vec2,
    facing_left: bool,
    /// Progress through the walk cycle, in frames: from 0 up to `HERO_FRAMES`.
    animation: f32,
}

struct Sprites2d {
    textures: Option<Textures>,
    balls: Vec<Ball>,
    hero: Hero,
}

impl Sprites2d {
    fn new() -> Self {
        let mut random = Random(SEED);
        Self {
            textures: None,
            balls: (0..BALL_COUNT).map(|_| Ball::random(&mut random)).collect(),
            hero: Hero {
                position: Vec2::ZERO,
                facing_left: false,
                animation: 0.0,
            },
        }
    }
}

impl Game for Sprites2d {
    fn start(&mut self, context: &mut GameContext<'_>) -> EngineResult<()> {
        self.textures = Some(Textures {
            floor: context.create_texture(&floor_image()?, TextureFilter::Pixelated)?,
            ball: context.create_texture(&ball_image()?, TextureFilter::Smooth)?,
            hero: context.create_texture(&hero_sheet()?, TextureFilter::Pixelated)?,
        });
        Ok(())
    }

    fn update(&mut self, time: &Time, input: &Input) {
        let seconds = time.delta_seconds();
        for ball in &mut self.balls {
            ball.advance(seconds);
        }
        self.hero.walk(direction(input.actions()), seconds);
    }

    fn draw(&self, frame: &mut Frame) {
        frame.set_camera(Camera::new_2d(VIEW_HEIGHT));
        let Some(textures) = &self.textures else {
            return;
        };
        draw_floor(frame, textures.floor);
        for ball in &self.balls {
            frame.draw_sprite(
                Sprite::new(
                    textures.ball,
                    ball.position,
                    Vec2::new(ball.size, ball.size),
                )
                .with_color(ball.color)
                .with_layer(BALL_LAYER),
            );
        }
        frame.draw_sprite(self.hero.sprite(textures.hero));
    }
}

impl Ball {
    fn random(random: &mut Random) -> Self {
        let angle = random.next() * TAU;
        let speed = random.next() * BALL_MAX_SPEED;
        Self {
            position: Vec2::new(
                random.between(-ARENA.x, ARENA.x),
                random.between(-ARENA.y, ARENA.y),
            ),
            velocity: Vec2::new(angle.cos(), angle.sin()) * speed,
            size: BALL_MIN_SIZE * (1.0 + random.next()),
            color: Color::rgba(random.next(), random.next(), random.next(), BALL_OPACITY),
        }
    }

    fn advance(&mut self, seconds: f32) {
        self.position = self.position + self.velocity * seconds;
        if self.position.x.abs() > ARENA.x {
            self.velocity.x = -self.velocity.x;
            self.position.x = self.position.x.clamp(-ARENA.x, ARENA.x);
        }
        if self.position.y.abs() > ARENA.y {
            self.velocity.y = -self.velocity.y;
            self.position.y = self.position.y.clamp(-ARENA.y, ARENA.y);
        }
    }
}

impl Hero {
    fn walk(&mut self, direction: Vec2, seconds: f32) {
        if direction == Vec2::ZERO {
            self.animation = 0.0;
            return;
        }
        if direction.x != 0.0 {
            self.facing_left = direction.x < 0.0;
        }
        self.animation =
            (self.animation + seconds * HERO_FRAMES_PER_SECOND) % f32::from(HERO_FRAMES);
        let step = direction.normalized().unwrap_or(Vec2::ZERO) * (HERO_SPEED * seconds);
        let position = self.position + step;
        self.position = Vec2::new(
            position.x.clamp(-ARENA.x, ARENA.x),
            position.y.clamp(-ARENA.y, ARENA.y),
        );
    }

    fn sprite(&self, sheet: TextureId) -> Sprite {
        // A negative width mirrors the hero to face left.
        let width = if self.facing_left {
            -HERO_SIZE
        } else {
            HERO_SIZE
        };
        let region =
            TextureRegion::cell(HERO_FRAMES, 1, self.frame(), 0).unwrap_or(TextureRegion::FULL);
        Sprite::new(sheet, self.position, Vec2::new(width, HERO_SIZE))
            .with_region(region)
            .with_layer(HERO_LAYER)
    }

    /// The frame of the sheet shown now: the first one while standing still.
    fn frame(&self) -> u16 {
        (0..HERO_FRAMES)
            .rev()
            .find(|&frame| f32::from(frame) <= self.animation)
            .unwrap_or(0)
    }
}

/// Where the player wants to go, from -1 to 1 in each axis.
fn direction(actions: &Actions) -> Vec2 {
    let axis = |negative, positive| {
        f32::from(i8::from(actions.is_down(positive)) - i8::from(actions.is_down(negative)))
    };
    Vec2::new(axis(LEFT, RIGHT), axis(DOWN, UP))
}

/// Every tile shares the texture, so the whole floor is a single batch.
fn draw_floor(frame: &mut Frame, texture: TextureId) {
    for row in -FLOOR_TILES..=FLOOR_TILES {
        for column in -FLOOR_TILES..=FLOOR_TILES {
            let position = Vec2::new(f32::from(column), f32::from(row));
            frame.draw_sprite(Sprite::new(texture, position, Vec2::ONE).with_layer(FLOOR_LAYER));
        }
    }
}

/// 2 x 2 checkerboard.
fn floor_image() -> EngineResult<Image> {
    let pixels = [FLOOR_LIGHT, FLOOR_DARK, FLOOR_DARK, FLOOR_LIGHT];
    Image::from_rgba8(2, 2, pixels.as_flattened().to_vec())
}

/// A white disc with soft edges, to be tinted.
fn ball_image() -> EngineResult<Image> {
    let radius = f32::from(BALL_PIXELS) / 2.0;
    let mut pixels = Vec::new();
    for row in 0..BALL_PIXELS {
        for column in 0..BALL_PIXELS {
            let offset = Vec2::new(f32::from(column) + 0.5, f32::from(row) + 0.5)
                - Vec2::new(radius, radius);
            // Opaque inside, fading to transparent over the last pixel.
            let opacity = (radius - offset.length()).clamp(0.0, 1.0);
            pixels.extend_from_slice(&[255, 255, 255, unit_to_byte(opacity)]);
        }
    }
    let side = u32::from(BALL_PIXELS);
    Image::from_rgba8(side, side, pixels)
}

/// Frames of a little square creature whose eye looks up and down as it walks.
fn hero_sheet() -> EngineResult<Image> {
    let eye_rows = [2, 3, 2, 1];
    let width = HERO_FRAMES * HERO_FRAME_PIXELS;
    let mut pixels = Vec::new();
    for row in 0..HERO_FRAME_PIXELS {
        for column in 0..width {
            let frame = usize::from(column / HERO_FRAME_PIXELS);
            let column_in_frame = column % HERO_FRAME_PIXELS;
            let is_eye = (5..=6).contains(&column_in_frame) && row == eye_rows[frame];
            pixels.extend_from_slice(if is_eye { &HERO_EYE } else { &HERO_BODY });
        }
    }
    Image::from_rgba8(u32::from(width), u32::from(HERO_FRAME_PIXELS), pixels)
}

/// `0.0..=1.0` to `0..=255`.
// The value is clamped first, so the conversion neither truncates nor loses a sign.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn unit_to_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * f32::from(u8::MAX)).round() as u8
}

/// Small pseudo-random generator, so the example needs no dependencies.
struct Random(u32);

impl Random {
    /// Constants of the classic linear congruential generator from "Numerical Recipes".
    const MULTIPLIER: u32 = 1_664_525;
    const INCREMENT: u32 = 1_013_904_223;
    /// Bits of 1.0 as an `f32`; the random bits fill its fraction.
    const ONE_BITS: u32 = 0x3F80_0000;
    /// Bits of the fraction of an `f32`.
    const FRACTION_BITS: u32 = 23;

    /// A number from 0 (included) to 1 (excluded).
    fn next(&mut self) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::INCREMENT);
        // 1.0 to 2.0, from the highest random bits, minus 1.
        f32::from_bits(Self::ONE_BITS | (self.0 >> (32 - Self::FRACTION_BITS))) - 1.0
    }

    fn between(&mut self, low: f32, high: f32) -> f32 {
        low + (high - low) * self.next()
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
        LEFT,
        &[
            Key::ArrowLeft.into(),
            Key::A.into(),
            GamepadButton::DPadLeft.into(),
        ],
    )?;
    actions.add(
        RIGHT,
        &[
            Key::ArrowRight.into(),
            Key::D.into(),
            GamepadButton::DPadRight.into(),
        ],
    )?;
    actions.add(
        UP,
        &[
            Key::ArrowUp.into(),
            Key::W.into(),
            GamepadButton::DPadUp.into(),
        ],
    )?;
    actions.add(
        DOWN,
        &[
            Key::ArrowDown.into(),
            Key::S.into(),
            GamepadButton::DPadDown.into(),
        ],
    )?;

    App::new()
        .with_title(format!("Sprites 2D — {backend}"))
        .with_actions(actions)
        .with_graphics_backend(backend)
        .with_clear_color(BACKGROUND)
        .run_game(Sprites2d::new())?;
    Ok(())
}
