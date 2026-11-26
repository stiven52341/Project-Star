//! The kinds of files of the assets folder the editor treats apart.

/// Extension of StarScript files.
pub(crate) const SCRIPT_EXTENSION: &str = ".star";
/// Extension of blueprint files.
pub(crate) const BLUEPRINT_EXTENSION: &str = ".blueprint";
/// Extension of scene files.
pub(crate) const SCENE_EXTENSION: &str = ".scene";
/// Extension of Rust source files.
pub(crate) const RUST_EXTENSION: &str = ".rs";
/// Extension of the files of a sprite's animations (`.frames`).
pub(crate) const FRAMES_EXTENSION: &str = star_engine::star_scene::FRAMES_EXTENSION;
/// Extension of the files of an animator's keyframe animations (`.anim`).
pub(crate) const KEYFRAMES_EXTENSION: &str = star_engine::star_scene::ANIMATIONS_EXTENSION;
/// Extension of the files of an animator's state machine (`.states`).
pub(crate) const STATES_EXTENSION: &str = star_engine::star_scene::STATES_EXTENSION;
