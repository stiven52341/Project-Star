//! Connects StarScript's runtime with the engine.
//!
//! Games compiled from StarScript depend on this crate; StarScript
//! programmers never see it. It holds the part of a game that is the same
//! for every program — reading scenes, playing sounds, sharing the input
//! with the runtime — as ordinary Rust, built
//! once with the engine instead of being generated again into every game.
//! The StarScript interpreter uses it too, as one more `Host` of the
//! runtime (decisions 64 and 70).

pub mod audio;
pub mod game;
pub mod input;
mod mirror;
pub mod scenes;

use std::cell::RefCell;
use std::rc::Rc;

use star_engine::star_assets::AssetFolder;
use star_engine::star_audio::Audio;

/// Gives StarScript's runtime the scenes and sounds of the asset `folder`:
/// `Scene.load` reads them and `Sound` plays them through the returned
/// audio output, which stays silent until the game replaces it with the
/// engine's.
#[must_use]
pub fn connect_runtime(folder: &AssetFolder) -> Rc<RefCell<Audio>> {
    let output = Rc::new(RefCell::new(Audio::silent()));
    star_script_runtime::audio::set_backend(audio::EngineAudio::new(
        folder.clone(),
        Rc::clone(&output),
    ));
    let folder = folder.clone();
    star_script_runtime::scenes::set_loader(move |path| scenes::read_scene(&folder, path));
    output
}
