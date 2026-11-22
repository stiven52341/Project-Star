//! Platform layer for ProjectStar.
//!
//! This is the only crate allowed to contain platform-specific code
//! (`#[cfg(target_os = ...)]`) and the only one that knows about `winit`.
//! Console support (PlayStation, Xbox) will be added in isolated modules
//! because their SDKs are under NDA.

mod event_loop;
mod target;
pub mod user_directories;
mod window;

pub use event_loop::{WindowEventHandler, run_event_loop};
pub use target::TargetPlatform;
pub use window::{Window, WindowConfig, WindowSize};
