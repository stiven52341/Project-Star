//! GPU program that draws the sprites of a frame in batches.

use std::num::NonZeroU64;

use star_core::EngineResult;
use star_math::Mat4;
use wgpu::util::DeviceExt as _;

use crate::depth::DEPTH_FORMAT;
use crate::sprite_batch::{MAX_SPRITES_PER_FRAME, SpriteBatcher, VERTEX_STRIDE, sprite_indices};
use crate::{Color, Sprite, TextureId};

/// Size of the camera's `mat4x4<f32>` in `sprite.wgsl`.
const CAMERA_SIZE: usize = 64;
/// Sprites the buffers hold when first created; they grow when a frame needs more.
const INITIAL_SPRITE_CAPACITY: u32 = 256;
/// Bytes of the four corners of one sprite.
const SPRITE_VERTEX_BYTES: u64 = 4 * VERTEX_STRIDE;

const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x4];

/// Shader, camera and buffers needed to draw sprites.
pub(crate) struct SpritePipeline {
    pipeline: wgpu::RenderPipeline,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    buffers: Option<SpriteBuffers>,
    batcher: SpriteBatcher,
}

/// Vertex and index buffers with room for `capacity` sprites.
struct SpriteBuffers {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    capacity: u32,
}

impl SpritePipeline {
    pub(crate) fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let camera_layout = create_camera_layout(device);
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sprite camera"),
            size: CAMERA_SIZE as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sprite camera"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite pipeline layout"),
            bind_group_layouts: &[Some(&camera_layout), Some(texture_layout)],
            immediate_size: 0,
        });
        Self {
            pipeline: create_pipeline(device, &layout, target_format),
            camera_buffer,
            camera_bind_group,
            buffers: None,
            batcher: SpriteBatcher::default(),
        }
    }

    /// Copies the sprites of a frame and its camera to the GPU.
    ///
    /// # Errors
    ///
    /// Returns [`star_core::EngineError::CapacityExceeded`] if there are too many sprites.
    pub(crate) fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view_projection: Mat4,
        sprites: &[Sprite],
        to_target_space: impl Fn(Color) -> Color,
    ) -> EngineResult<()> {
        self.batcher.prepare(sprites, to_target_space)?;
        if sprites.is_empty() {
            return Ok(());
        }
        queue.write_buffer(&self.camera_buffer, 0, &camera_bytes(view_projection));
        let buffers = buffers_for(&mut self.buffers, device, self.batcher.sprite_count());
        queue.write_buffer(&buffers.vertices, 0, self.batcher.vertices());
        Ok(())
    }

    /// Draws the prepared sprites, finding the bind group of each texture with `texture`.
    ///
    /// # Errors
    ///
    /// Returns the error of `texture` for an unknown texture.
    pub(crate) fn draw<'textures>(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        texture: impl Fn(TextureId) -> EngineResult<&'textures wgpu::BindGroup>,
    ) -> EngineResult<()> {
        let Some(buffers) = &self.buffers else {
            return Ok(());
        };
        if self.batcher.batches().is_empty() {
            return Ok(());
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.camera_bind_group, &[]);
        pass.set_vertex_buffer(0, buffers.vertices.slice(..));
        pass.set_index_buffer(buffers.indices.slice(..), wgpu::IndexFormat::Uint32);
        for batch in self.batcher.batches() {
            pass.set_bind_group(1, texture(batch.texture)?, &[]);
            pass.draw_indexed(batch.indices.clone(), 0, 0..1);
        }
        Ok(())
    }
}

impl SpriteBuffers {
    fn new(device: &wgpu::Device, capacity: u32) -> Self {
        Self {
            vertices: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprite vertices"),
                size: u64::from(capacity) * SPRITE_VERTEX_BYTES,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            indices: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("sprite indices"),
                contents: &sprite_indices(capacity),
                usage: wgpu::BufferUsages::INDEX,
            }),
            capacity,
        }
    }
}

/// The buffers in `slot`, replaced by bigger ones if they cannot hold
/// `sprite_count` sprites. Their size doubles, so growing is rare.
fn buffers_for<'slot>(
    slot: &'slot mut Option<SpriteBuffers>,
    device: &wgpu::Device,
    sprite_count: u32,
) -> &'slot SpriteBuffers {
    if slot
        .as_ref()
        .is_some_and(|buffers| buffers.capacity < sprite_count)
    {
        *slot = None;
    }
    slot.get_or_insert_with(|| {
        let capacity = sprite_count
            .next_power_of_two()
            .clamp(INITIAL_SPRITE_CAPACITY, MAX_SPRITES_PER_FRAME);
        SpriteBuffers::new(device, capacity)
    })
}

/// The camera matrix as `sprite.wgsl` reads it.
fn camera_bytes(view_projection: Mat4) -> [u8; CAMERA_SIZE] {
    let mut bytes = [0; CAMERA_SIZE];
    let values = view_projection.to_columns();
    for (chunk, value) in bytes
        .chunks_exact_mut(size_of::<f32>())
        .zip(values.as_flattened())
    {
        chunk.copy_from_slice(&value.to_le_bytes());
    }
    bytes
}

fn create_camera_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sprite camera"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: NonZeroU64::new(CAMERA_SIZE as u64),
            },
            count: None,
        }],
    })
}

fn create_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    target_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/sprite.wgsl"));
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sprite pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: VERTEX_STRIDE,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &ATTRIBUTES,
            })],
        },
        primitive: wgpu::PrimitiveState {
            // Mirrored sprites (negative sizes) turn around and must stay visible.
            cull_mode: None,
            ..wgpu::PrimitiveState::default()
        },
        // Sprites are hidden by meshes in front of them, but do not hide each
        // other through depth: their layers and order decide which one is on top.
        depth_stencil: Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: target_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_camera_is_stored_column_by_column() {
        let bytes = camera_bytes(Mat4::IDENTITY);

        let first = f32::from_le_bytes(bytes[0..4].try_into().unwrap());
        let second = f32::from_le_bytes(bytes[4..8].try_into().unwrap());
        assert_eq!((first, second), (1.0, 0.0));
    }
}
