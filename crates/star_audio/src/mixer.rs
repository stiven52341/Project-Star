//! The voices playing at once, added together into the output's samples.

use std::sync::Arc;

use star_core::{EngineError, EngineResult, Sound};

use crate::OutputFormat;
use crate::voice::{PlaySettings, Voice, sanitized_volume};

/// Most sounds playing at once. The voices are reserved up front, so
/// starting one never allocates while the output is being filled.
pub const MAX_VOICES: usize = 128;

/// Most groups a mixer keeps: music, effects, voices… a game has a few.
pub const MAX_GROUPS: usize = 32;

/// A group of sounds whose volume is changed together (decision 87):
/// music, effects… made by [`Mixer::group`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupId(usize);

/// Names a sound started by [`Mixer::play`], to change or stop it later.
/// Ids are never reused, so an old id never names a newer sound.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VoiceId(u64);

impl VoiceId {
    /// The id as a number, for code that cannot hold engine types (the
    /// support code of StarScript).
    #[must_use]
    pub const fn to_bits(self) -> u64 {
        self.0
    }

    /// The id a number from [`VoiceId::to_bits`] stands for. A number that
    /// never came from it names no sound: it is never playing.
    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }
}

/// Plays many sounds at once: each is read at the output's rate and
/// channels, scaled by its volume and the master volume, and added up.
#[derive(Debug)]
pub struct Mixer {
    format: OutputFormat,
    voices: Vec<(VoiceId, Voice)>,
    next_id: u64,
    volume: f32,
    /// Each group's name and volume, by its id.
    groups: Vec<(String, f32)>,
}

impl Mixer {
    /// A silent mixer for an output of `format`, at full master volume.
    #[must_use]
    pub fn new(format: OutputFormat) -> Self {
        Self {
            format,
            voices: Vec::with_capacity(MAX_VOICES),
            next_id: 0,
            volume: 1.0,
            groups: Vec::with_capacity(MAX_GROUPS),
        }
    }

    /// The output the mixer writes for.
    #[must_use]
    pub const fn format(&self) -> OutputFormat {
        self.format
    }

    /// Starts playing `sound` from its beginning.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::CapacityExceeded`] if [`MAX_VOICES`] sounds
    /// are already playing.
    pub fn play(&mut self, sound: Arc<Sound>, settings: PlaySettings) -> EngineResult<VoiceId> {
        if self.voices.len() >= MAX_VOICES {
            return Err(EngineError::CapacityExceeded {
                resource: "audio voices",
            });
        }
        let id = self.unused_id();
        self.voices
            .push((id, Voice::new(sound, settings, self.format)));
        Ok(id)
    }

    /// An id no sound has had, for a sound that is not played.
    pub(crate) const fn unused_id(&mut self) -> VoiceId {
        let id = VoiceId(self.next_id);
        self.next_id += 1;
        id
    }

    /// Stops the sound `id`; nothing happens if it already ended.
    pub fn stop(&mut self, id: VoiceId) {
        self.voices.retain(|(voice_id, _)| *voice_id != id);
    }

    /// Stops every sound.
    pub fn stop_all(&mut self) {
        self.voices.clear();
    }

    /// Whether the sound `id` is still playing.
    #[must_use]
    pub fn is_playing(&self, id: VoiceId) -> bool {
        self.voices.iter().any(|(voice_id, _)| *voice_id == id)
    }

    /// Number of sounds playing.
    #[must_use]
    pub fn playing_count(&self) -> usize {
        self.voices.len()
    }

    /// Changes the volume of the sound `id` (see [`PlaySettings::volume`]);
    /// nothing happens if it already ended.
    pub fn set_voice_volume(&mut self, id: VoiceId, volume: f32) {
        if let Some((_, voice)) = self.voices.iter_mut().find(|(voice_id, _)| *voice_id == id) {
            voice.set_volume(volume);
        }
    }

    /// The volume every sound is scaled by.
    #[must_use]
    pub const fn volume(&self) -> f32 {
        self.volume
    }

    /// Changes the volume every sound is scaled by (see [`PlaySettings::volume`]).
    pub fn set_volume(&mut self, volume: f32) {
        self.volume = sanitized_volume(volume);
    }

    /// The group called `name`, made at full volume the first time.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::CapacityExceeded`] if [`MAX_GROUPS`] groups
    /// already exist.
    pub fn group(&mut self, name: &str) -> EngineResult<GroupId> {
        if let Some(index) = self.groups.iter().position(|(group, _)| group == name) {
            return Ok(GroupId(index));
        }
        if self.groups.len() >= MAX_GROUPS {
            return Err(EngineError::CapacityExceeded {
                resource: "audio groups",
            });
        }
        self.groups.push((name.to_owned(), 1.0));
        Ok(GroupId(self.groups.len() - 1))
    }

    /// The volume of the group called `name`: 1 for a group never made.
    #[must_use]
    pub fn group_volume(&self, name: &str) -> f32 {
        self.groups
            .iter()
            .find(|(group, _)| group == name)
            .map_or(1.0, |(_, volume)| *volume)
    }

    /// Changes the volume of the group called `name`, which scales the
    /// sounds played in it (see [`PlaySettings::volume`]); the group is
    /// made if it does not exist yet.
    ///
    /// # Errors
    ///
    /// As [`Mixer::group`].
    pub fn set_group_volume(&mut self, name: &str, volume: f32) -> EngineResult<()> {
        let GroupId(index) = self.group(name)?;
        if let Some((_, group_volume)) = self.groups.get_mut(index) {
            *group_volume = sanitized_volume(volume);
        }
        Ok(())
    }

