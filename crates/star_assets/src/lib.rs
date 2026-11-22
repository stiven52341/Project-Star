//! Asset management for ProjectStar.
//!
//! Responsible for loading resources (textures, meshes, audio, scenes),
//! caching them behind typed handles and hot-reloading them during development.
//! For now it decodes PNG images; the `png` crate stays hidden behind it.

mod png_image;

pub use png_image::{decode_png, load_png};
