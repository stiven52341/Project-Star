//! GPU program that draws 3D meshes with their materials.

use std::num::NonZeroU64;

use star_core::{EngineError, EngineResult};
use star_math::{Mat4, Vec2, Vec3};
use wgpu::util::DeviceExt as _;

use crate::bytes::extend_with_f32s;
use crate::depth::DEPTH_FORMAT;
use crate::light::{LIGHTS_SIZE, encode_lights};
use crate::mesh_pipeline::slot_stride;
use crate::{AmbientLight, Color, Light, MAX_LIGHTS, MeshData, MeshVertex};

/// 3D draws allowed per frame; bounds the size of the draw buffer.
const MAX_DRAWS_PER_FRAME: u32 = 4096;
/// Size of the `Draw` struct of `surface.wgsl`: two `mat4x4<f32>`, a
/// `mat3x3<f32>` (three padded columns) and two `vec4<f32>`.
const DRAW_SIZE: u32 = 208;
/// Position (3 floats), normal (3) and texture coordinates (2).
const FLOATS_PER_VERTEX: usize = 8;
const BYTES_PER_FLOAT: usize = 4;
const VERTEX_STRIDE: usize = FLOATS_PER_VERTEX * BYTES_PER_FLOAT;

const ATTRIBUTES: [wgpu::VertexAttribute; 3] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x3, 2 => Float32x2];

/// The buffers of a 3D mesh, ready to be drawn.
pub(crate) struct GpuSurfaceMesh {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    index_count: u32,
}

impl GpuSurfaceMesh {
    /// Uploads `mesh` to the GPU.
    pub(crate) fn upload(device: &wgpu::Device, mesh: &MeshData) -> EngineResult<Self> {
        let index_count =
            u32::try_from(mesh.indices().len()).map_err(|_| EngineError::CapacityExceeded {
                resource: "mesh indices",
            })?;
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3D mesh vertices"),
            contents: &encode_vertices(mesh.vertices()),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices: Vec<u8> = mesh
            .indices()
            .iter()
            .flat_map(|index| index.to_le_bytes())
            .collect();
        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("3D mesh indices"),
            contents: &indices,
            usage: wgpu::BufferUsages::INDEX,
        });
        Ok(Self {
            vertex_buffer,
            index_buffer,
            index_count,
        })
    }
}

/// What the GPU needs to draw one 3D mesh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SurfaceUniforms {
    pub(crate) model_view_projection: Mat4,
    pub(crate) model: Mat4,
    /// Already in the target's color space.
    pub(crate) base_color: Color,
    pub(crate) metallic: f32,
    pub(crate) roughness: f32,
}

/// The lights of a frame, as the GPU needs them.
pub(crate) struct FrameLights<'a> {
    pub(crate) camera_position: Vec3,
    pub(crate) ambient: AmbientLight,
    pub(crate) lights: &'a [Light],
}

/// Shader, GPU state and per-draw uniforms needed to draw 3D meshes.
///
/// As with colored meshes, every draw gets its own slot of one uniform
/// buffer, selected with a dynamic offset.
pub(crate) struct SurfacePipeline {
    pipeline: wgpu::RenderPipeline,
    draw_buffer: wgpu::Buffer,
    draw_bind_group: wgpu::BindGroup,
    draw_stride: u32,
    lights_buffer: wgpu::Buffer,
    lights_bind_group: wgpu::BindGroup,
    /// Reused every frame so uploading draws and lights does not allocate.
    draw_staging: Vec<u8>,
    lights_staging: Vec<u8>,
}

