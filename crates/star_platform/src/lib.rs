//! Platform layer for ProjectStar.
//!
//! This is the only crate allowed to contain platform-specific code
//! (`#[cfg(target_os = ...)]`) and the only one that knows about `winit` and
//! `gilrs` (gamepads).
//! Console support (PlayStation, Xbox) will be added in isolated modules
//! because their SDKs are under NDA.

#[cfg(target_os = "android")]
pub mod android;
mod entry;
mod event_loop;
mod file_explorer;
mod gilrs_input;
mod input_event;
mod packaged_assets;
mod target;
pub mod user_directories;
mod window;
mod winit_input;

pub use event_loop::{WindowEventHandler, run_event_loop};
pub use file_explorer::show_in_file_explorer;
pub use input_event::{
    ButtonState, GamepadAxis, GamepadButton, InputEvent, Key, MouseButton, TouchPhase,
};
pub use packaged_assets::PackagedAssets;
pub use target::TargetPlatform;
pub use window::{Window, WindowConfig, WindowSize};
