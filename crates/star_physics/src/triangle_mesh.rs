//! The triangles a 3D body with a [`mesh`](crate::Shape3d::mesh) shape
//! collides with: those of the mesh or model its node shows.

use star_core::{EngineError, EngineResult};
use star_math::Vec3;

/// The surface of a mesh as the physics sees it: its corners and the
/// triangles between them, in the space of its node (not scaled by it).
///
/// Built through [`TriangleMesh::new`], which checks that every triangle
/// names corners it has and every corner is a finite point.
#[derive(Debug, Clone, PartialEq)]
pub struct TriangleMesh {
    corners: Vec<Vec3>,
    triangles: Vec<[u32; 3]>,
}

impl TriangleMesh {
    /// The surface made of `triangles`, each three places in `corners`.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`] if there are no triangles, a corner is not
    /// finite, or a triangle names a corner that is not there.
    pub fn new(corners: Vec<Vec3>, triangles: Vec<[u32; 3]>) -> EngineResult<Self> {
        if triangles.is_empty() {
            return Err(EngineError::Physics(
                "a mesh body needs at least one triangle".to_owned(),
            ));
        }
        if let Some(corner) = corners.iter().find(|corner| {
            ![corner.x, corner.y, corner.z]
                .iter()
                .all(|part| part.is_finite())
        }) {
            return Err(EngineError::Physics(format!(
                "the corner {corner:?} of a mesh body is not a finite point"
            )));
        }
        let count = corners.len();
        if let Some(triangle) = triangles.iter().find(|triangle| {
            triangle
                .iter()
                .any(|&corner| usize::try_from(corner).map_or(true, |corner| corner >= count))
        }) {
            return Err(EngineError::Physics(format!(
                "the triangle {triangle:?} of a mesh body names corners it does not have ({count})"
            )));
        }
        Ok(Self { corners, triangles })
    }

    /// Its corners, in its node's space.
    #[must_use]
    pub fn corners(&self) -> &[Vec3] {
        &self.corners
    }

    /// Its triangles, as places in [`corners`](Self::corners).
    #[must_use]
    pub fn triangles(&self) -> &[[u32; 3]] {
        &self.triangles
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corners() -> Vec<Vec3> {
        vec![
            Vec3::ZERO,
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(0.0, 1.0, 0.0),
        ]
    }

    #[test]
    fn triangles_over_their_corners_make_a_mesh() {
        let mesh = TriangleMesh::new(corners(), vec![[0, 1, 2]]).unwrap();

        assert_eq!(mesh.corners().len(), 3);
        assert_eq!(mesh.triangles(), &[[0, 1, 2]]);
    }

    #[test]
    fn broken_meshes_are_rejected() {
        let mut far = corners();
        far[1].x = f32::INFINITY;

        assert!(TriangleMesh::new(corners(), Vec::new()).is_err());
        assert!(TriangleMesh::new(corners(), vec![[0, 1, 3]]).is_err());
        assert!(TriangleMesh::new(far, vec![[0, 1, 2]]).is_err());
    }
}
