//! 3D meshes in memory: corners shared by indexed triangles, with the
//! normals and texture coordinates that lighting and textures need.

use std::f32::consts::{PI, TAU};

use star_core::{EngineError, EngineResult, ModelPart};
use star_math::{Vec2, Vec3};

/// The fewest segments around a sphere: fewer would not enclose anything.
pub const MIN_SPHERE_SEGMENTS: u32 = 3;
/// The fewest rings from pole to pole of a sphere.
pub const MIN_SPHERE_RINGS: u32 = 2;
/// The most segments or rings of a sphere, so its corners always fit in the
/// indices.
pub const MAX_SPHERE_DIVISIONS: u32 = 1024;

/// Segments around a sphere when none are given: round enough up close.
pub const DEFAULT_SPHERE_SEGMENTS: u32 = 32;
/// Rings from pole to pole of a sphere when none are given.
pub const DEFAULT_SPHERE_RINGS: u32 = 16;

const VERTICES_PER_TRIANGLE: usize = 3;

/// A mesh described by its shape instead of its corners, as scene files
/// keep it: the same shape always builds the same mesh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MeshShape {
    /// See [`MeshData::cube`].
    Cube {
        /// Width, height and depth.
        size: Vec3,
    },
    /// See [`MeshData::plane`].
    Plane {
        /// Width (along x) and depth (along z).
        size: Vec2,
    },
    /// See [`MeshData::sphere`].
    Sphere {
        /// Distance from the center to the surface.
        radius: f32,
        /// Divisions around it.
        segments: u32,
        /// Divisions from pole to pole.
        rings: u32,
    },
}

impl MeshShape {
    /// The mesh of this shape.
    ///
    /// # Errors
    ///
    /// [`EngineError::Rendering`] if its sizes are not valid (see each
    /// constructor of [`MeshData`]).
    pub fn to_mesh_data(self) -> EngineResult<MeshData> {
        match self {
            Self::Cube { size } => MeshData::cube(size),
            Self::Plane { size } => MeshData::plane(size),
            Self::Sphere {
                radius,
                segments,
                rings,
            } => MeshData::sphere(radius, segments, rings),
        }
    }
}

/// A corner of a 3D mesh.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshVertex {
    /// Where it is, in the mesh's own space.
    pub position: Vec3,
    /// The direction the surface faces there, of length 1.
    pub normal: Vec3,
    /// Where it is on a texture: `(0, 0)` is the top-left corner of the
    /// image and `(1, 1)` the bottom-right one.
    pub uv: Vec2,
}

impl MeshVertex {
    /// A corner at `position`, facing `normal`, at `uv` on a texture.
    #[must_use]
    pub const fn new(position: Vec3, normal: Vec3, uv: Vec2) -> Self {
        Self {
            position,
            normal,
            uv,
        }
    }
}

/// A 3D mesh: corners and the triangles between them, every 3 indices one
/// triangle. Triangles whose corners are counterclockwise seen from a side
/// show that side: the other one is not drawn, so closed meshes only draw
/// what faces the camera.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshData {
    vertices: Vec<MeshVertex>,
    indices: Vec<u32>,
}

impl MeshData {
    /// A mesh of `vertices`, joined into triangles by `indices`.
    ///
    /// # Errors
    ///
    /// [`EngineError::Rendering`] if there are no triangles, the indices do
    /// not make whole triangles, an index has no vertex, or a vertex is not
    /// made of finite numbers.
    pub fn new(vertices: Vec<MeshVertex>, indices: Vec<u32>) -> EngineResult<Self> {
        if indices.is_empty() || indices.len() % VERTICES_PER_TRIANGLE != 0 {
            return Err(EngineError::Rendering(format!(
                "a mesh needs a non-zero multiple of {VERTICES_PER_TRIANGLE} indices, got {}",
                indices.len()
            )));
        }
        let count = vertices.len();
        if let Some(index) = indices
            .iter()
            .find(|&&index| usize::try_from(index).map_or(true, |index| index >= count))
        {
            return Err(EngineError::Rendering(format!(
                "the mesh index {index} has no vertex: there are {count}"
            )));
        }
        if !vertices.iter().all(is_finite) {
            return Err(EngineError::Rendering(
                "the vertices of a mesh must be finite numbers".to_owned(),
            ));
        }
        Ok(Self { vertices, indices })
    }

