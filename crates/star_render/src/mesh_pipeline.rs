//! GPU program that draws colored meshes.

use std::num::NonZeroU64;

use star_core::{EngineError, EngineResult};
use star_math::Mat4;

use crate::bytes::extend_with_f32s;
use crate::depth::DEPTH_FORMAT;
use crate::mesh::{GpuMesh, Topology};
use crate::vertex::vertex_buffer_layout;

/// Draw calls allowed per frame; bounds the size of the transform buffer.
const MAX_DRAWS_PER_FRAME: u32 = 4096;
/// Size of one `mat4x4<f32>` in `mesh.wgsl`.
const TRANSFORM_SIZE: u32 = 64;

/// Shader, GPU state and per-draw transforms needed to draw meshes.
///
/// Every draw gets its own slot in one uniform buffer, selected with a
/// dynamic offset. This works on every backend (including OpenGL) and
/// uploads all transforms of a frame in a single copy.
pub(crate) struct MeshPipeline {
    triangle_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    transform_buffer: wgpu::Buffer,
    transform_bind_group: wgpu::BindGroup,
    transform_stride: u32,
    /// Reused every frame so uploading transforms does not allocate.
    transform_staging: Vec<u8>,
}

impl MeshPipeline {
    pub(crate) fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
        let transform_stride = slot_stride(
            TRANSFORM_SIZE,
            device.limits().min_uniform_buffer_offset_alignment,
        );
        let bind_group_layout = create_bind_group_layout(device);
        let transform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("mesh transforms"),
            size: u64::from(transform_stride) * u64::from(MAX_DRAWS_PER_FRAME),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let transform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("mesh transforms"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &transform_buffer,
                    offset: 0,
                    size: NonZeroU64::new(u64::from(TRANSFORM_SIZE)),
                }),
            }],
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/mesh.wgsl"));
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mesh pipeline layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });
        let pipeline_for = |topology| {
            PipelineParts {
                shader: &shader,
                layout: &layout,
                target_format,
                topology,
            }
            .create(device)
        };
        Self {
            triangle_pipeline: pipeline_for(wgpu::PrimitiveTopology::TriangleList),
            line_pipeline: pipeline_for(wgpu::PrimitiveTopology::LineList),
            transform_buffer,
            transform_bind_group,
            transform_stride,
            transform_staging: Vec::new(),
        }
    }

    /// Copies the transform of every draw of this frame to the GPU, in order.
    pub(crate) fn upload_transforms(
        &mut self,
        queue: &wgpu::Queue,
        transforms: impl ExactSizeIterator<Item = Mat4>,
    ) -> EngineResult<()> {
        if transforms.len() > MAX_DRAWS_PER_FRAME as usize {
            return Err(EngineError::CapacityExceeded {
                resource: "draws per frame",
            });
        }
        let padding = (self.transform_stride - TRANSFORM_SIZE) as usize;
        self.transform_staging.clear();
        for transform in transforms {
            extend_with_f32s(
                &mut self.transform_staging,
                transform.to_columns().as_flattened(),
            );
            self.transform_staging
                .resize(self.transform_staging.len() + padding, 0);
        }
        if !self.transform_staging.is_empty() {
            queue.write_buffer(&self.transform_buffer, 0, &self.transform_staging);
        }
        Ok(())
    }

    /// Draws `mesh` using the transform uploaded in position `slot`.
    pub(crate) fn draw(&self, pass: &mut wgpu::RenderPass<'_>, slot: u32, mesh: &GpuMesh) {
        pass.set_pipeline(match mesh.topology {
            Topology::Triangles => &self.triangle_pipeline,
            Topology::Lines => &self.line_pipeline,
        });
        pass.set_bind_group(
            0,
            &self.transform_bind_group,
            &[slot * self.transform_stride],
        );
        pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        pass.draw(0..mesh.vertex_count, 0..1);
    }
}

/// Smallest multiple of `alignment` that fits `size` bytes.
fn slot_stride(size: u32, alignment: u32) -> u32 {
    size.next_multiple_of(alignment)
}

fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("mesh transforms"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: NonZeroU64::new(u64::from(TRANSFORM_SIZE)),
            },
            count: None,
        }],
    })
}

/// Everything that differs between the pipelines of each topology.
struct PipelineParts<'a> {
    shader: &'a wgpu::ShaderModule,
    layout: &'a wgpu::PipelineLayout,
    target_format: wgpu::TextureFormat,
    topology: wgpu::PrimitiveTopology,
}

impl PipelineParts<'_> {
    fn create(&self, device: &wgpu::Device) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh pipeline"),
            layout: Some(self.layout),
            vertex: wgpu::VertexState {
                module: self.shader,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[Some(vertex_buffer_layout())],
            },
            primitive: wgpu::PrimitiveState {
                topology: self.topology,
                // Both faces are drawn so single triangles stay visible when they turn around.
                cull_mode: None,
                ..wgpu::PrimitiveState::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: Some(true),
                depth_compare: Some(wgpu::CompareFunction::Less),
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: self.shader,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: self.target_format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_stride_rounds_up_to_alignment() {
        assert_eq!(slot_stride(64, 256), 256);
    }

    #[test]
    fn slot_stride_keeps_already_aligned_sizes() {
        assert_eq!(slot_stride(64, 32), 64);
    }
}
