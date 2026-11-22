//! Vertex format shared by the CPU and the GPU shaders.

use star_math::Vec3;

use crate::Color;
use crate::bytes::extend_with_f32s;

/// A corner of a triangle, with its own color.
///
/// Colors are blended smoothly across the triangle between its corners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vertex {
    /// Location in the mesh's local space.
    pub position: Vec3,
    /// Color at this corner.
    pub color: Color,
}

impl Vertex {
    /// Creates a vertex at `position` with `color`.
    #[must_use]
    pub const fn new(position: Vec3, color: Color) -> Self {
        Self { position, color }
    }
}

/// Position (3 floats) followed by color (4 floats), matching `mesh.wgsl`.
const FLOATS_PER_VERTEX: usize = 7;
const BYTES_PER_FLOAT: usize = 4;
const VERTEX_STRIDE: usize = FLOATS_PER_VERTEX * BYTES_PER_FLOAT;

const ATTRIBUTES: [wgpu::VertexAttribute; 2] =
    wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x4];

/// How the GPU reads [`Vertex`] data from a vertex buffer.
pub(crate) const fn vertex_buffer_layout() -> wgpu::VertexBufferLayout<'static> {
    wgpu::VertexBufferLayout {
        array_stride: VERTEX_STRIDE as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &ATTRIBUTES,
    }
}

/// Converts vertices to GPU bytes, mapping each color with `to_target_space`.
pub(crate) fn encode_vertices(
    vertices: &[Vertex],
    to_target_space: impl Fn(Color) -> Color,
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vertices.len() * VERTEX_STRIDE);
    for vertex in vertices {
        let Vec3 { x, y, z } = vertex.position;
        let Color { r, g, b, a } = to_target_space(vertex.color);
        extend_with_f32s(&mut bytes, &[x, y, z, r, g, b, a]);
    }
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_vertex_occupies_one_stride() {
        let vertex = Vertex::new(Vec3::ZERO, Color::WHITE);

        let bytes = encode_vertices(&[vertex, vertex], |color| color);

        assert_eq!(bytes.len(), 2 * VERTEX_STRIDE);
    }

    #[test]
    fn colors_are_converted_to_target_space() {
        let vertex = Vertex::new(Vec3::ZERO, Color::WHITE);

        let bytes = encode_vertices(&[vertex], |_| Color::BLACK);

        assert!(
            bytes[3 * BYTES_PER_FLOAT..]
                .iter()
                .take(12)
                .all(|&byte| byte == 0)
        );
    }
}