    /// The mesh of a part of a 3D model, in the part's own space.
    ///
    /// # Errors
    ///
    /// [`EngineError::Rendering`] as [`MeshData::new`].
    pub fn from_model_part(part: &ModelPart) -> EngineResult<Self> {
        let vertices = part
            .vertices
            .iter()
            .map(|vertex| {
                let point = |[x, y, z]: [f32; 3]| Vec3::new(x, y, z);
                let [u, v] = vertex.uv;
                MeshVertex::new(
                    point(vertex.position),
                    point(vertex.normal),
                    Vec2::new(u, v),
                )
            })
            .collect();
        Self::new(vertices, part.indices.clone())
    }

    /// A box of `size` (width, height and depth) centered on the origin,
    /// each face showing a whole texture.
    ///
    /// # Errors
    ///
    /// [`EngineError::Rendering`] if a side is not positive and finite.
    pub fn cube(size: Vec3) -> EngineResult<Self> {
        check_sizes("cube", &[size.x, size.y, size.z])?;
        let half = size * 0.5;
        // Each face: the direction it faces, and its right and up directions
        // seen from outside, so `right × up = normal`.
        let faces = [
            (
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, -1.0),
                Vec3::UP,
            ),
            (
                Vec3::new(-1.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
                Vec3::UP,
            ),
            (
                Vec3::UP,
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, -1.0),
            ),
            (
                Vec3::new(0.0, -1.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ),
            (Vec3::new(0.0, 0.0, 1.0), Vec3::new(1.0, 0.0, 0.0), Vec3::UP),
            (
                Vec3::new(0.0, 0.0, -1.0),
                Vec3::new(-1.0, 0.0, 0.0),
                Vec3::UP,
            ),
        ];
        let mut builder = Builder::default();
        for (normal, right, up) in faces {
            builder.quad(
                |across, along| {
                    let direction = normal + right * across + up * along;
                    scaled(direction, half)
                },
                normal,
            );
        }
        Ok(builder.finish())
    }

    /// A flat square of `size` (along x and z) on the origin, facing up,
    /// showing a whole texture.
    ///
    /// # Errors
    ///
    /// [`EngineError::Rendering`] if a side is not positive and finite.
    pub fn plane(size: Vec2) -> EngineResult<Self> {
        check_sizes("plane", &[size.x, size.y])?;
        let half = Vec3::new(size.x * 0.5, 0.0, size.y * 0.5);
        let mut builder = Builder::default();
        builder.quad(
            |across, along| scaled(Vec3::new(across, 0.0, -along), half),
            Vec3::UP,
        );
        Ok(builder.finish())
    }

    /// A sphere of `radius` centered on the origin, made of `segments`
    /// around it and `rings` from pole to pole; a texture wraps around it
    /// once.
    ///
    /// # Errors
    ///
    /// [`EngineError::Rendering`] if the radius is not positive and finite,
    /// or there are fewer than [`MIN_SPHERE_SEGMENTS`] segments, fewer than
    /// [`MIN_SPHERE_RINGS`] rings, or more than [`MAX_SPHERE_DIVISIONS`] of
    /// either.
    pub fn sphere(radius: f32, segments: u32, rings: u32) -> EngineResult<Self> {
        check_sizes("sphere", &[radius])?;
        let valid = (MIN_SPHERE_SEGMENTS..=MAX_SPHERE_DIVISIONS).contains(&segments)
            && (MIN_SPHERE_RINGS..=MAX_SPHERE_DIVISIONS).contains(&rings);
        if !valid {
            return Err(EngineError::Rendering(format!(
                "a sphere needs {MIN_SPHERE_SEGMENTS} to {MAX_SPHERE_DIVISIONS} segments and \
                 {MIN_SPHERE_RINGS} to {MAX_SPHERE_DIVISIONS} rings, got {segments} and {rings}"
            )));
        }
        let mut vertices = Vec::new();
        for ring in 0..=rings {
            let v = fraction(ring, rings);
            let (sin_polar, cos_polar) = (v * PI).sin_cos();
            for segment in 0..=segments {
                let u = fraction(segment, segments);
                let (sin_around, cos_around) = (u * TAU).sin_cos();
                let normal = Vec3::new(sin_polar * cos_around, cos_polar, -sin_polar * sin_around);
                vertices.push(MeshVertex::new(normal * radius, normal, Vec2::new(u, v)));
            }
        }
        let row = segments + 1;
        let mut indices = Vec::new();
        for ring in 0..rings {
            for segment in 0..segments {
                let top_left = ring * row + segment;
                let bottom_left = top_left + row;
                // The triangles that would touch a pole with two corners
                // have no area.
                if ring + 1 < rings {
                    indices.extend_from_slice(&[top_left, bottom_left, bottom_left + 1]);
                }
                if ring > 0 {
                    indices.extend_from_slice(&[top_left, bottom_left + 1, top_left + 1]);
                }
            }
        }
        Self::new(vertices, indices)
    }

