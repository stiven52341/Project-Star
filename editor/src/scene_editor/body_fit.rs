//! “Fit” in a 3D body's section of the inspector (decision 84): its box,
//! sphere or capsule sized and placed around what its node shows, a mesh
//! of a shape or a model, so complex models get a simple shape without
//! measuring them.

use std::path::Path;

use star_engine::prelude::{AssetPath, Vec3};
use star_engine::star_assets::AssetFolder;
use star_engine::star_physics::{Shape3d, ShapeKind3d};
use star_engine::star_render::MeshShape;
use star_engine::star_scene::{CollisionMesh, NodeOutline};

use super::fields::MIN_SIZE;

/// The box around what a node shows, in the node's space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Bounds {
    min: Vec3,
    max: Vec3,
}

impl Bounds {
    /// The box around `points`; `None` without points.
    fn around(points: impl IntoIterator<Item = Vec3>) -> Option<Self> {
        points.into_iter().fold(None, |bounds, point| {
            Some(bounds.map_or(
                Self {
                    min: point,
                    max: point,
                },
                |Self { min, max }| Self {
                    min: Vec3::new(min.x.min(point.x), min.y.min(point.y), min.z.min(point.z)),
                    max: Vec3::new(max.x.max(point.x), max.y.max(point.y), max.z.max(point.z)),
                },
            ))
        })
    }

    fn center(self) -> Vec3 {
        (self.min + self.max) * 0.5
    }

    /// Its size along each axis, at least [`MIN_SIZE`].
    fn size(self) -> Vec3 {
        let side = |min: f32, max: f32| (max - min).max(MIN_SIZE);
        Vec3::new(
            side(self.min.x, self.max.x),
            side(self.min.y, self.max.y),
            side(self.min.z, self.max.z),
        )
    }
}

/// What a node shows that a body can be fitted to.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Shown {
    /// A mesh of a shape.
    Mesh(MeshShape),
    /// The model of a file of the assets folder.
    Model(AssetPath),
}

impl Shown {
    /// What `node` shows, if a body can be fitted to it.
    pub(super) fn of(node: &NodeOutline) -> Option<Self> {
        node.mesh
            .as_ref()
            .map(|mesh| Self::Mesh(mesh.shape))
            .or_else(|| {
                node.model
                    .as_ref()
                    .map(|model| Self::Model(model.file.clone()))
            })
    }

    /// The box around it, reading a model from `folder`; `None` if the
    /// model cannot be read (the reason is logged).
    pub(super) fn bounds(&self, folder: &Path) -> Option<Bounds> {
        match self {
            Self::Mesh(shape) => Some(mesh_bounds(*shape)),
            Self::Model(file) => model_bounds(file, folder),
        }
    }
}

fn mesh_bounds(shape: MeshShape) -> Bounds {
    let half = match shape {
        MeshShape::Cube { size } => size * 0.5,
        MeshShape::Plane { size } => Vec3::new(size.x * 0.5, 0.0, size.y * 0.5),
        MeshShape::Sphere { radius, .. } => Vec3::ONE * radius,
    };
    Bounds {
        min: -half,
        max: half,
    }
}

fn model_bounds(file: &AssetPath, folder: &Path) -> Option<Bounds> {
    AssetFolder::new(folder)
        .load_model(file)
        .and_then(|model| CollisionMesh::from_model(&model))
        .inspect_err(|error| log::warn!("Cannot fit a body to \"{file}\": {error}"))
        .ok()
        .and_then(|mesh| Bounds::around(mesh.0.corners().iter().copied()))
}

/// A shape of `shape`'s kind around `bounds`: a box of its size, a sphere
/// that holds its largest side, or an upright capsule as wide as its
/// widest side across, all centered on it. A mesh already fits.
pub(super) fn fitted(shape: Shape3d, bounds: Bounds) -> Shape3d {
    let size = bounds.size();
    let fitted = match shape.kind() {
        ShapeKind3d::Mesh => return shape,
        ShapeKind3d::Box { .. } => Shape3d::cuboid(size),
        ShapeKind3d::Sphere { .. } => Shape3d::sphere(size.x.max(size.y).max(size.z) * 0.5),
        ShapeKind3d::Capsule { .. } => {
            let radius = size.x.max(size.z) * 0.5;
            Shape3d::capsule(size.y.max(2.0 * radius), radius)
        }
    };
    fitted
        .and_then(|fitted| fitted.with_offset(bounds.center()))
        .unwrap_or(shape)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A soldier 2 high and 1 across, its origin at its feet.
    const SOLDIER: Bounds = Bounds {
        min: Vec3::new(-0.5, 0.0, -0.25),
        max: Vec3::new(0.5, 2.0, 0.25),
    };

    #[test]
    fn capsules_stand_around_the_model() {
        let capsule = fitted(Shape3d::capsule(1.0, 0.25).unwrap(), SOLDIER);

        assert_eq!(
            capsule.kind(),
            ShapeKind3d::Capsule {
                height: 2.0,
                radius: 0.5
            }
        );
        assert_eq!(capsule.offset(), Vec3::new(0.0, 1.0, 0.0));
    }

    #[test]
    fn boxes_and_spheres_hold_the_model() {
        let crate_box = fitted(Shape3d::cuboid(Vec3::ONE).unwrap(), SOLDIER);
        let ball = fitted(Shape3d::sphere(0.1).unwrap(), SOLDIER);

        assert_eq!(
            crate_box.kind(),
            ShapeKind3d::Box {
                size: Vec3::new(1.0, 2.0, 0.5)
            }
        );
        assert_eq!(ball.kind(), ShapeKind3d::Sphere { radius: 1.0 });
        assert_eq!(ball.offset(), Vec3::new(0.0, 1.0, 0.0));
        assert_eq!(fitted(Shape3d::mesh(), SOLDIER), Shape3d::mesh());
    }

    #[test]
    fn flat_meshes_give_thin_boxes() {
        let plane = mesh_bounds(MeshShape::Plane {
            size: star_engine::prelude::Vec2::new(4.0, 2.0),
        });

        let floor = fitted(Shape3d::cuboid(Vec3::ONE).unwrap(), plane);

        assert_eq!(
            floor.kind(),
            ShapeKind3d::Box {
                size: Vec3::new(4.0, MIN_SIZE, 2.0)
            }
        );
    }

    #[test]
    fn bounds_hold_every_point() {
        let bounds =
            Bounds::around([Vec3::new(1.0, -2.0, 0.0), Vec3::new(-1.0, 3.0, 0.5)]).unwrap();

        assert_eq!(bounds.center(), Vec3::new(0.0, 0.5, 0.25));
        assert_eq!(Bounds::around([]), None);
    }
}
