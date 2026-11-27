//! StarScript's `Sound`, played by the engine.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use star_engine::star_assets::{AssetFolder, AssetPath};
use star_engine::star_audio::{self, Audio, PlaySettings};
use star_engine::star_core::Sound;
use star_script_runtime::audio::{AudioBackend, Playing, SoundId, VoiceId};

/// Loads the sounds of a game's asset folder and plays them on an output.
pub struct EngineAudio {
    folder: AssetFolder,
    output: Rc<RefCell<Audio>>,
    /// Every sound loaded, by its number.
    sounds: Vec<Arc<Sound>>,
}

impl EngineAudio {
    /// Sounds of `folder`, played on `output`, which the game can replace
    /// (e.g. a silent one until the engine's is ready).
    #[must_use]
    pub fn new(folder: AssetFolder, output: Rc<RefCell<Audio>>) -> Self {
        Self {
            folder,
            output,
            sounds: Vec::new(),
        }
    }
}

impl AudioBackend for EngineAudio {
    fn load(&mut self, path: &str) -> Result<SoundId, String> {
        let sound = AssetPath::new(path)
            .and_then(|path| self.folder.load_sound(&path))
            .map_err(|error| error.to_string())?;
        self.sounds.push(Arc::new(sound));
        Ok(SoundId(self.sounds.len() - 1))
    }

    fn play(&mut self, sound: SoundId, how: Playing<'_>) -> Result<VoiceId, String> {
        let sound = self
            .sounds
            .get(sound.0)
            .ok_or_else(|| String::from("the sound was never loaded"))?;
        let output = self.output.borrow();
        let group = (!how.group.is_empty())
            .then(|| output.group(how.group))
            .transpose()
            .map_err(|error| error.to_string())?;
        let settings = PlaySettings {
            volume: how.volume,
            looping: how.looping,
            group,
        };
        output
            .play(Arc::clone(sound), settings)
            .map(|voice| VoiceId(voice.to_bits()))
            .map_err(|error| error.to_string())
    }

    fn stop(&mut self, voice: VoiceId) {
        self.output
            .borrow()
            .stop(star_audio::VoiceId::from_bits(voice.0));
    }

    fn is_playing(&self, voice: VoiceId) -> bool {
        self.output
            .borrow()
            .is_playing(star_audio::VoiceId::from_bits(voice.0))
    }

    fn group_volume(&self, group: &str) -> f32 {
        self.output.borrow().group_volume(group)
    }

    fn set_group_volume(&mut self, group: &str, volume: f32) -> Result<(), String> {
        self.output
            .borrow()
            .set_group_volume(group, volume)
            .map_err(|error| error.to_string())
    }

    fn volume(&self) -> f32 {
        self.output.borrow().volume()
    }

    fn set_volume(&mut self, volume: f32) {
        self.output.borrow().set_volume(volume);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn silent_audio() -> EngineAudio {
        EngineAudio::new(
            AssetFolder::new(std::env::temp_dir()),
            Rc::new(RefCell::new(Audio::silent())),
        )
    }

    #[test]
    fn missing_sounds_are_explained() {
        let mut audio = silent_audio();

        assert!(audio.load("sounds/missing.wav").is_err());
    }

    #[test]
    fn sounds_never_loaded_are_not_played() {
        let mut audio = silent_audio();

        let how = Playing {
            volume: 1.0,
            looping: false,
            group: "music",
        };
        let played = audio.play(SoundId(0), how);

        assert_eq!(played, Err(String::from("the sound was never loaded")));
    }

    #[test]
    fn group_volumes_are_the_output_s() {
        let mut audio = silent_audio();

        audio.set_group_volume("music", 0.5).unwrap();

        assert!((audio.group_volume("music") - 0.5).abs() < f32::EPSILON);
        assert!((audio.output.borrow().group_volume("music") - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn the_volume_is_the_output_s() {
        let mut audio = silent_audio();

        audio.set_volume(0.25);

        assert!((audio.volume() - audio.output.borrow().volume()).abs() < f32::EPSILON);
    }
}
