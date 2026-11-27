//! The tab of a sound file (decision 87): its waveform, how long it is,
//! its channels and rate, and buttons to hear it.

mod waveform;

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use eframe::egui;
use star_engine::star_assets::{AssetFolder, AssetPath};
use star_engine::star_audio::{Audio, PlaySettings, VoiceId};
use star_engine::star_core::Sound;

use self::waveform::{WAVEFORM_HEIGHT, Waveform};
use crate::editor_audio::EditorAudio;
use crate::style::ERROR_COLOR;
use crate::texts;

/// A sound open in a tab: read once, never changed.
pub(crate) struct SoundTab {
    path: String,
    sound: Result<(Arc<Sound>, Waveform), String>,
    playing: Option<Playing>,
}

/// A sound the editor plays, and since when.
struct Playing {
    audio: Audio,
    voice: VoiceId,
    since: Instant,
}

impl Drop for SoundTab {
    fn drop(&mut self) {
        self.stop();
    }
}

impl SoundTab {
    /// Opens the sound at `path` inside the `assets` folder.
    pub(crate) fn open(assets: &Path, path: String) -> Self {
        let sound = read_sound(assets, &path).map(|sound| {
            let waveform = Waveform::of(&sound);
            (Arc::new(sound), waveform)
        });
        Self {
            path,
            sound,
            playing: None,
        }
    }

    /// Where it is inside the assets folder.
    pub(crate) fn path(&self) -> &str {
        &self.path
    }

    /// The name of its file, for its tab.
    pub(crate) fn title(&self) -> String {
        self.path
            .rsplit('/')
            .next()
            .unwrap_or(&self.path)
            .to_owned()
    }

    /// The middle of the tab: what the sound is, its buttons and its
    /// waveform, with a line where it plays.
    pub(crate) fn show(&mut self, ui: &mut egui::Ui, audio: &mut EditorAudio) {
        let texts = texts::current();
        let sound = match &self.sound {
            Ok((sound, _)) => Arc::clone(sound),
            Err(error) => {
                ui.colored_label(ERROR_COLOR, texts.sound_unreadable(error));
                return;
            }
        };
        if self
            .playing
            .as_ref()
            .is_some_and(|playing| !playing.audio.is_playing(playing.voice))
        {
            self.playing = None;
        }
        ui.horizontal(|ui| {
            if self.playing.is_some() {
                if ui.button(texts.stop_sound()).clicked() {
                    self.stop();
                }
            } else if ui.button(texts.play_sound()).clicked() {
                self.play(&sound, audio);
            }
            ui.label(texts.sound_details(
                sound.duration_seconds(),
                sound.channels(),
                sound.sample_rate(),
            ));
        });
        ui.separator();
        let played = self.playing.as_ref().map(|playing| {
            // The length of a sound is far within `f32`.
            #[allow(clippy::cast_possible_truncation)]
            let length = sound.duration_seconds() as f32;
            playing.since.elapsed().as_secs_f32() / length.max(f32::EPSILON)
        });
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), WAVEFORM_HEIGHT),
            egui::Sense::hover(),
        );
        if let Ok((_, waveform)) = &self.sound {
            waveform.draw(ui.painter(), rect, played);
        }
        if self.playing.is_some() {
            ui.ctx().request_repaint();
        }
    }

    fn play(&mut self, sound: &Arc<Sound>, audio: &mut EditorAudio) {
        let Some(audio) = audio.audio() else {
            return;
        };
        match audio.play(Arc::clone(sound), PlaySettings::default()) {
            Ok(voice) => {
                self.playing = Some(Playing {
                    audio,
                    voice,
                    since: Instant::now(),
                });
            }
            Err(error) => log::warn!("Cannot play \"{}\": {error}", self.path),
        }
    }

    fn stop(&mut self) {
        if let Some(playing) = self.playing.take() {
            playing.audio.stop(playing.voice);
        }
    }
}

/// The sound at `path` inside the `assets` folder, decoded.
pub(crate) fn read_sound(assets: &Path, path: &str) -> Result<Sound, String> {
    AssetPath::new(path)
        .and_then(|path| AssetFolder::new(assets).load_sound(&path))
        .map_err(|error| error.to_string())
}
