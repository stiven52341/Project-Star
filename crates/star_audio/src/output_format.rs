//! The shape of the samples an audio output takes.

use star_core::sound::MAX_CHANNELS;
use star_core::{EngineError, EngineResult};

/// The channels and sample rate of an audio output: what the mixer writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputFormat {
    channels: u16,
    sample_rate: u32,
}

impl OutputFormat {
    /// An output of `channels` channels played `sample_rate` frames per second.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Initialization`] if there are no channels or
    /// more than [`MAX_CHANNELS`], or the sample rate is zero.
    pub fn new(channels: u16, sample_rate: u32) -> EngineResult<Self> {
        if channels == 0 || channels > MAX_CHANNELS || sample_rate == 0 {
            return Err(EngineError::Initialization {
                subsystem: "audio",
                reason: format!(
                    "outputs need 1 to {MAX_CHANNELS} channels and a sample rate, got {channels} channels at {sample_rate} Hz"
                ),
            });
        }
        Ok(Self {
            channels,
            sample_rate,
        })
    }

    /// Number of channels: 2 for stereo speakers or headphones.
    #[must_use]
    pub const fn channels(self) -> u16 {
        self.channels
    }

    /// Frames played per second.
    #[must_use]
    pub const fn sample_rate(self) -> u32 {
        self.sample_rate
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_keep_their_channels_and_rate() {
        let format = OutputFormat::new(2, 48_000).unwrap();

        assert_eq!((format.channels(), format.sample_rate()), (2, 48_000));
    }

    #[test]
    fn invalid_formats_are_rejected() {
        assert!(OutputFormat::new(0, 48_000).is_err());
        assert!(OutputFormat::new(MAX_CHANNELS + 1, 48_000).is_err());
        assert!(OutputFormat::new(2, 0).is_err());
    }
}
