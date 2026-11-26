//! The joint matrices of skinned 3D draws on the GPU (decision 83): one
//! slot of a uniform buffer for each skinned draw of a frame, selected
//! with a dynamic offset, as the other per-draw uniforms are, so every
//! backend (OpenGL included) runs it.

use std::num::NonZeroU64;

use star_core::{EngineError, EngineResult};
use star_math::Mat4;

use crate::bytes::extend_with_f32s;
use crate::mesh_pipeline::slot_stride;

/// The most joints a skinned draw moves; as `MAX_JOINTS` of `surface.wgsl`.
pub const MAX_JOINTS: usize = 128;
/// Skinned draws allowed per frame; bounds the size of the joint buffer.
const MAX_SKINNED_DRAWS_PER_FRAME: u32 = 64;
/// Size of one `mat4x4<f32>`.
const MATRIX_SIZE: u32 = 64;
/// Size of the `Joints` struct of `surface.wgsl`.
#[expect(
    clippy::cast_possible_truncation,
    reason = "128 matrices of 64 bytes fit in a u32"
)]
const JOINTS_SIZE: u32 = MAX_JOINTS as u32 * MATRIX_SIZE;

/// The joint matrices of the skinned draws of a frame.
pub(crate) struct JointBuffer {
    layout: wgpu::BindGroupLayout,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    stride: u32,
    /// Reused every frame so uploading joints does not allocate.
    staging: Vec<u8>,
}

impl JointBuffer {
    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let stride = slot_stride(
            JOINTS_SIZE,
            device.limits().min_uniform_buffer_offset_alignment,
        );
        let size = NonZeroU64::new(u64::from(JOINTS_SIZE));
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3D joints"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: size,
                },
                count: None,
            }],
        });
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3D joints"),
            size: u64::from(stride) * u64::from(MAX_SKINNED_DRAWS_PER_FRAME),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3D joints"),
            layout: &layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &buffer,
                    offset: 0,
                    size,
                }),
            }],
        });
        Self {
            layout,
            buffer,
            bind_group,
            stride,
            staging: Vec::new(),
        }
    }

    /// How the joints are bound, for the skinned pipeline's layout.
    pub(crate) const fn layout(&self) -> &wgpu::BindGroupLayout {
        &self.layout
    }

    /// Copies the joint matrices of every skinned draw of this frame to the
    /// GPU, each in its slot, in order.
    pub(crate) fn upload<'a>(
        &mut self,
        queue: &wgpu::Queue,
        draws: impl Iterator<Item = &'a [Mat4]>,
    ) -> EngineResult<()> {
        self.staging.clear();
        let slot = self.stride as usize;
        for (index, joints) in draws.enumerate() {
            if index >= MAX_SKINNED_DRAWS_PER_FRAME as usize {
                return Err(EngineError::CapacityExceeded {
                    resource: "skinned 3D draws per frame",
                });
            }
            for joint in joints {
                extend_with_f32s(&mut self.staging, joint.to_columns().as_flattened());
            }
            self.staging.resize((index + 1) * slot, 0);
        }
        if !self.staging.is_empty() {
            queue.write_buffer(&self.buffer, 0, &self.staging);
        }
        Ok(())
    }

    /// Binds the joints of the skinned draw in position `slot`.
    pub(crate) fn bind(&self, pass: &mut wgpu::RenderPass<'_>, group: u32, slot: u32) {
        pass.set_bind_group(group, &self.bind_group, &[slot * self.stride]);
    }
}
