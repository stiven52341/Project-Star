//! The triangles a node's 3D body collides with when its shape is a
//! [`mesh`](star_physics::Shape3d::mesh) (decision 84): those of the mesh
//! or model the node shows.

use std::sync::Arc;

use star_core::{EngineResult, Model};
use star_math::{Mat4, Vec3};
use star_physics::TriangleMesh;
use star_render::MeshData;

/// The triangles of what a 3D node shows, in its space, for its body with
/// a mesh shape. The engine gives it to nodes with a [`MeshNode`] of a
/// shape or a [`ModelNode`] once their model is read; a game that shows a
/// mesh of its own gives it with [`CollisionMesh::from_mesh`]. A body with
/// a mesh shape waits for it before it is added.
///
/// [`MeshNode`]: crate::MeshNode
/// [`ModelNode`]: crate::ModelNode
#[derive(Debug, Clone, PartialEq)]
pub struct CollisionMesh(pub Arc<TriangleMesh>);

impl CollisionMesh {
    /// The triangles of `mesh`.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`](star_core::EngineError::Physics) if it has
    /// no triangles.
    pub fn from_mesh(mesh: &MeshData) -> EngineResult<Self> {
        let corners = mesh
            .vertices()
            .iter()
            .map(|vertex| vertex.position)
            .collect();
        let triangles = mesh
            .indices()
            .chunks_exact(3)
            .map(|triangle| [triangle[0], triangle[1], triangle[2]])
            .collect();
        Ok(Self(Arc::new(TriangleMesh::new(corners, triangles)?)))
    }

    /// The triangles of every part of `model`, where the model puts it; a
    /// part a skeleton bends, as it is at rest.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`](star_core::EngineError::Physics) if the
    /// model has no triangles or too many corners to count.
    pub fn from_model(model: &Model) -> EngineResult<Self> {
        let mut corners = Vec::new();
        let mut triangles = Vec::new();
        for part in model.parts() {
            let first = u32::try_from(corners.len()).map_err(|_| too_big())?;
            // Skinned vertices are already where the skeleton at rest
            // takes them; the part's place is not theirs.
            let place = if part.skinned {
                Mat4::IDENTITY
            } else {
                Mat4::from_columns(part.transform)
            };
            corners.extend(part.vertices.iter().map(|vertex| {
                let [x, y, z] = vertex.position;
                place.transform_point(Vec3::new(x, y, z))
            }));
            for triangle in part.indices.chunks_exact(3) {
                let corner = |index: usize| first.checked_add(triangle[index]).ok_or_else(too_big);
                triangles.push([corner(0)?, corner(1)?, corner(2)?]);
            }
        }
        Ok(Self(Arc::new(TriangleMesh::new(corners, triangles)?)))
    }
}

fn too_big() -> star_core::EngineError {
    star_core::EngineError::Physics("a model has too many corners for a mesh body".to_owned())
}

#[cfg(test)]
mod tests {
    use star_core::model::IDENTITY_COLUMNS;
    use star_core::{ModelMaterial, ModelPart, ModelVertex};
    use star_render::MeshShape;

    use super::*;

    fn triangle(transform: [[f32; 4]; 4], skinned: bool) -> ModelPart {
        let vertex = |x, y| ModelVertex::new([x, y, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0]);
        ModelPart {
            vertices: vec![vertex(0.0, 0.0), vertex(1.0, 0.0), vertex(0.0, 1.0)],
            indices: vec![0, 1, 2],
            material: ModelMaterial::default(),
            transform,
            skinned,
        }
    }

    #[test]
    fn meshes_give_their_triangles() {
        let cube = MeshShape::Cube { size: Vec3::ONE }.to_mesh_data().unwrap();

        let mesh = CollisionMesh::from_mesh(&cube).unwrap();

        assert_eq!(mesh.0.triangles().len(), cube.indices().len() / 3);
        assert_eq!(mesh.0.corners().len(), cube.vertices().len());
    }

    #[test]
    fn models_give_every_part_where_they_put_it() {
        let moved = Mat4::from_translation(Vec3::new(0.0, 0.0, 5.0)).to_columns();
        let model = Model::new(
            vec![
                triangle(IDENTITY_COLUMNS, false),
                triangle(moved, false),
                triangle(moved, true),
            ],
            Vec::new(),
        )
        .unwrap();

        let mesh = CollisionMesh::from_model(&model).unwrap();

        assert_eq!(mesh.0.triangles(), &[[0, 1, 2], [3, 4, 5], [6, 7, 8]]);
        assert_eq!(mesh.0.corners()[3], Vec3::new(0.0, 0.0, 5.0));
        assert_eq!(
            mesh.0.corners()[6],
            Vec3::ZERO,
            "skinned parts stay at rest"
        );
    }
}
