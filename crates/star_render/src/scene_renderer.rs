//! Draws frames into any color target: a window surface or an offscreen texture.

use star_core::{EngineError, EngineResult};
use wgpu::util::DeviceExt as _;

use crate::depth::{DepthBuffer, FARTHEST_DEPTH};
use crate::mesh::{GpuMesh, MeshStore, Topology, primitive_vertex_count};
use crate::mesh_pipeline::MeshPipeline;
use crate::vertex::encode_vertices;
use crate::{Color, Frame, MeshId, Vertex};

/// GPU resources shared by every render target: device, pipelines and meshes.
pub(crate) struct SceneRenderer {
    device: wgpu::Device,
    queue: wgpu::Queue,
    target_format: wgpu::TextureFormat,
    depth_buffer: DepthBuffer,
    mesh_pipeline: MeshPipeline,
    meshes: MeshStore,
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
        Self {
            depth_buffer: DepthBuffer::new(&device, width, height),
            mesh_pipeline: MeshPipeline::new(&device, target_format),
            meshes: MeshStore::default(),
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
        Ok(self.meshes.insert(GpuMesh {
            vertex_buffer,
            vertex_count,
            topology,
        }))
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
        let commands = self.encode(target, frame)?;
        self.queue.submit([commands]);
        Ok(())
    }

    /// sRGB render targets expect linear colors; the GPU re-encodes them.
    pub(crate) fn to_target_space(&self, color: Color) -> Color {
        if self.target_format.is_srgb() {
            color.to_linear()
        } else {
            color
        }
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
            for (slot, draw) in (0..).zip(frame.draws()) {
                let mesh = self.meshes.get(draw.mesh)?;
                self.mesh_pipeline.draw(&mut pass, slot, mesh);
            }
        }
        Ok(encoder.finish())
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
