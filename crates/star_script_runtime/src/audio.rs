//! The sounds StarScript's `Sound` and `Audio` classes play.
//!
//! Decoding and playing sounds needs the engine, which this crate cannot
//! depend on (console programs run without it). So a game registers an
//! [`AudioBackend`] when it starts ([`set_backend`]), and the generated
//! classes call the functions of this module, which forward to it. Each
//! path is loaded only once ([`load`]): every `Sound` of the same file
//! shares its samples. Without a backend nothing is heard, and sounds
//! cannot be loaded. The game runs on one thread, so all of it is per
//! thread.

use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Write as _;

/// A sound the backend loaded, by its number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SoundId(pub usize);

/// One playing of a sound, by its number (never reused).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VoiceId(pub u64);

/// What actually loads and plays sounds: the engine, in a game.
pub trait AudioBackend {
    /// Loads the sound at the asset path `path`, or explains why it cannot.
    ///
    /// # Errors
    ///
    /// The reason, for the warning the game writes.
    fn load(&mut self, path: &str) -> Result<SoundId, String>;

    /// Starts playing `sound` at `volume` (1 as recorded), again and again
    /// if `looping`.
    ///
    /// # Errors
    ///
    /// The reason it cannot play now (too many sounds at once).
    fn play(&mut self, sound: SoundId, volume: f32, looping: bool) -> Result<VoiceId, String>;

    /// Stops `voice`, if it still plays.
    fn stop(&mut self, voice: VoiceId);

    /// Whether `voice` still plays.
    fn is_playing(&self, voice: VoiceId) -> bool;

    /// The volume every sound is scaled by.
    fn volume(&self) -> f32;

    /// Changes the volume every sound is scaled by.
    fn set_volume(&mut self, volume: f32);
}

/// Master volume while no backend is set.
const DEFAULT_VOLUME: f32 = 1.0;

thread_local! {
    static BACKEND: RefCell<Option<Box<dyn AudioBackend>>> = const { RefCell::new(None) };
    /// Each path loaded, with its sound or `None` if it could not be loaded.
    static LOADED: RefCell<HashMap<String, Option<SoundId>>> = RefCell::new(HashMap::new());
}

/// Sets what loads and plays sounds. Called once, before the game is created.
pub fn set_backend(backend: impl AudioBackend + 'static) {
    BACKEND.with(|current| {
        if let Ok(mut current) = current.try_borrow_mut() {
            *current = Some(Box::new(backend));
        }
    });
    LOADED.with(|loaded| {
        if let Ok(mut loaded) = loaded.try_borrow_mut() {
            loaded.clear();
        }
    });
}

/// The sound at the asset path `path`. `None` if it cannot be loaded: the
/// reason is written once to the error output, and the game goes on.
#[must_use]
pub fn load(path: &str) -> Option<SoundId> {
    if let Some(loaded) = LOADED.with(|loaded| loaded.borrow().get(path).copied()) {
        return loaded;
    }
    let result = with_backend(
        |backend| backend.load(path),
        || Err("sounds can only be loaded while a game runs".to_owned()),
    );
    let loaded = result
        .inspect_err(|reason| warn(&format!("cannot load the sound \"{path}\": {reason}")))
        .ok();
    LOADED.with(|cache| {
        cache.borrow_mut().insert(path.to_owned(), loaded);
    });
    loaded
}

/// Starts playing `sound`; `None` if it cannot play now (the reason is
/// written to the error output).
#[must_use]
pub fn play(sound: SoundId, volume: f32, looping: bool) -> Option<VoiceId> {
    with_backend(
        |backend| backend.play(sound, volume, looping),
        || Err(String::new()),
    )
    .inspect_err(|reason| {
        if !reason.is_empty() {
            warn(&format!("cannot play a sound: {reason}"));
        }
    })
    .ok()
}

