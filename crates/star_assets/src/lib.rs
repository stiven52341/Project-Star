//! Asset management for ProjectStar.
//!
//! A game's assets live in its [`AssetFolder`] and are named by
//! [`AssetPath`]s, the same on every platform. Images are decoded by their
//! [`ImageFormat`], sounds by their [`SoundFormat`] and 3D models by their
//! [`ModelFormat`], and a [`FileWatcher`]
//! notices when their files change so they can be reloaded while the game
//! runs. The `png`, `zune-jpeg`, `symphonia` and `gltf` crates stay hidden behind
//! this crate.

mod asset_folder;
mod asset_path;
mod builtin_image;
mod file_watcher;
mod gltf_model;
mod image_format;
mod jpeg_image;
mod model_format;
mod png_image;
mod sound_format;

pub use asset_folder::{AssetFolder, DEFAULT_ASSET_FOLDER, PackagedFiles};
pub use asset_path::AssetPath;
pub use builtin_image::{BuiltinImage, CIRCLE_IMAGE, SQUARE_IMAGE};
pub use file_watcher::{DEFAULT_CHECK_INTERVAL, FileWatcher};
pub use gltf_model::{ReadBeside, decode_gltf};
pub use image_format::ImageFormat;
pub use jpeg_image::decode_jpeg;
pub use model_format::ModelFormat;
pub use png_image::decode_png;
pub use sound_format::SoundFormat;
