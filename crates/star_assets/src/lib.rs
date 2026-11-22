//! Asset management for ProjectStar.
//!
//! A game's assets live in its [`AssetFolder`] and are named by
//! [`AssetPath`]s, the same on every platform. Images are decoded by their
//! [`ImageFormat`]; the `png` crate stays hidden behind this crate.

mod asset_folder;
mod asset_path;
mod image_format;
mod png_image;

pub use asset_folder::{AssetFolder, DEFAULT_ASSET_FOLDER};
pub use asset_path::AssetPath;
pub use image_format::ImageFormat;
pub use png_image::decode_png;
