//! Low-level access for engine tools that already own a `wgpu` device.
//!
//! **Not for games.** This module is the only part of the public API that
//! exposes `wgpu` types, and only exists when the `interop` feature is on.
//! The editor uses it to draw the scene into a texture shown by its UI,
//! sharing the GPU with it instead of copying pixels through the CPU.
//! It may change whenever `wgpu` is updated.

use star_core::EngineResult;

pub use wgpu;

use crate::depth::DepthBuffer;
use crate::mesh::Topology;
use crate::scene_renderer::SceneRenderer;
use crate::{Frame, Image, Mesh3dId, MeshData, MeshId, TextureFilter, TextureId, Vertex};

/// Non-sRGB storage holding sRGB-encoded values: UI toolkits such as egui
/// sample textures expecting gamma-encoded colors, and an `...Srgb` format
/// would make the GPU decode them a second time (darker image).
const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// Size of the target until the first [`OffscreenRenderer::resize`].
const INITIAL_SIZE: u32 = 1;

/// Draws frames into a texture owned by the renderer, on a device owned by
/// the caller, and into other [`OffscreenTarget`]s with the same meshes and
/// textures.
pub struct OffscreenRenderer {
    scene: SceneRenderer,
    target: OffscreenTarget,
}

/// A texture to draw frames into, with its depth buffer.
pub struct OffscreenTarget {
    color_view: wgpu::TextureView,
    depth: DepthBuffer,
    width: u32,
    height: u32,
}

impl OffscreenTarget {
    /// A target of one pixel on the device of `renderer`, until it is
    /// resized.
    #[must_use]
    pub fn new(renderer: &OffscreenRenderer) -> Self {
        Self::of_size(renderer.scene.device(), INITIAL_SIZE, INITIAL_SIZE)
    }

    fn of_size(device: &wgpu::Device, width: u32, height: u32) -> Self {
        Self {
            color_view: create_color_target(device, width, height),
            depth: DepthBuffer::new(device, width, height),
            width,
            height,
        }
    }

    /// Texture the frames are drawn into. Replaced by [`resize`](Self::resize).
    #[must_use]
    pub fn color_view(&self) -> &wgpu::TextureView {
        &self.color_view
    }

    /// Sets the size of the target in pixels (zero is treated as one), on
    /// the device of `renderer`. Returns `true` if the texture was
    /// replaced, so its users must refresh their references to
    /// [`color_view`](Self::color_view).
    pub fn resize(&mut self, renderer: &OffscreenRenderer, width: u32, height: u32) -> bool {
        let (width, height) = (width.max(1), height.max(1));
        if (width, height) == (self.width, self.height) {
            return false;
        }
        *self = Self::of_size(renderer.scene.device(), width, height);
        true
    }
}

impl OffscreenRenderer {
    /// Creates a renderer that draws with the caller's `device` and `queue`.
    #[must_use]
    pub fn new(device: wgpu::Device, queue: wgpu::Queue) -> Self {
        let target = OffscreenTarget::of_size(&device, INITIAL_SIZE, INITIAL_SIZE);
        Self {
            scene: SceneRenderer::new(device, queue, COLOR_FORMAT, INITIAL_SIZE, INITIAL_SIZE),
            target,
        }
    }

    /// Texture the frames are drawn into. Replaced by [`resize`](Self::resize).
    #[must_use]
    pub fn color_view(&self) -> &wgpu::TextureView {
        self.target.color_view()
    }

    /// Sets the size of the target in pixels (zero is treated as one).
    /// Returns `true` if the texture was replaced, so its users must refresh
    /// their references to [`color_view`](Self::color_view).
    pub fn resize(&mut self, width: u32, height: u32) -> bool {
        let (width, height) = (width.max(1), height.max(1));
        if (width, height) == (self.target.width, self.target.height) {
            return false;
        }
        self.target = OffscreenTarget::of_size(self.scene.device(), width, height);
        true
    }

    /// Uploads a triangle list: every 3 vertices form a triangle.
    ///
    /// # Errors
    ///
    /// Returns an error if `vertices` is empty or not a multiple of 3.
    pub fn create_mesh(&mut self, vertices: &[Vertex]) -> EngineResult<MeshId> {
        self.scene.create_mesh(vertices, Topology::Triangles)
    }

    /// Uploads line segments: every 2 vertices form a segment.
    ///
    /// # Errors
    ///
    /// Returns an error if `vertices` is empty or odd.
    pub fn create_lines(&mut self, vertices: &[Vertex]) -> EngineResult<MeshId> {
        self.scene.create_mesh(vertices, Topology::Lines)
    }

    /// Uploads a 3D mesh, to be drawn with a material.
    ///
    /// # Errors
    ///
    /// Same as [`Renderer::create_mesh_3d`](crate::Renderer::create_mesh_3d).
    pub fn create_mesh_3d(&mut self, mesh: &MeshData) -> EngineResult<Mesh3dId> {
        self.scene.create_mesh_3d(mesh)
    }

    /// Uploads `image` to the GPU, to be drawn with `filter`.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Rendering`](star_core::EngineError::Rendering)
    /// if the GPU does not support textures that big.
    pub fn create_texture(
        &mut self,
        image: &Image,
        filter: TextureFilter,
    ) -> EngineResult<TextureId> {
        self.scene.create_texture(image, filter)
    }

    /// Width and height in pixels of the texture `id`, if it is one of
    /// this renderer: what [`PixelRegion`](crate::PixelRegion)s are cut
    /// from.
    #[must_use]
    pub fn texture_size(&self, id: TextureId) -> Option<[u32; 2]> {
        self.scene.texture_size(id)
    }

    /// Draws `frame` into the color texture and submits the work to the queue.
    ///
    /// # Errors
    ///
    /// Same as [`Renderer::render`](crate::Renderer::render).
    pub fn render(&mut self, frame: &Frame) -> EngineResult<()> {
        render_into(&mut self.scene, &self.target, frame)
    }

    /// Draws `frame` into `target`, made for this renderer, and submits the
    /// work to the queue. Frames drawn one after the other into different
    /// targets each keep what they drew.
    ///
    /// # Errors
    ///
    /// Same as [`Renderer::render`](crate::Renderer::render).
    pub fn render_into(&mut self, target: &OffscreenTarget, frame: &Frame) -> EngineResult<()> {
        render_into(&mut self.scene, target, frame)
    }
}

fn render_into(
    scene: &mut SceneRenderer,
    target: &OffscreenTarget,
    frame: &Frame,
) -> EngineResult<()> {
    let aspect_ratio = aspect_ratio(target.width, target.height);
    scene.render_with_depth(
        &target.color_view,
        Some(target.depth.view()),
        aspect_ratio,
        frame,
    )
}

fn create_color_target(device: &wgpu::Device, width: u32, height: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen color target"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&wgpu::TextureViewDescriptor::default())
}

// Texture dimensions are far below 2^24, so converting them to `f32` is exact.
#[allow(clippy::cast_precision_loss)]
fn aspect_ratio(width: u32, height: u32) -> f32 {
    width as f32 / height as f32
}
