//! Draws frames into any color target: a window surface or an offscreen texture.

use star_core::{EngineError, EngineResult};
use wgpu::util::DeviceExt as _;

use crate::depth::{DepthBuffer, FARTHEST_DEPTH};
use crate::mesh::{GpuMesh, Topology, primitive_vertex_count};
use crate::mesh_pipeline::MeshPipeline;
use crate::sprite_pipeline::SpritePipeline;
use crate::store::Store;
use crate::surface_pipeline::{GpuSurfaceMesh, SurfacePipeline, SurfaceUniforms};
use crate::texture::{GpuTexture, TextureBindings};
use crate::vertex::encode_vertices;
use crate::{Color, Frame, Image, Mesh3dId, MeshData, MeshId, TextureFilter, TextureId, Vertex};

/// GPU resources shared by every render target: device, pipelines, meshes
/// and textures.
pub(crate) struct SceneRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    target_format: wgpu::TextureFormat,
    depth_buffer: DepthBuffer,
    mesh_pipeline: MeshPipeline,
    surface_pipeline: SurfacePipeline,
    sprite_pipeline: SpritePipeline,
    texture_bindings: TextureBindings,
    meshes: Store<GpuMesh>,
    surface_meshes: Store<GpuSurfaceMesh>,
    textures: Store<GpuTexture>,
}

impl SceneRenderer {
    /// Prepares to draw into `width` x `height` targets of `target_format`.
    pub(crate) fn new(
        device: wgpu::Device,
        queue: wgpu::Queue,
        target_format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let texture_bindings = TextureBindings::new(&device, target_format);
        Self {
            depth_buffer: DepthBuffer::new(&device, width, height),
            mesh_pipeline: MeshPipeline::new(&device, target_format),
            surface_pipeline: SurfacePipeline::new(&device, target_format),
            sprite_pipeline: SpritePipeline::new(&device, target_format, texture_bindings.layout()),
            texture_bindings,
            meshes: Store::default(),
            surface_meshes: Store::default(),
            textures: Store::default(),
            device,
            queue,
            target_format,
        }
    }

    pub(crate) fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub(crate) fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Matches the depth buffer to a new target size.
    pub(crate) fn resize(&mut self, width: u32, height: u32) {
        self.depth_buffer = DepthBuffer::new(&self.device, width, height);
    }