impl SurfacePipeline {
    /// Prepares to draw into targets of `target_format`, with textures
    /// bound as `texture_layout` says.
    pub(crate) fn new(
        device: &wgpu::Device,
        target_format: wgpu::TextureFormat,
        texture_layout: &wgpu::BindGroupLayout,
    ) -> Self {
        let draw_stride = slot_stride(
            DRAW_SIZE,
            device.limits().min_uniform_buffer_offset_alignment,
        );
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3D draws"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: NonZeroU64::new(u64::from(DRAW_SIZE)),
                },
                count: None,
            }],
        });
        let draw_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3D draws"),
            size: u64::from(draw_stride) * u64::from(MAX_DRAWS_PER_FRAME),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let draw_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3D draws"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &draw_buffer,
                    offset: 0,
                    size: NonZeroU64::new(u64::from(DRAW_SIZE)),
                }),
            }],
        });
        let lights_size = NonZeroU64::new(LIGHTS_SIZE as u64);
        let lights_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("3D lights"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: lights_size,
                },
                count: None,
            }],
        });
        let lights_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("3D lights"),
            size: LIGHTS_SIZE as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let lights_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("3D lights"),
            layout: &lights_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: lights_buffer.as_entire_binding(),
            }],
        });
        let shader = device.create_shader_module(wgpu::include_wgsl!("shaders/surface.wgsl"));
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("3D pipeline layout"),
            bind_group_layouts: &[
                Some(&bind_group_layout),
                Some(&lights_layout),
                Some(texture_layout),
            ],
            immediate_size: 0,
        });
        Self {
            pipeline: create_pipeline(device, &shader, &layout, target_format),
            draw_buffer,
            draw_bind_group,
            draw_stride,
            lights_buffer,
            lights_bind_group,
            draw_staging: Vec::new(),
            lights_staging: Vec::new(),
        }
    }

    /// Copies the lights of this frame to the GPU, with their colors mapped
    /// by `to_target_space`.
    pub(crate) fn upload_lights(
        &mut self,
        queue: &wgpu::Queue,
        frame: &FrameLights<'_>,
        to_target_space: impl Fn(Color) -> Color,
    ) -> EngineResult<()> {
        if frame.lights.len() > MAX_LIGHTS {
            return Err(EngineError::CapacityExceeded {
                resource: "lights per frame",
            });
        }
        self.lights_staging.clear();
        encode_lights(
            &mut self.lights_staging,
            frame.camera_position,
            frame.ambient,
            frame.lights,
            to_target_space,
        );
        queue.write_buffer(&self.lights_buffer, 0, &self.lights_staging);
        Ok(())
    }

    /// Copies the uniforms of every 3D draw of this frame to the GPU, in
    /// order.
    pub(crate) fn upload_draws(
        &mut self,
        queue: &wgpu::Queue,
        draws: impl ExactSizeIterator<Item = SurfaceUniforms>,
    ) -> EngineResult<()> {
        if draws.len() > MAX_DRAWS_PER_FRAME as usize {
            return Err(EngineError::CapacityExceeded {
                resource: "3D draws per frame",
            });
        }
        let padding = (self.draw_stride - DRAW_SIZE) as usize;
        self.draw_staging.clear();
        for draw in draws {
            encode_uniforms(&mut self.draw_staging, &draw);
            self.draw_staging
                .resize(self.draw_staging.len() + padding, 0);
        }
        if !self.draw_staging.is_empty() {
            queue.write_buffer(&self.draw_buffer, 0, &self.draw_staging);
        }
        Ok(())
    }

    /// Draws `mesh` with the uniforms uploaded in position `slot`, showing
    /// `texture`.
    pub(crate) fn draw(
        &self,
        pass: &mut wgpu::RenderPass<'_>,
        slot: u32,
        mesh: &GpuSurfaceMesh,
        texture: &wgpu::BindGroup,
    ) {
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.draw_bind_group, &[slot * self.draw_stride]);
        pass.set_bind_group(1, &self.lights_bind_group, &[]);
        pass.set_bind_group(2, texture, &[]);
        pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
        pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..mesh.index_count, 0, 0..1);
    }
}

fn encode_uniforms(bytes: &mut Vec<u8>, draw: &SurfaceUniforms) {
    extend_with_f32s(
        bytes,
        draw.model_view_projection.to_columns().as_flattened(),
    );
    extend_with_f32s(bytes, draw.model.to_columns().as_flattened());
    // Normals follow the inverse transpose, so non-uniform scales keep them
    // perpendicular to their surface; a flattened mesh has no inside to
    // light, and keeps the model's own.
    let normal_matrix = draw
        .model
        .inverse()
        .map_or(draw.model, Mat4::transposed)
        .to_columns();
    for column in &normal_matrix[..3] {
        extend_with_f32s(bytes, &[column[0], column[1], column[2], 0.0]);
    }
    let Color { r, g, b, a } = draw.base_color;
    extend_with_f32s(
        bytes,
        &[r, g, b, a, draw.metallic, draw.roughness, 0.0, 0.0],
    );
}

fn encode_vertices(vertices: &[MeshVertex]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vertices.len() * VERTEX_STRIDE);
    for vertex in vertices {
        let (Vec3 { x, y, z }, normal, Vec2 { x: u, y: v }) =
            (vertex.position, vertex.normal, vertex.uv);
        extend_with_f32s(&mut bytes, &[x, y, z, normal.x, normal.y, normal.z, u, v]);
    }
    bytes
}

fn create_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    layout: &wgpu::PipelineLayout,
    target_format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("3D pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[Some(wgpu::VertexBufferLayout {
                array_stride: VERTEX_STRIDE as wgpu::BufferAddress,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &ATTRIBUTES,
            })],
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            front_face: wgpu::FrontFace::Ccw,
            // Closed meshes only show the faces that look at the camera.
            cull_mode: Some(wgpu::Face::Back),
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
            module: shader,
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
    fn each_vertex_occupies_one_stride() {
        let mesh = MeshData::cube(Vec3::ONE).unwrap();

        assert_eq!(encode_vertices(mesh.vertices()).len(), 24 * VERTEX_STRIDE);
    }

    #[test]
    fn uniforms_fill_the_draw_struct() {
        let mut bytes = Vec::new();

        encode_uniforms(
            &mut bytes,
            &SurfaceUniforms {
                model_view_projection: Mat4::IDENTITY,
                model: Mat4::from_scale(Vec3::new(0.0, 1.0, 1.0)),
                base_color: Color::WHITE,
                metallic: 0.0,
                roughness: 0.5,
            },
        );

        assert_eq!(bytes.len(), DRAW_SIZE as usize);
    }
}