    /// The corners of the mesh.
    #[must_use]
    pub fn vertices(&self) -> &[MeshVertex] {
        &self.vertices
    }

    /// The corners of each triangle, three by three.
    #[must_use]
    pub fn indices(&self) -> &[u32] {
        &self.indices
    }
}

/// Collects the corners and triangles of meshes made of flat squares.
#[derive(Default)]
struct Builder {
    vertices: Vec<MeshVertex>,
    indices: Vec<u32>,
}

impl Builder {
    /// Adds a square facing `normal`, whose corner `corner(across, along)`
    /// is at `across` (-1 left, 1 right) and `along` (-1 bottom, 1 top)
    /// seen from the side it faces.
    fn quad(&mut self, corner: impl Fn(f32, f32) -> Vec3, normal: Vec3) {
        let first = u32::try_from(self.vertices.len()).unwrap_or(u32::MAX);
        let corners = [
            (-1.0, -1.0, Vec2::new(0.0, 1.0)),
            (1.0, -1.0, Vec2::new(1.0, 1.0)),
            (1.0, 1.0, Vec2::new(1.0, 0.0)),
            (-1.0, 1.0, Vec2::new(0.0, 0.0)),
        ];
        for (across, along, uv) in corners {
            self.vertices
                .push(MeshVertex::new(corner(across, along), normal, uv));
        }
        self.indices
            .extend([0, 1, 2, 0, 2, 3].map(|offset| first + offset));
    }

    fn finish(self) -> MeshData {
        MeshData {
            vertices: self.vertices,
            indices: self.indices,
        }
    }
}

/// `part / whole`, both at most [`MAX_SPHERE_DIVISIONS`], so exact.
fn fraction(part: u32, whole: u32) -> f32 {
    let exact = |value: u32| f32::from(u16::try_from(value).unwrap_or(u16::MAX));
    exact(part) / exact(whole)
}

fn scaled(direction: Vec3, half: Vec3) -> Vec3 {
    Vec3::new(
        direction.x * half.x,
        direction.y * half.y,
        direction.z * half.z,
    )
}

fn check_sizes(shape: &str, sizes: &[f32]) -> EngineResult<()> {
    match sizes
        .iter()
        .find(|size| !(size.is_finite() && **size > 0.0))
    {
        Some(size) => Err(EngineError::Rendering(format!(
            "the sizes of a {shape} must be positive and finite, not {size}"
        ))),
        None => Ok(()),
    }
}

