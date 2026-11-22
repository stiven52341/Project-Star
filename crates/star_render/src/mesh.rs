//! Geometry stored on the GPU.

use star_core::{EngineError, EngineResult};

/// How the vertices of a mesh are grouped into primitives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Topology {
    /// Every 3 vertices form a filled triangle.
    Triangles,
    /// Every 2 vertices form a line segment.
    Lines,
}

impl Topology {
    const fn vertices_per_primitive(self) -> usize {
        match self {
            Self::Triangles => 3,
            Self::Lines => 2,
        }
    }
}

/// Handle to a mesh uploaded with [`Renderer::create_mesh`](crate::Renderer::create_mesh)
/// or [`Renderer::create_lines`](crate::Renderer::create_lines).
///
/// Only valid for the renderer that created it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MeshId(pub(crate) usize);

/// Vertex buffer of a mesh, ready to be drawn.
pub(crate) struct GpuMesh {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) vertex_count: u32,
    pub(crate) topology: Topology,
}

/// Checks that `vertex_count` describes whole primitives of `topology` and
/// returns it as the GPU's draw count.
pub(crate) fn primitive_vertex_count(vertex_count: usize, topology: Topology) -> EngineResult<u32> {
    let per_primitive = topology.vertices_per_primitive();
    if vertex_count == 0 || vertex_count % per_primitive != 0 {
        return Err(EngineError::Rendering(format!(
            "{topology:?} need a non-zero multiple of {per_primitive} vertices, got {vertex_count}"
        )));
    }
    u32::try_from(vertex_count).map_err(|_| EngineError::CapacityExceeded {
        resource: "mesh vertices",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whole_triangles_are_accepted() {
        assert_eq!(primitive_vertex_count(6, Topology::Triangles), Ok(6));
    }

    #[test]
    fn empty_meshes_are_rejected() {
        assert!(primitive_vertex_count(0, Topology::Lines).is_err());
    }

    #[test]
    fn partial_triangles_are_rejected() {
        assert!(primitive_vertex_count(4, Topology::Triangles).is_err());
    }

    #[test]
    fn lines_need_vertex_pairs() {
        assert_eq!(primitive_vertex_count(4, Topology::Lines), Ok(4));
        assert!(primitive_vertex_count(3, Topology::Lines).is_err());
    }
}
