//! Images dragged from the assets panel and dropped on the scene: each
//! becomes a new node that shows it, where it was dropped. In 2D it is a
//! sprite on the `z = 0` plane; in 3D, a plane on the ground (`y = 0`)
//! with the image as its texture.

use star_engine::prelude::{
    Affine2, Mat4, MeshShape, TextureFilter, Transform2D, Transform3D, Vec2, Vec3,
};
use star_engine::star_assets::AssetPath;
use star_engine::star_scene::{MeshOutline, NodeOutline};

use super::components::{new_mesh, new_node, sprite_of};
use super::picking::Ray;
use crate::project::ProjectDimension;

/// How tall a dropped image is, in world units; its width keeps its shape.
const DROPPED_HEIGHT: f32 = 1.0;
/// How far along the pointer's ray a 3D node goes when the ray never
/// reaches the ground.
const DISTANCE_OFF_THE_GROUND: f32 = 10.0;
/// Rays closer than this to lying on a plane never meet it.
const PARALLEL_EPSILON: f32 = 1e-6;

/// What the assets panel carries while an image of it is dragged.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ImageDrop {
    /// Where it is inside the assets folder.
    pub(crate) path: String,
    /// Its width over its height, if it is known.
    pub(crate) aspect: Option<f32>,
}

impl ImageDrop {
    /// The name of a node that shows it: its file's name without the
    /// extension.
    pub(super) fn node_name(&self) -> &str {
        let file = self.path.rsplit('/').next().unwrap_or(&self.path);
        file.rsplit_once('.').map_or(file, |(stem, _)| stem)
    }

    /// The size it is shown at: [`DROPPED_HEIGHT`] tall, as wide as its
    /// shape asks.
    fn size(&self) -> Vec2 {
        let aspect = self
            .aspect
            .filter(|aspect| aspect.is_finite() && *aspect > 0.0)
            .unwrap_or(1.0);
        Vec2::new(DROPPED_HEIGHT * aspect, DROPPED_HEIGHT)
    }

    /// A node named `name` showing the image, at `world` for a parent that
    /// places its children at `parent`. `None` if the path is not valid or
    /// the parent cannot be undone (a scale of zero).
    pub(super) fn node(
        &self,
        name: String,
        dimension: ProjectDimension,
        world: Vec3,
        parent: (Affine2, Mat4),
    ) -> Option<NodeOutline> {
        let texture = AssetPath::new(&self.path).ok()?;
        let mut node = new_node(name, dimension);
        match dimension {
            ProjectDimension::TwoD => {
                let position = parent
                    .0
                    .inverse()?
                    .transform_point(Vec2::new(world.x, world.y));
                node.transform = Some(Transform2D::from_position(position));
                node.sprite = Some(sprite_of(texture, self.size()));
            }
            ProjectDimension::ThreeD => {
                node.transform_3d = Some(Transform3D {
                    position: parent.1.inverse()?.transform_point(world),
                    ..Transform3D::IDENTITY
                });
                node.mesh = Some(MeshOutline {
                    shape: MeshShape::Plane { size: self.size() },
                    texture: Some((texture, TextureFilter::Smooth)),
                    ..new_mesh()
                });
            }
        }
        Some(node)
    }
}

/// Where in the world something dropped along `ray` lands: on the 2D
/// plane, or on the ground in 3D (in front of the camera if the ray never
/// reaches it).
pub(super) fn drop_point(ray: Ray, dimension: ProjectDimension) -> Vec3 {
    let (height, along) = match dimension {
        ProjectDimension::TwoD => (ray.origin.z, ray.direction.z),
        ProjectDimension::ThreeD => (ray.origin.y, ray.direction.y),
    };
    let distance = -height / along;
    if along.abs() > PARALLEL_EPSILON && distance >= 0.0 {
        return ray.at(distance);
    }
    let forward = ray.direction.normalized().unwrap_or(ray.direction);
    ray.origin + forward * DISTANCE_OFF_THE_GROUND
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drop(path: &str, aspect: Option<f32>) -> ImageDrop {
        ImageDrop {
            path: path.to_owned(),
            aspect,
        }
    }

    const NO_PARENT: (Affine2, Mat4) = (Affine2::IDENTITY, Mat4::IDENTITY);

    #[test]
    fn nodes_are_named_after_the_image() {
        assert_eq!(drop("enemies/boss.png", None).node_name(), "boss");
        assert_eq!(drop("tile", None).node_name(), "tile");
    }

    #[test]
    fn two_d_drops_become_sprites_where_they_land_inside_their_parent() {
        let parent = Affine2::from_scale_rotation_translation(Vec2::ONE, 0.0, Vec2::new(1.0, 0.0));

        let node = drop("hero.png", Some(2.0))
            .node(
                "hero".to_owned(),
                ProjectDimension::TwoD,
                Vec3::new(3.0, 1.0, 0.0),
                (parent, Mat4::IDENTITY),
            )
            .unwrap();

        assert_eq!(node.transform.unwrap().position, Vec2::new(2.0, 1.0));
        let sprite = node.sprite.unwrap();
        assert_eq!(sprite.size, Vec2::new(2.0, 1.0));
        assert_eq!(sprite.texture.as_str(), "hero.png");
    }

    #[test]
    fn three_d_drops_become_textured_planes() {
        let node = drop("floor.png", None)
            .node(
                "floor".to_owned(),
                ProjectDimension::ThreeD,
                Vec3::new(1.0, 0.0, 2.0),
                NO_PARENT,
            )
            .unwrap();

        assert_eq!(
            node.transform_3d.unwrap().position,
            Vec3::new(1.0, 0.0, 2.0)
        );
        let mesh = node.mesh.unwrap();
        assert_eq!(mesh.shape, MeshShape::Plane { size: Vec2::ONE });
        assert!(mesh.texture.is_some());
    }

    #[test]
    fn drops_land_on_the_plane_of_the_project() {
        let down = Ray {
            origin: Vec3::new(2.0, 5.0, 10.0),
            direction: Vec3::new(0.0, -10.0, -10.0),
        };

        assert_eq!(
            drop_point(down, ProjectDimension::TwoD),
            Vec3::new(2.0, -5.0, 0.0)
        );
        assert_eq!(
            drop_point(down, ProjectDimension::ThreeD),
            Vec3::new(2.0, 0.0, 5.0)
        );
    }

    #[test]
    fn rays_that_miss_the_ground_drop_in_front_of_the_camera() {
        let up = Ray {
            origin: Vec3::UP,
            direction: Vec3::new(0.0, 2.0, 0.0),
        };

        let point = drop_point(up, ProjectDimension::ThreeD);

        assert_eq!(point, Vec3::new(0.0, 1.0 + DISTANCE_OFF_THE_GROUND, 0.0));
    }
}