/// Stops `voice`, if it still plays.
pub fn stop(voice: VoiceId) {
    with_backend(|backend| backend.stop(voice), || ());
}

/// Whether `voice` still plays.
#[must_use]
pub fn is_playing(voice: VoiceId) -> bool {
    with_backend(|backend| backend.is_playing(voice), || false)
}

/// The volume every sound is scaled by.
#[must_use]
pub fn volume() -> f32 {
    with_backend(|backend| backend.volume(), || DEFAULT_VOLUME)
}

/// Changes the volume every sound is scaled by.
pub fn set_volume(volume: f32) {
    with_backend(|backend| backend.set_volume(volume), || ());
}

/// Runs `action` on the backend, or gives `missing()` without one.
fn with_backend<T>(
    action: impl FnOnce(&mut dyn AudioBackend) -> T,
    missing: impl FnOnce() -> T,
) -> T {
    BACKEND.with(|backend| match backend.try_borrow_mut() {
        Ok(mut backend) => match backend.as_deref_mut() {
            Some(backend) => action(backend),
            None => missing(),
        },
        Err(_) => missing(),
    })
}

fn warn(message: &str) {
    // Nothing can be reported if the error output itself fails.
    let _ = writeln!(std::io::stderr(), "warning: {message}");
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;

    /// Plays nothing; remembers what it was asked.
    #[derive(Default)]
    struct FakeBackend {
        loads: Rc<Cell<usize>>,
        playing: Vec<VoiceId>,
        next_voice: u64,
        volume: f32,
    }

    impl AudioBackend for FakeBackend {
        fn load(&mut self, path: &str) -> Result<SoundId, String> {
            self.loads.set(self.loads.get() + 1);
            if path.starts_with("sounds/") {
                Ok(SoundId(self.loads.get()))
            } else {
                Err("not a sound".to_owned())
            }
        }

        fn play(&mut self, _: SoundId, _: f32, _: bool) -> Result<VoiceId, String> {
            let voice = VoiceId(self.next_voice);
            self.next_voice += 1;
            self.playing.push(voice);
            Ok(voice)
        }

        fn stop(&mut self, voice: VoiceId) {
            self.playing.retain(|playing| *playing != voice);
        }

        fn is_playing(&self, voice: VoiceId) -> bool {
            self.playing.contains(&voice)
        }

        fn volume(&self) -> f32 {
            self.volume
        }

        fn set_volume(&mut self, volume: f32) {
            self.volume = volume;
        }
    }

    #[test]
    fn without_a_game_nothing_loads_or_plays() {
        // Each test runs on its own thread, so no backend is set here.
        assert_eq!(load("sounds/jump.wav"), None);
        assert_eq!(play(SoundId(0), 1.0, false), None);
        assert!(!is_playing(VoiceId(0)));
        assert!((volume() - DEFAULT_VOLUME).abs() < f32::EPSILON);
    }

    #[test]
    fn each_path_is_loaded_once() {
        let loads = Rc::new(Cell::new(0));
        set_backend(FakeBackend {
            loads: Rc::clone(&loads),
            ..FakeBackend::default()
        });

        let first = load("sounds/jump.wav");
        let second = load("sounds/jump.wav");
        assert!(load("notes/jump.txt").is_none());
        assert!(load("notes/jump.txt").is_none());

        assert!(first.is_some());
        assert_eq!(first, second);
        assert_eq!(loads.get(), 2);
    }

    #[test]
    fn voices_play_until_stopped() {
        set_backend(FakeBackend::default());
        let sound = load("sounds/jump.wav").unwrap();

        let voice = play(sound, 1.0, true).unwrap();
        assert!(is_playing(voice));

        stop(voice);
        assert!(!is_playing(voice));
    }

    #[test]
    fn the_volume_goes_to_the_backend() {
        set_backend(FakeBackend::default());

        set_volume(0.25);

        assert!((volume() - 0.25).abs() < f32::EPSILON);
    }
}
