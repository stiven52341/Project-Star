//! Audio for ProjectStar.
//!
//! An [`AudioOutput`] opens the system's audio device and gives [`Audio`]
//! handles to play [`Sound`](star_core::Sound)s on it. Behind them, a
//! [`Mixer`] plays many sounds at once: each one is a voice with its own
//! volume that may loop, read at the rate and channels of the
//! [`OutputFormat`] and added into the device's samples. The `cpal` crate
//! stays hidden behind this crate.

mod audio;
mod audio_output;
mod mixer;
mod output_format;
mod voice;

pub use audio::Audio;
pub use audio_output::AudioOutput;
pub use mixer::{MAX_VOICES, Mixer, VoiceId};
pub use output_format::OutputFormat;
pub use voice::PlaySettings;
