//! Sound players as parts of nodes (decision 87, as Unity's audio
//! sources): a node that plays a sound file, at a volume, once or again
//! and again, from the moment it appears if it plays on start; and its
//! `[node.sound_player]` table.

use serde::{Deserialize, Serialize};
use star_assets::{AssetPath, SoundFormat};
use star_core::{EngineError, EngineResult};

/// The volume of a player that does not say: as recorded.
pub const DEFAULT_PLAYER_VOLUME: f32 = 1.0;
/// Within this distance a spatial player is as loud as it plays, in world
/// units, unless it says.
pub const DEFAULT_NEAR: f32 = 1.0;
/// Beyond this distance a spatial player is not heard, unless it says.
pub const DEFAULT_FAR: f32 = 20.0;

/// Plays a sound at a node.
#[derive(Debug, Clone, PartialEq)]
pub struct SoundPlayerNode {
    /// The sound file, in the asset folder.
    pub sound: AssetPath,
    /// How loud: 1 as recorded, 0 silent.
    pub volume: f32,
    /// Whether it starts again when it ends.
    pub looping: bool,
    /// Whether it plays as soon as the node appears.
    pub play_on_start: bool,
    /// The group of sounds whose volume also scales it (music, effects…),
    /// if any.
    pub group: Option<String>,
    /// Whether it is heard from where it is: quieter the farther it is from
    /// the camera, and from its side.
    pub spatial: bool,
    /// Within this distance it is as loud as it plays.
    pub near: f32,
    /// Beyond this distance it is not heard.
    pub far: f32,
}

impl SoundPlayerNode {
    /// A player of `sound`, as recorded, once, from the start.
    #[must_use]
    pub const fn new(sound: AssetPath) -> Self {
        Self {
            sound,
            volume: DEFAULT_PLAYER_VOLUME,
            looping: false,
            play_on_start: true,
            group: None,
            spatial: false,
            near: DEFAULT_NEAR,
            far: DEFAULT_FAR,
        }
    }
}

/// A `[node.sound_player]` table: only what is not the default is written.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SoundPlayerData {
    sound: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    volume: Option<f32>,
    #[serde(default, skip_serializing_if = "is_false")]
    looping: bool,
    #[serde(default = "playing_on_start", skip_serializing_if = "is_true")]
    play_on_start: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    group: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    spatial: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    near: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    far: Option<f32>,
}

const fn playing_on_start() -> bool {
    true
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde passes the field by reference"
)]
const fn is_false(value: &bool) -> bool {
    !*value
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde passes the field by reference"
)]
const fn is_true(value: &bool) -> bool {
    *value
}

impl SoundPlayerData {
    /// The player the table describes.
    ///
    /// # Errors
    ///
    /// [`EngineError::Asset`] if the file is not a sound of a known
    /// format, or [`EngineError::Scene`] if the volume is negative or not
    /// a number.
    pub(crate) fn to_player(&self) -> EngineResult<SoundPlayerNode> {
        let sound = AssetPath::new(&self.sound)?;
        SoundFormat::of(&sound)?;
        let volume = self.volume.unwrap_or(DEFAULT_PLAYER_VOLUME);
        if !(volume.is_finite() && volume >= 0.0) {
            return Err(EngineError::Scene(format!(
                "the volume of a sound player cannot be {volume}"
            )));
        }
        let near = self.near.unwrap_or(DEFAULT_NEAR);
        let far = self.far.unwrap_or(DEFAULT_FAR);
        if !(near.is_finite() && far.is_finite() && near >= 0.0 && far >= near) {
            return Err(EngineError::Scene(format!(
                "a sound player is heard from {near} to {far}, which is no reach"
            )));
        }
        Ok(SoundPlayerNode {
            sound,
            volume,
            looping: self.looping,
            play_on_start: self.play_on_start,
            group: self.group.clone().filter(|group| !group.is_empty()),
            spatial: self.spatial,
            near,
            far,
        })
    }

    pub(crate) fn from_player(player: &SoundPlayerNode) -> Self {
        // Only the volume exactly the default is left out.
        let volume = (player.volume != DEFAULT_PLAYER_VOLUME).then_some(player.volume);
        Self {
            sound: player.sound.as_str().to_owned(),
            volume,
            looping: player.looping,
            play_on_start: player.play_on_start,
            group: player.group.clone(),
            spatial: player.spatial,
            // Only reaches exactly the default are left out.
            near: (player.near != DEFAULT_NEAR).then_some(player.near),
            far: (player.far != DEFAULT_FAR).then_some(player.far),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(text: &str) -> EngineResult<SoundPlayerNode> {
        toml::from_str::<SoundPlayerData>(text)
            .map_err(|error| EngineError::Scene(error.to_string()))?
            .to_player()
    }

    #[test]
    fn players_name_a_sound_and_play_it_on_start_as_recorded() {
        let music = player("sound = \"music/theme.ogg\"").unwrap();

        assert_eq!(
            music,
            SoundPlayerNode::new(AssetPath::new("music/theme.ogg").unwrap())
        );
        let quiet =
            player("sound = \"hit.wav\"\nvolume = 0.5\nlooping = true\nplay_on_start = false")
                .unwrap();
        assert!(quiet.looping && !quiet.play_on_start);
        assert!((quiet.volume - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn players_need_a_sound_and_a_volume_that_can_be_heard() {
        for text in [
            "sound = \"hit.png\"",
            "sound = \"../hit.wav\"",
            "sound = \"hit.wav\"\nvolume = -1.0",
            "sound = \"hit.wav\"\nnear = 5.0\nfar = 2.0",
            "volume = 1.0",
        ] {
            assert!(player(text).is_err(), "{text}");
        }
    }

    #[test]
    fn only_what_is_not_the_default_is_written() {
        let mut player = SoundPlayerNode::new(AssetPath::new("hit.wav").unwrap());
        assert_eq!(
            toml::to_string(&SoundPlayerData::from_player(&player)).unwrap(),
            "sound = \"hit.wav\"\n"
        );

        player.volume = 0.25;
        player.play_on_start = false;
        player.group = Some("effects".to_owned());
        player.spatial = true;
        player.far = 8.0;
        let text = toml::to_string(&SoundPlayerData::from_player(&player)).unwrap();

        assert_eq!(
            toml::from_str::<SoundPlayerData>(&text)
                .unwrap()
                .to_player()
                .unwrap(),
            player
        );
    }
}