fn is_finite(vertex: &MeshVertex) -> bool {
    let MeshVertex {
        position,
        normal,
        uv,
    } = vertex;
    [
        position.x, position.y, position.z, normal.x, normal.y, normal.z, uv.x, uv.y,
    ]
    .iter()
    .all(|value| value.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_parts_become_meshes_of_their_vertices() {
        let corner = |x: f32| star_core::ModelVertex::new([x, 0.0, 0.0], [0.0, 1.0, 0.0], [x, 1.0]);
        let part = ModelPart {
            vertices: vec![corner(0.0), corner(1.0), corner(2.0)],
            indices: vec![0, 2, 1],
            material: star_core::ModelMaterial::default(),
            transform: star_core::model::IDENTITY_COLUMNS,
            skinned: false,
        };

        let mesh = MeshData::from_model_part(&part).unwrap();

        assert_eq!(mesh.indices(), [0, 2, 1]);
        assert_eq!(mesh.vertices()[2].position, Vec3::new(2.0, 0.0, 0.0));
        assert_eq!(mesh.vertices()[1].uv, Vec2::new(1.0, 1.0));
    }

    const TOLERANCE: f32 = 1e-4;

    fn triangles(mesh: &MeshData) -> impl Iterator<Item = [MeshVertex; 3]> + '_ {
        mesh.indices()
            .chunks_exact(3)
            .map(|corners| [0, 1, 2].map(|corner| mesh.vertices()[corners[corner] as usize]))
    }

    /// Every triangle shows the side its normals point to, and has area.
    fn assert_faces_outward(mesh: &MeshData) {
        for [a, b, c] in triangles(mesh) {
            let facing = (b.position - a.position).cross(c.position - a.position);
            assert!(facing.length() > 0.0, "{a:?} {b:?} {c:?}");
            assert!(facing.dot(a.normal) > 0.0, "{a:?} {b:?} {c:?}");
        }
        for vertex in mesh.vertices() {
            assert!(
                (vertex.normal.length() - 1.0).abs() < TOLERANCE,
                "{vertex:?}"
            );
        }
    }

    #[test]
    fn cubes_have_six_faces_showing_outward() {
        let cube = MeshData::cube(Vec3::new(2.0, 4.0, 6.0)).unwrap();

        assert_eq!((cube.vertices().len(), cube.indices().len()), (24, 36));
        assert_faces_outward(&cube);
        for vertex in cube.vertices() {
            let Vec3 { x, y, z } = vertex.position;
            assert_eq!((x.abs(), y.abs(), z.abs()), (1.0, 2.0, 3.0));
            // Each corner lies on the face it faces.
            assert!(vertex.position.dot(vertex.normal) > 0.0);
        }
    }

    #[test]
    fn planes_face_up() {
        let plane = MeshData::plane(Vec2::new(4.0, 2.0)).unwrap();

        assert_eq!(plane.indices().len(), 6);
        assert_faces_outward(&plane);
        for vertex in plane.vertices() {
            let Vec3 { x, y, z } = vertex.position;
            assert_eq!((x.abs(), y, z.abs()), (2.0, 0.0, 1.0));
            assert_eq!(vertex.normal, Vec3::UP);
        }
    }

    #[test]
    fn spheres_are_round_and_show_outward() {
        let sphere = MeshData::sphere(2.0, 16, 8).unwrap();

        assert_eq!(sphere.vertices().len(), 17 * 9);
        // Two triangles per square, but one at each pole.
        assert_eq!(sphere.indices().len(), 3 * 16 * (2 * 8 - 2));
        assert_faces_outward(&sphere);
        for vertex in sphere.vertices() {
            assert!((vertex.position.length() - 2.0).abs() < TOLERANCE);
        }
    }

    #[test]
    fn shapes_build_their_meshes() {
        let shape = MeshShape::Sphere {
            radius: 1.0,
            segments: DEFAULT_SPHERE_SEGMENTS,
            rings: DEFAULT_SPHERE_RINGS,
        };

        assert_eq!(
            shape.to_mesh_data().unwrap(),
            MeshData::sphere(1.0, DEFAULT_SPHERE_SEGMENTS, DEFAULT_SPHERE_RINGS).unwrap()
        );
        assert!(MeshShape::Cube { size: Vec3::ZERO }.to_mesh_data().is_err());
    }

    #[test]
    fn impossible_shapes_are_rendering_errors() {
        let errors = [
            MeshData::cube(Vec3::new(1.0, 0.0, 1.0)).err(),
            MeshData::plane(Vec2::new(f32::NAN, 1.0)).err(),
            MeshData::sphere(-1.0, 8, 8).err(),
            MeshData::sphere(1.0, 2, 8).err(),
            MeshData::sphere(1.0, 8, MAX_SPHERE_DIVISIONS + 1).err(),
        ];

        for error in errors {
            assert!(
                matches!(error, Some(EngineError::Rendering(_))),
                "{error:?}"
            );
        }
    }

    #[test]
    fn meshes_need_whole_triangles_of_existing_finite_corners() {
        let corner = MeshVertex::new(Vec3::ZERO, Vec3::UP, Vec2::ZERO);
        let broken = MeshVertex::new(Vec3::new(f32::INFINITY, 0.0, 0.0), Vec3::UP, Vec2::ZERO);

        assert!(MeshData::new(vec![corner; 3], vec![0, 1, 2]).is_ok());
        assert!(MeshData::new(vec![corner; 3], vec![]).is_err());
        assert!(MeshData::new(vec![corner; 3], vec![0, 1]).is_err());
        assert!(MeshData::new(vec![corner; 3], vec![0, 1, 3]).is_err());
        assert!(MeshData::new(vec![corner, corner, broken], vec![0, 1, 2]).is_err());
    }
}
