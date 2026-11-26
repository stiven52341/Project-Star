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
mod light;
mod light_2d;
mod material;
mod mesh;
mod mesh_data;
mod mesh_pipeline;
mod model_mesh;
mod renderer;
mod scene_renderer;
mod sprite;
mod sprite_batch;
mod sprite_pipeline;
mod store;
mod surface_pipeline;
mod texture;
mod vertex;

pub use backend::{GraphicsBackend, UnknownBackendError};
pub use camera::{Camera, DEFAULT_2D_VIEW_HEIGHT, DEFAULT_VERTICAL_FOV, Projection};
pub use color::Color;
pub use frame::Frame;
pub use gpu_info::GpuInfo;
pub use light::{
    AmbientLight, DEFAULT_SPOT_ANGLE, DEFAULT_SPOT_BLEND, DirectionalLight, Light, MAX_LIGHTS,
    MAX_SPOT_ANGLE, PointLight, SpotLight,
};
pub use light_2d::{Light2d, PointLight2d, SpotLight2d};
pub use material::{DEFAULT_ROUGHNESS, Material};
pub use mesh::{Mesh3dId, MeshId};
pub use mesh_data::{
    DEFAULT_SPHERE_RINGS, DEFAULT_SPHERE_SEGMENTS, MAX_SPHERE_DIVISIONS, MIN_SPHERE_RINGS,
    MIN_SPHERE_SEGMENTS, MeshData, MeshShape, MeshVertex,
};
pub use model_mesh::{ModelMeshPart, upload_model};
pub use renderer::{Renderer, RendererConfig};
pub use sprite::{Sprite, SpriteSheet, TextureRegion};
pub use star_core::Image;
pub use texture::{TextureFilter, TextureId};
pub use vertex::Vertex;
