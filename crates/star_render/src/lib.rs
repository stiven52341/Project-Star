//! Rendering for ProjectStar.
//!
//! Provides 2D (sprites, tilemaps, text) and 3D (meshes, materials, lights)
//! rendering on top of `wgpu`, which targets DirectX 12, Vulkan, Metal and
//! OpenGL. `wgpu` types never appear in this crate's public API, so a custom
//! backend (e.g. for consoles) can be added later without breaking games.
//! The only exception is the opt-in `interop` module (feature `interop`),
//! reserved for engine tools such as the editor.

mod backend;
mod bytes;
mod camera;
mod color;
mod depth;
mod frame;
mod gpu_info;
#[cfg(test)]
mod gpu_tests;
#[cfg(feature = "interop")]
pub mod interop;
mod mesh;
mod mesh_pipeline;
mod renderer;
mod scene_renderer;
mod sprite;
mod sprite_batch;
mod sprite_pipeline;
mod store;
mod texture;
mod vertex;

pub use backend::{GraphicsBackend, UnknownBackendError};
pub use camera::{Camera, Projection};
pub use color::Color;
pub use frame::Frame;
pub use gpu_info::GpuInfo;
pub use mesh::MeshId;
pub use renderer::{Renderer, RendererConfig};
pub use sprite::{Sprite, TextureRegion};
pub use star_core::Image;
pub use texture::{TextureFilter, TextureId};
pub use vertex::Vertex;
