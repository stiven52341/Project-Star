//! One sound being played: where it is, how loud and whether it loops.

use std::sync::Arc;

use star_core::Sound;

use crate::OutputFormat;

/// How a sound is played.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaySettings {
    /// Loudness: 1 as recorded, 0 silent, above 1 louder. Negative or
    /// not-a-number volumes are silent.
    pub volume: f32,
    /// Whether the sound starts again when it ends (music, engines…).
    pub looping: bool,
}

impl Default for PlaySettings {
    fn default() -> Self {
        Self {
            volume: 1.0,
            looping: false,
        }
    }
}

/// A volume the mixer can use: a finite number, at least 0.
pub(crate) fn sanitized_volume(volume: f32) -> f32 {
    if volume.is_finite() && volume > 0.0 {
        volume
    } else {
        0.0
    }
}

/// A sound being played, read at the output's sample rate.
#[derive(Debug)]
pub(crate) struct Voice {
    sound: Arc<Sound>,
    /// Where it is, in frames of the sound (with a fraction: the output's
    /// frames fall between the sound's when their rates differ).
    position: f64,
    /// Frames of the sound per frame of the output.
    step: f64,
    volume: f32,
    looping: bool,
}

impl Voice {
    pub(crate) fn new(sound: Arc<Sound>, settings: PlaySettings, format: OutputFormat) -> Self {
        let step = f64::from(sound.sample_rate()) / f64::from(format.sample_rate());
        Self {
            sound,
            position: 0.0,
            step,
            volume: sanitized_volume(settings.volume),
            looping: settings.looping,
        }
    }

    pub(crate) fn set_volume(&mut self, volume: f32) {
        self.volume = sanitized_volume(volume);
    }

    /// Adds this voice to the interleaved `output`, scaled by `master`
    /// volume; returns whether the voice still has something to play.
    pub(crate) fn mix_into(&mut self, output: &mut [f32], channels: u16, master: f32) -> bool {
        let gain = self.volume * master;
        for frame in output.chunks_exact_mut(usize::from(channels)) {
            if !self.is_playing() {
                return false;
            }
            for (channel, sample) in (0..channels).zip(frame.iter_mut()) {
                *sample += self.sample(channel, channels) * gain;
            }
            self.advance();
        }
        self.is_playing()
    }

    fn is_playing(&self) -> bool {
        self.position < self.frame_count()
    }

    /// The sound's frame count, as a position.
    // Sounds of more than 2^53 frames (thousands of years) lose precision.
    #[allow(clippy::cast_precision_loss)]
    fn frame_count(&self) -> f64 {
        self.sound.frames() as f64
    }

    fn advance(&mut self) {
        self.position += self.step;
        if self.looping && self.frame_count() > 0.0 {
            self.position %= self.frame_count();
        }
    }

