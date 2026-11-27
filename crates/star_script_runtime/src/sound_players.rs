//! StarScript's `SoundPlayer` (decision 87): a node that plays a sound
//! file, through the game's audio backend ([`crate::audio`]), at its
//! volume, once or again and again; from the frame it starts if it plays
//! on start, until it is stopped or removed from the tree.

use std::cell::Cell;

use crate::Field;
use crate::audio::{VoiceId, is_playing, load, play, stop};
use crate::nodes::Host;

/// The volume of a new player: as recorded.
const DEFAULT_VOLUME: f32 = 1.0;

/// The fields of a `SoundPlayer`, and the sound it plays.
pub struct SoundPlayerFields {
    /// `sound`: the asset path of its sound file.
    pub sound: Field<String>,
    /// `volume`: 1 as recorded, 0 silent.
    pub volume: Field<f32>,
    /// `looping`: whether it starts again when it ends.
    pub looping: Field<bool>,
    /// `play_on_start`: whether it plays when it starts.
    pub play_on_start: Field<bool>,
    voice: Cell<Option<VoiceId>>,
}

impl SoundPlayerFields {
    /// A player of `sound`, as recorded, once, from the start.
    #[must_use]
    pub const fn new(sound: String) -> Self {
        Self {
            sound: Field::new(sound),
            volume: Field::new(DEFAULT_VOLUME),
            looping: Field::new(false),
            play_on_start: Field::new(true),
            voice: Cell::new(None),
        }
    }

    /// `func play`: plays its sound from the beginning, stopping the one it
    /// played; a sound that cannot be loaded is warned of once, and stays
    /// silent.
    pub fn play(&self) {
        self.stop();
        let voice = self
            .sound
            .read(|path| load(path))
            .and_then(|sound| play(sound, self.volume.get(), self.looping.get()));
        self.voice.set(voice);
    }

    /// `func stop`: stops its sound, if it plays.
    pub fn stop(&self) {
        if let Some(voice) = self.voice.take() {
            stop(voice);
        }
    }

    /// `func is_playing`: whether its sound still plays.
    #[must_use]
    pub fn is_playing(&self) -> bool {
        self.voice.get().is_some_and(is_playing)
    }
}

/// Plays the sound of `node` if it is a player that plays on start: for
/// the frame it starts.
pub fn started<H: Host>(node: &H::Node) {
    H::with_sound_player(node, |player| {
        if player.play_on_start.get() {
            player.play();
        }
    });
}

/// Stops the sounds of the players in the tree under `node`, itself
/// included: it was removed from the game.
pub fn removed<H: Host>(node: &H::Node) {
    H::with_sound_player(node, SoundPlayerFields::stop);
    for child in H::node(node).children().iter() {
        removed::<H>(child);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn players_without_a_game_are_silent() {
        let player = SoundPlayerFields::new("music/theme.ogg".to_owned());

        player.play();

        assert!(!player.is_playing());
        assert!(player.play_on_start.get());
        player.stop();
    }
}
