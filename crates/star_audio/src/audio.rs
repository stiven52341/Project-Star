//! What games use to play sounds: a handle to the mixer of the output.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use star_core::{EngineResult, Sound};

use crate::{GroupId, Mixer, OutputFormat, PlaySettings, VoiceId};

/// Plays sounds on the game's audio output. Copies share the same output,
/// so systems and the game can each keep one.
///
/// Without an audio device (see [`Audio::silent`]) every call still works:
/// sounds are accepted but never heard, and never count as playing.
#[derive(Debug, Clone)]
pub struct Audio {
    mixer: Arc<Mutex<Mixer>>,
    has_output: bool,
}

impl Audio {
    /// The handle of an output that reads `mixer`.
    pub(crate) const fn with_output(mixer: Arc<Mutex<Mixer>>) -> Self {
        Self {
            mixer,
            has_output: true,
        }
    }

    /// Audio that plays nothing: for machines without an audio device, and
    /// for tests. Games run the same, only silent.
    #[must_use]
    pub fn silent() -> Self {
        Self {
            // Never heard, so any format will do.
            mixer: Arc::new(Mutex::new(Mixer::new(OutputFormat::STEREO_48K))),
            has_output: false,
        }
    }

    /// Whether sounds are heard: `false` without an audio device.
    #[must_use]
    pub const fn has_output(&self) -> bool {
        self.has_output
    }

    /// The channels and sample rate of the output.
    #[must_use]
    pub fn format(&self) -> OutputFormat {
        self.mixer().format()
    }

    /// Starts playing `sound` from its beginning (see [`Mixer::play`]).
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::CapacityExceeded`](star_core::EngineError::CapacityExceeded)
    /// if [`MAX_VOICES`](crate::MAX_VOICES) sounds are already playing.
    pub fn play(&self, sound: Arc<Sound>, settings: PlaySettings) -> EngineResult<VoiceId> {
        let mut mixer = self.mixer();
        if self.has_output {
            mixer.play(sound, settings)
        } else {
            Ok(mixer.unused_id())
        }
    }

    /// Stops the sound `id`; nothing happens if it already ended.
    pub fn stop(&self, id: VoiceId) {
        self.mixer().stop(id);
    }

    /// Stops every sound.
    pub fn stop_all(&self) {
        self.mixer().stop_all();
    }

    /// Whether the sound `id` is still playing.
    #[must_use]
    pub fn is_playing(&self, id: VoiceId) -> bool {
        self.mixer().is_playing(id)
    }

    /// Changes the volume of the sound `id`; nothing happens if it ended.
    pub fn set_voice_volume(&self, id: VoiceId, volume: f32) {
        self.mixer().set_voice_volume(id, volume);
    }

    /// Moves the sound `id` to the left or the right (see
    /// [`Mixer::set_voice_pan`]).
    pub fn set_voice_pan(&self, id: VoiceId, pan: f32) {
        self.mixer().set_voice_pan(id, pan);
    }

    /// The group called `name` (see [`Mixer::group`]).
    ///
    /// # Errors
    ///
    /// As [`Mixer::group`].
    pub fn group(&self, name: &str) -> EngineResult<GroupId> {
        self.mixer().group(name)
    }

    /// The volume of the group called `name` (see [`Mixer::group_volume`]).
    #[must_use]
    pub fn group_volume(&self, name: &str) -> f32 {
        self.mixer().group_volume(name)
    }

    /// Changes the volume of the group called `name` (see
    /// [`Mixer::set_group_volume`]).
    ///
    /// # Errors
    ///
    /// As [`Mixer::group`].
    pub fn set_group_volume(&self, name: &str, volume: f32) -> EngineResult<()> {
        self.mixer().set_group_volume(name, volume)
    }

    /// The volume every sound is scaled by.
    #[must_use]
    pub fn volume(&self) -> f32 {
        self.mixer().volume()
    }

    /// Changes the volume every sound is scaled by.
    pub fn set_volume(&self, volume: f32) {
        self.mixer().set_volume(volume);
    }

    fn mixer(&self) -> MutexGuard<'_, Mixer> {
        lock(&self.mixer)
    }
}

/// Locks `mixer`. A thread that stopped while holding it left it whole
/// (every change is a single step), so its lock is taken back.
pub(crate) fn lock(mixer: &Mutex<Mixer>) -> MutexGuard<'_, Mixer> {
    mixer.lock().unwrap_or_else(PoisonError::into_inner)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sound() -> Arc<Sound> {
        Arc::new(
            Sound::from_interleaved(1, OutputFormat::STEREO_48K.sample_rate(), vec![0.5; 4])
                .unwrap(),
        )
    }

    #[test]
    fn silent_audio_accepts_sounds_but_plays_none() {
        let audio = Audio::silent();

        let first = audio.play(sound(), PlaySettings::default()).unwrap();
        let second = audio.play(sound(), PlaySettings::default()).unwrap();

        assert!(!audio.has_output());
        assert_ne!(first, second);
        assert!(!audio.is_playing(first));
    }

    #[test]
    fn copies_share_the_output() {
        let mixer = Arc::new(Mutex::new(Mixer::new(
            OutputFormat::new(1, OutputFormat::STEREO_48K.sample_rate()).unwrap(),
        )));
        let audio = Audio::with_output(Arc::clone(&mixer));
        let copy = audio.clone();

        let id = audio.play(sound(), PlaySettings::default()).unwrap();
        copy.set_volume(0.5);

        assert!(copy.is_playing(id));
        assert!((audio.volume() - 0.5).abs() < f32::EPSILON);

        copy.stop(id);

        assert!(!audio.is_playing(id));
        assert_eq!(lock(&mixer).playing_count(), 0);
    }
}