    /// The sample of output channel `channel` (of `channels`) at the current
    /// position, between the two nearest frames of the sound.
    // The position is never negative and below the frame count, so it fits
    // in a `usize`; the fraction is between 0 and 1.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    fn sample(&self, channel: u16, channels: u16) -> f32 {
        let frame = self.position.floor();
        let fraction = (self.position - frame) as f32;
        let index = frame as usize;
        let next = if index + 1 < self.sound.frames() {
            index + 1
        } else if self.looping {
            0
        } else {
            index
        };
        let current = self.frame_sample(index, channel, channels);
        let following = self.frame_sample(next, channel, channels);
        current + (following - current) * fraction
    }

    /// The sample frame `frame` gives output channel `channel`: a mono sound
    /// goes to every channel, a mono output hears every channel at once,
    /// and otherwise each channel goes to its own (channels the output does
    /// not have are left out, channels the sound does not have are silent).
    fn frame_sample(&self, frame: usize, channel: u16, channels: u16) -> f32 {
        let sound_channels = usize::from(self.sound.channels());
        let start = frame * sound_channels;
        let Some(samples) = self.sound.samples().get(start..start + sound_channels) else {
            return 0.0;
        };
        if sound_channels == 1 {
            samples[0]
        } else if channels == 1 {
            samples.iter().sum::<f32>() / f32::from(self.sound.channels())
        } else {
            samples.get(usize::from(channel)).copied().unwrap_or(0.0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RATE: u32 = 4;

    fn sound(channels: u16, sample_rate: u32, samples: Vec<f32>) -> Arc<Sound> {
        Arc::new(Sound::from_interleaved(channels, sample_rate, samples).unwrap())
    }

    fn voice(sound: Arc<Sound>, settings: PlaySettings, channels: u16) -> Voice {
        Voice::new(sound, settings, OutputFormat::new(channels, RATE).unwrap())
    }

    fn mixed(voice: &mut Voice, frames: usize, channels: u16) -> (Vec<f32>, bool) {
        let mut output = vec![0.0; frames * usize::from(channels)];
        let playing = voice.mix_into(&mut output, channels, 1.0);
        (output, playing)
    }

    #[test]
    fn sounds_are_added_frame_by_frame() {
        let mut voice = voice(
            sound(1, RATE, vec![0.1, 0.2, 0.3]),
            PlaySettings::default(),
            1,
        );

        let (output, playing) = mixed(&mut voice, 2, 1);

        assert_eq!(output, [0.1, 0.2]);
        assert!(playing);
    }

    #[test]
    fn sounds_end_and_leave_silence() {
        let mut voice = voice(sound(1, RATE, vec![0.5, 0.5]), PlaySettings::default(), 1);

        let (output, playing) = mixed(&mut voice, 4, 1);

        assert_eq!(output, [0.5, 0.5, 0.0, 0.0]);
        assert!(!playing);
    }

    #[test]
    fn looping_sounds_start_again() {
        let settings = PlaySettings {
            looping: true,
            ..PlaySettings::default()
        };
        let mut voice = voice(sound(1, RATE, vec![0.1, 0.2]), settings, 1);

        let (output, playing) = mixed(&mut voice, 5, 1);

        assert_eq!(output, [0.1, 0.2, 0.1, 0.2, 0.1]);
        assert!(playing);
    }

    #[test]
    fn slower_sounds_are_stretched_between_frames() {
        let mut voice = voice(
            sound(1, RATE / 2, vec![0.0, 1.0]),
            PlaySettings::default(),
            1,
        );

        let (output, _) = mixed(&mut voice, 4, 1);

        assert_eq!(output, [0.0, 0.5, 1.0, 1.0]);
    }

    #[test]
    fn faster_sounds_skip_frames() {
        let mut voice = voice(
            sound(1, RATE * 2, vec![0.1, 0.2, 0.3, 0.4]),
            PlaySettings::default(),
            1,
        );

        let (output, playing) = mixed(&mut voice, 3, 1);

        assert_eq!(output, [0.1, 0.3, 0.0]);
        assert!(!playing);
    }

    #[test]
    fn mono_sounds_go_to_every_channel() {
        let mut voice = voice(sound(1, RATE, vec![0.5]), PlaySettings::default(), 2);

        let (output, _) = mixed(&mut voice, 1, 2);

        assert_eq!(output, [0.5, 0.5]);
    }

    #[test]
    fn mono_outputs_hear_every_channel() {
        let mut voice = voice(sound(2, RATE, vec![0.2, 0.6]), PlaySettings::default(), 1);

        let (output, _) = mixed(&mut voice, 1, 1);

        assert!((output[0] - 0.4).abs() < f32::EPSILON);
    }

    #[test]
    fn stereo_channels_keep_their_side() {
        let mut voice = voice(sound(2, RATE, vec![0.2, 0.6]), PlaySettings::default(), 2);

        let (output, _) = mixed(&mut voice, 1, 2);

        assert_eq!(output, [0.2, 0.6]);
    }

    #[test]
    fn channels_the_sound_lacks_are_silent() {
        let mut voice = voice(sound(2, RATE, vec![0.2, 0.6]), PlaySettings::default(), 4);

        let (output, _) = mixed(&mut voice, 1, 4);

        assert_eq!(output, [0.2, 0.6, 0.0, 0.0]);
    }

    #[test]
    fn volume_scales_the_samples() {
        let settings = PlaySettings {
            volume: 0.5,
            ..PlaySettings::default()
        };
        let mut voice = voice(sound(1, RATE, vec![0.8]), settings, 1);

        let (output, _) = mixed(&mut voice, 1, 1);

        assert_eq!(output, [0.4]);
    }

    #[test]
    fn unusable_volumes_are_silent() {
        assert!(sanitized_volume(-1.0).abs() < f32::EPSILON);
        assert!(sanitized_volume(f32::NAN).abs() < f32::EPSILON);
        assert!(sanitized_volume(f32::INFINITY).abs() < f32::EPSILON);
        assert!((sanitized_volume(2.0) - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn empty_sounds_end_at_once() {
        let settings = PlaySettings {
            looping: true,
            ..PlaySettings::default()
        };
        let mut voice = voice(sound(1, RATE, Vec::new()), settings, 1);

        let (output, playing) = mixed(&mut voice, 2, 1);

        assert_eq!(output, [0.0, 0.0]);
        assert!(!playing);
    }
}
