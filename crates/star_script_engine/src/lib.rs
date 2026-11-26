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
pub mod frames;
pub mod game;
pub mod input;
pub mod machines;
mod mirror;
pub mod scenes;
pub mod tracks;

use std::cell::RefCell;
use std::rc::Rc;

use star_engine::star_assets::AssetFolder;
use star_engine::star_audio::Audio;

/// Gives StarScript's runtime the scenes, sounds, `.frames`, `.anim` and
/// `.states` files of the asset `folder`: `Scene.load` reads scenes,
/// sprites play the clips of `.frames` files, animators the animations of
/// `.anim` files as their `.states` machines choose, and
/// `Sound` plays sounds through the returned
/// audio output, which stays silent until the game replaces it with the
/// engine's.
#[must_use]
pub fn connect_runtime(folder: &AssetFolder) -> Rc<RefCell<Audio>> {
    let output = Rc::new(RefCell::new(Audio::silent()));
    star_script_runtime::audio::set_backend(audio::EngineAudio::new(
        folder.clone(),
        Rc::clone(&output),
    ));
    let scene_folder = folder.clone();
    star_script_runtime::scenes::set_loader(move |path| scenes::read_scene(&scene_folder, path));
    let frames_folder = folder.clone();
    star_script_runtime::animations::set_loader(move |path| {
        frames::read_frames(&frames_folder, path)
    });
    let tracks_folder = folder.clone();
    star_script_runtime::tracks::set_loader(move |path| {
        tracks::read_animations(&tracks_folder, path)
    });
    let states_folder = folder.clone();
    star_script_runtime::machines::set_loader(move |path| {
        machines::read_states(&states_folder, path)
    });
    output
}
