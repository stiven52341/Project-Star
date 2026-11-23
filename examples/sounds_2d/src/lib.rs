//! Audio test: sounds loaded from the asset folder, played at once.
//!
//! Space (or a gamepad's bottom button, a click or a tap) plays a short beep,
//! from a WAV file; every press starts another one, so fast presses
//! overlap. M (or the gamepad's right button) starts and stops a melody
//! that loops, from an OGG Vorbis file. The arrows up and down change the
//! master volume, shown by the width of the bar. Run it with
//! `cargo run -p sounds_2d`, or on Android
//! with `cargo apk2 run --lib --release` from its folder.

use std::error::Error;
use std::sync::Arc;

use star_engine::prelude::*;

const BACKGROUND: Color = Color::rgb(0.08, 0.06, 0.12);
/// World units visible from the bottom to the top of the window.
const VIEW_HEIGHT: f32 = 10.0;

/// The example's assets, found wherever it is run from.
const ASSET_FOLDER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets");
const BEEP_SOUND: &str = "beep.wav";
const MUSIC_SOUND: &str = "music.ogg";

/// Volume change per press of up or down.
const VOLUME_STEP: f32 = 0.1;
const MAX_VOLUME: f32 = 1.0;
const MUSIC_VOLUME: f32 = 0.6;

/// Width of the bar at full volume, in world units.
const BAR_WIDTH: f32 = 8.0;
const BAR_HEIGHT: f32 = 0.6;
const BAR_COLOR: Color = Color::rgb(0.4, 0.8, 0.5);
/// How much the bar grows while the melody plays.
const PLAYING_HEIGHT: f32 = 1.5;
const WHITE_PIXEL: [u8; 4] = [255, 255, 255, 255];

const BEEP: &str = "beep";
const MUSIC: &str = "music";
const LOUDER: &str = "louder";
const QUIETER: &str = "quieter";

/// What the example loaded when it started.
struct Loaded {
    beep: Arc<Sound>,
    music: Arc<Sound>,
    bar: TextureId,
}

#[derive(Default)]
struct SoundsGame {
    loaded: Option<Loaded>,
    melody: Option<VoiceId>,
    /// What the bar shows: the master volume and whether the melody plays.
    volume: f32,
    is_melody_playing: bool,
}

impl SoundsGame {
    fn toggle_melody(&mut self, audio: &Audio, music: &Arc<Sound>) {
        if let Some(melody) = self.melody.take() {
            audio.stop(melody);
            return;
        }
        let settings = PlaySettings {
            volume: MUSIC_VOLUME,
            looping: true,
        };
        match audio.play(Arc::clone(music), settings) {
            Ok(melody) => self.melody = Some(melody),
            Err(error) => log::warn!("{error}"),
        }
    }
}

impl Game for SoundsGame {
    fn start(&mut self, context: &mut GameContext<'_>) -> EngineResult<()> {
        let white = Image::from_rgba8(1, 1, WHITE_PIXEL.to_vec())?;
        self.loaded = Some(Loaded {
            beep: context.load_sound(BEEP_SOUND)?,
            music: context.load_sound(MUSIC_SOUND)?,
            bar: context.create_texture(&white, TextureFilter::Pixelated)?,
        });
        if !context.world().audio().has_output() {
            log::warn!("There is no audio device: nothing will be heard");
        }
        Ok(())
    }

    fn update(&mut self, world: &mut World) {
        let Some(loaded) = &self.loaded else {
            return;
        };
        let (beep, music) = (Arc::clone(&loaded.beep), Arc::clone(&loaded.music));
        let audio = world.audio();
        let Some(input) = world.input() else {
            return;
        };
        let actions = input.actions();
        if actions.just_pressed(BEEP)
            && let Err(error) = audio.play(beep, PlaySettings::default())
        {
            log::warn!("{error}");
        }
        if actions.just_pressed(MUSIC) {
            self.toggle_melody(&audio, &music);
        }
        let mut volume = audio.volume();
        if actions.just_pressed(LOUDER) {
            volume += VOLUME_STEP;
        }
        if actions.just_pressed(QUIETER) {
            volume -= VOLUME_STEP;
        }
        audio.set_volume(volume.clamp(0.0, MAX_VOLUME));
        self.volume = audio.volume();
        self.is_melody_playing = self.melody.is_some_and(|melody| audio.is_playing(melody));
    }

    fn draw(&self, _world: &World, frame: &mut Frame) {
        frame.set_camera(Camera::new_2d(VIEW_HEIGHT));
        let Some(loaded) = &self.loaded else {
            return;
        };
        let height = if self.is_melody_playing {
            BAR_HEIGHT * PLAYING_HEIGHT
        } else {
            BAR_HEIGHT
        };
        let size = Vec2::new(BAR_WIDTH * self.volume, height);
        frame.draw_sprite(Sprite::new(loaded.bar, Vec2::ZERO, size).with_color(BAR_COLOR));
    }
}

/// Runs the example until its window is closed (on phones it fills the
/// screen).
///
/// # Errors
///
/// Returns an error if the actions are invalid or the engine cannot run.
pub fn run() -> Result<(), Box<dyn Error>> {
    let mut actions = ActionMap::new();
    actions.add(
        BEEP,
        &[
            Key::Space.into(),
            MouseButton::Left.into(),
            Binding::Touch,
            GamepadButton::South.into(),
        ],
    )?;
    actions.add(MUSIC, &[Key::M.into(), GamepadButton::East.into()])?;
    actions.add(LOUDER, &[Key::ArrowUp.into(), GamepadButton::DPadUp.into()])?;
    actions.add(
        QUIETER,
        &[Key::ArrowDown.into(), GamepadButton::DPadDown.into()],
    )?;

    App::new()
        .with_title("Sounds 2D")
        .with_actions(actions)
        .with_development_assets(ASSET_FOLDER)
        .with_clear_color(BACKGROUND)
        .run_game(SoundsGame::default())?;
    Ok(())
}

star_engine::android_main!(run);
