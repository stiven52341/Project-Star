//! Android: the app the system started, which the event loop runs in.
//!
//! Android does not call `main`. It loads the game as a library and calls
//! `android_main` with the [`AndroidApp`] it runs in; the game defines it
//! with [`android_main!`](crate::android_main), which hands the app to
//! [`start`]. The event loop takes it from there.

use std::fmt::Display;
use std::sync::OnceLock;

pub use winit::platform::android::activity::AndroidApp;

/// The tag of the engine's lines in `adb logcat`.
const LOG_TAG: &str = "ProjectStar";

/// The app the system started the game in.
static APP: OnceLock<AndroidApp> = OnceLock::new();

/// Runs the game `run` in `app`: sends the log to logcat, keeps the app for
/// the event loop, and logs the error `run` ends with, if any.
pub fn start<E: Display>(app: AndroidApp, run: impl FnOnce() -> Result<(), E>) {
    android_logger::init_once(
        android_logger::Config::default()
            .with_max_level(log::LevelFilter::Info)
            .with_tag(LOG_TAG),
    );
    if APP.set(app).is_err() {
        log::warn!("The game was started twice; the first app is kept");
    }
    if let Err(error) = run() {
        log::error!("{error}");
    }
}

/// The app the game was started in, if [`start`] was called.
pub(crate) fn app() -> Option<AndroidApp> {
    APP.get().cloned()
}