    /// Fills `output`, interleaved by the format's channels, with the next
    /// samples of every sound; sounds that end are forgotten. Samples are
    /// kept from -1 to 1, so loud mixes clip instead of overflowing.
    pub fn mix(&mut self, output: &mut [f32]) {
        output.fill(0.0);
        let channels = self.format.channels();
        let volume = self.volume;
        let groups = &self.groups;
        self.voices.retain_mut(|(_, voice)| {
            let group = voice
                .group()
                .and_then(|GroupId(index)| groups.get(index))
                .map_or(1.0, |(_, group_volume)| *group_volume);
            voice.mix_into(output, channels, volume * group)
        });
        for sample in output.iter_mut() {
            *sample = sample.clamp(-1.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 4;

    fn mixer() -> Mixer {
        Mixer::new(OutputFormat::new(1, RATE).unwrap())
    }

    fn sound(samples: Vec<f32>) -> Arc<Sound> {
        Arc::new(Sound::from_interleaved(1, RATE, samples).unwrap())
    }

    fn mixed(mixer: &mut Mixer, frames: usize) -> Vec<f32> {
        let mut output = vec![1.0; frames];
        mixer.mix(&mut output);
        output
    }

    #[test]
    fn a_silent_mixer_writes_silence() {
        assert_eq!(mixed(&mut mixer(), 2), [0.0, 0.0]);
    }

    #[test]
    fn sounds_playing_at_once_are_added() {
        let mut mixer = mixer();
        mixer
            .play(sound(vec![0.25, 0.25]), PlaySettings::default())
            .unwrap();
        mixer
            .play(sound(vec![0.5]), PlaySettings::default())
            .unwrap();

        assert_eq!(mixed(&mut mixer, 2), [0.75, 0.25]);
    }

    #[test]
    fn ended_sounds_are_forgotten() {
        let mut mixer = mixer();
        let id = mixer
            .play(sound(vec![0.5]), PlaySettings::default())
            .unwrap();

        mixed(&mut mixer, 2);

        assert!(!mixer.is_playing(id));
        assert_eq!(mixer.playing_count(), 0);
    }

    #[test]
    fn stopped_sounds_are_silent() {
        let mut mixer = mixer();
        let id = mixer
            .play(sound(vec![0.5; 4]), PlaySettings::default())
            .unwrap();
        let other = mixer
            .play(sound(vec![0.25; 4]), PlaySettings::default())
            .unwrap();

        mixer.stop(id);

        assert_eq!(mixed(&mut mixer, 1), [0.25]);
        assert!(mixer.is_playing(other));

        mixer.stop_all();

        assert_eq!(mixed(&mut mixer, 1), [0.0]);
    }

    #[test]
    fn volumes_scale_their_sound_and_every_sound() {
        let mut mixer = mixer();
        let id = mixer
            .play(sound(vec![0.8; 2]), PlaySettings::default())
            .unwrap();

        mixer.set_voice_volume(id, 0.5);
        mixer.set_volume(0.5);

        assert_eq!(mixed(&mut mixer, 1), [0.2]);
        assert!((mixer.volume() - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn groups_scale_the_sounds_played_in_them() {
        let mut mixer = mixer();
        let music = mixer.group("music").unwrap();
        mixer.set_group_volume("music", 0.5).unwrap();
        let settings = PlaySettings {
            group: Some(music),
            ..PlaySettings::default()
        };
        mixer.play(sound(vec![0.4; 4]), settings).unwrap();
        mixer
            .play(sound(vec![0.1; 4]), PlaySettings::default())
            .unwrap();

        let out = mixed(&mut mixer, 1);

        assert!((out[0] - 0.3).abs() < 1e-6, "{out:?}");
        assert_eq!(mixer.group("music").unwrap(), music, "one group per name");
        assert!((mixer.group_volume("music") - 0.5).abs() < f32::EPSILON);
        assert!((mixer.group_volume("voices") - 1.0).abs() < f32::EPSILON);
    }

    #[test]
    fn a_mixer_keeps_a_few_groups() {
        let mut mixer = mixer();
        for index in 0..MAX_GROUPS {
            mixer.group(&index.to_string()).unwrap();
        }

        assert!(mixer.group("one too many").is_err());
    }

    #[test]
    fn loud_mixes_are_clipped() {
        let mut mixer = mixer();
        for _ in 0..3 {
            mixer
                .play(sound(vec![0.5, -0.5]), PlaySettings::default())
                .unwrap();
        }

        assert_eq!(mixed(&mut mixer, 2), [1.0, -1.0]);
    }

    #[test]
    fn ids_survive_as_numbers() {
        let mut mixer = mixer();
        let id = mixer
            .play(sound(vec![0.5; 2]), PlaySettings::default())
            .unwrap();

        assert!(mixer.is_playing(VoiceId::from_bits(id.to_bits())));
    }

    #[test]
    fn ids_are_never_reused() {
        let mut mixer = mixer();
        let first = mixer
            .play(sound(vec![0.5]), PlaySettings::default())
            .unwrap();
        mixer.stop(first);

        let second = mixer
            .play(sound(vec![0.5]), PlaySettings::default())
            .unwrap();

        assert_ne!(first, second);
        assert!(!mixer.is_playing(first));
    }

    #[test]
    fn too_many_sounds_are_refused() {
        let mut mixer = mixer();
        let looping = PlaySettings {
            looping: true,
            ..PlaySettings::default()
        };
        for _ in 0..MAX_VOICES {
            mixer.play(sound(vec![0.0]), looping).unwrap();
        }

        let error = mixer.play(sound(vec![0.0]), looping).unwrap_err();

        assert!(matches!(error, EngineError::CapacityExceeded { .. }));
    }
}
