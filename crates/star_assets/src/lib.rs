//! Asset management for ProjectStar.
//!
//! A game's assets live in its [`AssetFolder`] and are named by
//! [`AssetPath`]s, the same on every platform. Images are decoded by their
//! [`ImageFormat`] and sounds by their [`SoundFormat`], and a [`FileWatcher`]
//! notices when their files change so they can be reloaded while the game
//! runs. The `png`, `zune-jpeg` and `symphonia` crates stay hidden behind
//! this crate.

mod asset_folder;
mod asset_path;
mod file_watcher;
mod image_format;
mod jpeg_image;
mod png_image;
mod sound_format;

pub use asset_folder::{AssetFolder, DEFAULT_ASSET_FOLDER, PackagedFiles};
pub use asset_path::AssetPath;
pub use file_watcher::{DEFAULT_CHECK_INTERVAL, FileWatcher};
pub use image_format::ImageFormat;
pub use jpeg_image::decode_jpeg;
pub use png_image::decode_png;
pub use sound_format::SoundFormat;