    /// Uploads `vertices`, grouped as `topology`, to the GPU.
    pub(crate) fn create_mesh(
        &mut self,
        vertices: &[Vertex],
        topology: Topology,
    ) -> EngineResult<MeshId> {
        let vertex_count = primitive_vertex_count(vertices.len(), topology)?;
        let contents = encode_vertices(vertices, |color| self.to_target_space(color));
        let vertex_buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("mesh vertices"),
                contents: &contents,
                usage: wgpu::BufferUsages::VERTEX,
            });
        Ok(MeshId(self.meshes.insert(GpuMesh {
            vertex_buffer,
            vertex_count,
            topology,
        })))
    }

    /// Uploads the 3D `mesh` to the GPU.
    pub(crate) fn create_mesh_3d(&mut self, mesh: &MeshData) -> EngineResult<Mesh3dId> {
        let uploaded = GpuSurfaceMesh::upload(&self.device, mesh)?;
        Ok(Mesh3dId(self.surface_meshes.insert(uploaded)))
    }

    /// Uploads `image` to the GPU, to be drawn with `filter`.
    pub(crate) fn create_texture(
        &mut self,
        image: &Image,
        filter: TextureFilter,
    ) -> EngineResult<TextureId> {
        let texture = self
            .texture_bindings
            .upload(&self.device, &self.queue, image, filter)?;
        Ok(TextureId(self.textures.insert(texture)))
    }

    /// Changes the image of the texture `id`, which keeps being drawn by the
    /// sprites that use it.
    pub(crate) fn replace_texture(
        &mut self,
        id: TextureId,
        image: &Image,
        filter: TextureFilter,
    ) -> EngineResult<()> {
        self.texture(id)?;
        let texture = self
            .texture_bindings
            .upload(&self.device, &self.queue, image, filter)?;
        self.textures.replace(id.0, texture);
        Ok(())
    }

    /// Draws `frame` into `target`, whose width divided by height is `aspect_ratio`,
    /// and submits the work to the GPU.
    pub(crate) fn render(
        &mut self,
        target: &wgpu::TextureView,
        aspect_ratio: f32,
        frame: &Frame,
    ) -> EngineResult<()> {
        let view_projection = frame.camera().view_projection(aspect_ratio).ok_or_else(|| {
            EngineError::Rendering(
                "the camera needs distinct position and target, and must not look straight up or down"
                    .to_owned(),
            )
        })?;
        self.mesh_pipeline.upload_transforms(
            &self.queue,
            frame
                .draws()
                .iter()
                .map(|draw| view_projection * draw.transform),
        )?;
        let target_format = self.target_format;
        self.surface_pipeline.upload_draws(
            &self.queue,
            frame.surfaces().iter().map(|draw| SurfaceUniforms {
                model_view_projection: view_projection * draw.transform,
                base_color: draw.material.base_color.in_target_space(target_format),
            }),
        )?;
        self.sprite_pipeline.prepare(
            &self.device,
            &self.queue,
            view_projection,
            frame.sprites(),
            |color| color.in_target_space(target_format),
        )?;
        let commands = self.encode(target, frame)?;
        self.queue.submit([commands]);
        Ok(())
    }

    /// sRGB render targets expect linear colors; the GPU re-encodes them.
    pub(crate) fn to_target_space(&self, color: Color) -> Color {
        color.in_target_space(self.target_format)
    }

    fn encode(
        &self,
        target: &wgpu::TextureView,
        frame: &Frame,
    ) -> EngineResult<wgpu::CommandBuffer> {
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("frame encoder"),
            });
        {
            let mut pass = self.begin_main_pass(&mut encoder, target, frame.clear_color());
            for (slot, draw) in (0..).zip(frame.surfaces()) {
                let mesh = self.mesh_3d(draw.mesh)?;
                self.surface_pipeline.draw(&mut pass, slot, mesh);
            }
            for (slot, draw) in (0..).zip(frame.draws()) {
                let mesh = self.mesh(draw.mesh)?;
                self.mesh_pipeline.draw(&mut pass, slot, mesh);
            }
            self.sprite_pipeline.draw(&mut pass, |id| {
                self.texture(id).map(|texture| &texture.bind_group)
            })?;
        }
        Ok(encoder.finish())
    }

    fn mesh(&self, id: MeshId) -> EngineResult<&GpuMesh> {
        self.meshes
            .get(id.0)
            .ok_or_else(|| EngineError::Rendering(format!("unknown mesh {id:?}")))
    }

    fn mesh_3d(&self, id: Mesh3dId) -> EngineResult<&GpuSurfaceMesh> {
        self.surface_meshes
            .get(id.0)
            .ok_or_else(|| EngineError::Rendering(format!("unknown 3D mesh {id:?}")))
    }

    fn texture(&self, id: TextureId) -> EngineResult<&GpuTexture> {
        self.textures
            .get(id.0)
            .ok_or_else(|| EngineError::Rendering(format!("unknown texture {id:?}")))
    }

    fn begin_main_pass<'encoder>(
        &self,
        encoder: &'encoder mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        clear_color: Color,
    ) -> wgpu::RenderPass<'encoder> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("main pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(self.to_target_space(clear_color).to_wgpu()),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: self.depth_buffer.view(),
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(FARTHEST_DEPTH),
                    store: wgpu::StoreOp::Discard,
                }),
                stencil_ops: None,
            }),
            ..Default::default()
        })
    }
}
