//! Playing the sound players of the world's nodes (decision 87): each one
//! that plays on start does so the first frame the engine sees it, and its
//! sound stops when the node or its player goes away.

use std::collections::HashMap;
use std::sync::Arc;

use star_audio::{Audio, PlaySettings, VoiceId};
use star_core::{EngineResult, Sound};
use star_ecs::{Entity, World};
use star_scene::SoundPlayerNode;

/// The players the engine has seen, and the sound each one plays.
#[derive(Debug, Default)]
pub(crate) struct SoundPlayers {
    seen: HashMap<Entity, Option<VoiceId>>,
}

impl SoundPlayers {
    /// Starts the players of `world` seen for the first time that play on
    /// start, loading their sounds with `load` (a sound that cannot be
    /// loaded or played is warned of, and stays silent), and stops the
    /// sounds of the players that are gone.
    ///
    /// # Errors
    ///
    /// [`EngineError::Ecs`](star_core::EngineError::Ecs) if a running
    /// query holds the players.
    pub(crate) fn update(
        &mut self,
        world: &World,
        audio: &Audio,
        mut load: impl FnMut(&SoundPlayerNode) -> EngineResult<Arc<Sound>>,
    ) -> EngineResult<()> {
        self.seen.retain(|&entity, voice| {
            let kept = world.is_alive(entity) && world.has::<SoundPlayerNode>(entity);
            if let (false, Some(voice)) = (kept, voice) {
                audio.stop(*voice);
            }
            kept
        });
        let seen = &mut self.seen;
        world.each::<&SoundPlayerNode>(|entity, player| {
            if seen.contains_key(&entity) {
                return;
            }
            let voice = player
                .play_on_start
                .then(|| {
                    let settings = PlaySettings {
                        volume: player.volume,
                        looping: player.looping,
                    };
                    load(player)
                        .and_then(|sound| audio.play(sound, settings))
                        .inspect_err(|error| {
                            log::warn!("Cannot play \"{}\": {error}", player.sound);
                        })
                        .ok()
                })
                .flatten();
            seen.insert(entity, voice);
        })
    }
}

#[cfg(test)]
mod tests {
    use star_assets::AssetPath;
    use star_scene::Hierarchy as _;

    use super::*;

    fn player(play_on_start: bool) -> SoundPlayerNode {
        SoundPlayerNode {
            play_on_start,
            ..SoundPlayerNode::new(AssetPath::new("hit.wav").unwrap())
        }
    }

    fn beep() -> EngineResult<Arc<Sound>> {
        Sound::from_interleaved(1, 44_100, vec![0.5; 44_100]).map(Arc::new)
    }

    #[test]
    fn players_on_start_play_once_and_stop_when_they_go() {
        // Silent: sounds are taken, never heard.
        let audio = Audio::silent();
        let mut world = World::new();
        let music = world.spawn_node("Music", None).unwrap();
        world.insert(music, player(true)).unwrap();
        let later = world.spawn_node("Later", None).unwrap();
        world.insert(later, player(false)).unwrap();
        let mut players = SoundPlayers::default();
        let mut loads = 0;

        for _ in 0..3 {
            players
                .update(&world, &audio, |_| {
                    loads += 1;
                    beep()
                })
                .unwrap();
        }
        assert_eq!(loads, 1, "only the player on start, once");
        assert!(players.seen[&music].is_some());
        assert_eq!(players.seen[&later], None);

        world.despawn_node(music);
        players.update(&world, &audio, |_| beep()).unwrap();

        assert!(!players.seen.contains_key(&music), "its sound is stopped");
        assert!(players.seen.contains_key(&later));
    }
}
