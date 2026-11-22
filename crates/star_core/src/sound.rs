//! Sounds in memory: what sound files decode to and the mixer plays.

use crate::{EngineError, EngineResult};

/// Most channels a sound may have: 7.1 surround.
pub const MAX_CHANNELS: u16 = 8;

/// A sound in memory: its samples as numbers from -1 to 1, interleaved
/// (one sample of every channel, then the next one of every channel…),
/// played `sample_rate` frames per second.
#[derive(Debug, Clone, PartialEq)]
pub struct Sound {
    channels: u16,
    sample_rate: u32,
    samples: Vec<f32>,
}

impl Sound {
    /// Creates a sound of `channels` channels from its interleaved samples.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Asset`] if there are no channels or more than
    /// [`MAX_CHANNELS`], the sample rate is zero, or `samples` does not hold
    /// the same number of samples for every channel.
    pub fn from_interleaved(
        channels: u16,
        sample_rate: u32,
        samples: Vec<f32>,
    ) -> EngineResult<Self> {
        if channels == 0 || channels > MAX_CHANNELS {
            return Err(EngineError::Asset(format!(
                "sounds have from 1 to {MAX_CHANNELS} channels, got {channels}"
            )));
        }
        if sample_rate == 0 {
            return Err(EngineError::Asset(
                "sounds need a sample rate above zero".to_owned(),
            ));
        }
        if samples.len() % usize::from(channels) != 0 {
            return Err(EngineError::Asset(format!(
                "a sound of {channels} channels got {} samples, not a whole number of frames",
                samples.len()
            )));
        }
        Ok(Self {
            channels,
            sample_rate,
            samples,
        })
    }

    /// Number of channels: 1 for mono, 2 for stereo…
    #[must_use]
    pub const fn channels(&self) -> u16 {
        self.channels
    }

    /// Frames played per second.
    #[must_use]
    pub const fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Every sample, interleaved by channel.
    #[must_use]
    pub fn samples(&self) -> &[f32] {
        &self.samples
    }

    /// Number of frames: one sample of every channel.
    #[must_use]
    pub fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels)
    }

    /// How long the sound lasts, in seconds.
    #[must_use]
    // A duration in seconds does not need every frame to be exact.
    #[allow(clippy::cast_precision_loss)]
    pub fn duration_seconds(&self) -> f64 {
        self.frames() as f64 / f64::from(self.sample_rate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sounds_keep_their_samples() {
        let sound = Sound::from_interleaved(2, 4, vec![0.0, 0.5, -0.5, 1.0]).unwrap();

        assert_eq!(sound.channels(), 2);
        assert_eq!(sound.sample_rate(), 4);
        assert_eq!(sound.frames(), 2);
        assert!((sound.duration_seconds() - 0.5).abs() < f64::EPSILON);
        assert_eq!(sound.samples(), [0.0, 0.5, -0.5, 1.0]);
    }

    #[test]
    fn channels_must_be_between_one_and_the_most() {
        assert!(Sound::from_interleaved(0, 44_100, Vec::new()).is_err());
        assert!(Sound::from_interleaved(MAX_CHANNELS + 1, 44_100, Vec::new()).is_err());
    }

    #[test]
    fn the_sample_rate_cannot_be_zero() {
        assert!(Sound::from_interleaved(1, 0, vec![0.0]).is_err());
    }

    #[test]
    fn every_channel_needs_the_same_samples() {
        assert!(Sound::from_interleaved(2, 44_100, vec![0.0; 3]).is_err());
    }

    #[test]
    fn empty_sounds_are_silent_and_last_nothing() {
        let sound = Sound::from_interleaved(1, 44_100, Vec::new()).unwrap();

        assert_eq!(sound.frames(), 0);
        assert!(sound.duration_seconds().abs() < f64::EPSILON);
    }
}
