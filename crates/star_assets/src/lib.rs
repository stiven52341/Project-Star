//! Asset management for ProjectStar.
//!
//! A game's assets live in its [`AssetFolder`] and are named by
//! [`AssetPath`]s, the same on every platform. Images are decoded by their
//! [`ImageFormat`]; the `png` and `zune-jpeg` crates stay
//! hidden behind this crate.

mod asset_folder;
mod asset_path;
mod image_format;
mod jpeg_image;
mod png_image;

pub use asset_folder::{AssetFolder, DEFAULT_ASSET_FOLDER};
pub use asset_path::AssetPath;
pub use image_format::ImageFormat;
pub use jpeg_image::decode_jpeg;
pub use png_image::decode_png;
