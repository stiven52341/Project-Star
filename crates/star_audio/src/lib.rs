//! Audio for ProjectStar.
//!
//! A [`Mixer`] plays many [`Sound`](star_core::Sound)s at once: each one is
//! a voice with its own volume that may loop, read at the rate and channels
//! of the [`OutputFormat`] and added into the output's samples.

mod mixer;
mod output_format;
mod voice;

pub use mixer::{MAX_VOICES, Mixer, VoiceId};
pub use output_format::OutputFormat;
pub use voice::PlaySettings;
