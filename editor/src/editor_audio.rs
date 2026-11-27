//! The editor's own sound output, to hear sounds while editing: opened the
//! first time something plays, and kept while the editor runs. Without a
//! sound device the editor stays silent (warned once).

use std::path::Path;
use std::sync::Arc;

use star_engine::star_audio::{Audio, AudioOutput, PlaySettings, VoiceId};

use crate::sound_editor::read_sound;

/// The sound device of the editor, once something played.
#[derive(Default)]
pub(crate) struct EditorAudio {
    /// The output, once opened; `Err` if it cannot be.
    output: Option<Result<AudioOutput, ()>>,
    /// The sound the asset list plays, until another one starts.
    listened: Option<VoiceId>,
}

impl EditorAudio {
    /// The mixer to play sounds with; `None` without a sound device.
    pub(crate) fn audio(&mut self) -> Option<Audio> {
        let output = self.output.get_or_insert_with(|| {
            AudioOutput::open_default()
                .inspect_err(|error| log::warn!("The editor cannot play sounds: {error}"))
                .map_err(|_| ())
        });
        output.as_ref().ok().map(AudioOutput::audio)
    }

    /// Plays the sound at `path` inside the `assets` folder, in place of
    /// the one this played before; a sound that cannot be read or played
    /// is only logged.
    pub(crate) fn listen(&mut self, assets: &Path, path: &str) {
        let Some(audio) = self.audio() else {
            return;
        };
        if let Some(voice) = self.listened.take() {
            audio.stop(voice);
        }
        let played = read_sound(assets, path).and_then(|sound| {
            audio
                .play(Arc::new(sound), PlaySettings::default())
                .map_err(|error| error.to_string())
        });
        match played {
            Ok(voice) => self.listened = Some(voice),
            Err(error) => log::warn!("Cannot play \"{path}\": {error}"),
        }
    }
}
