//! Finding the node under the pointer: a ray from the camera through the
//! point clicked, and the node it meets first.
//!
//! Nodes are hit where they are seen: sprites over their rectangle, meshes
//! over their box (spheres over their ball), and nodes that show nothing
//! over the cross that marks them.

use std::cmp::Ordering;

use star_engine::MeshFile;
use star_engine::prelude::{Affine2, Mat4, MeshShape, SceneOutline, Vec2, Vec3};
use star_engine::star_scene::{NodeOutline, SpriteOutline};

use super::drawing::{POINT_MARK_SIZE, Place, places};

/// Near and far ends of the depth the GPU keeps, in its screen space.
const NEAR_DEPTH: f32 = 0.0;
const FAR_DEPTH: f32 = 1.0;
/// Directions closer than this to lying on a plane never meet it.
const PARALLEL_EPSILON: f32 = 1e-6;
/// Distances closer than this, along the ray, count as the same: sprites
/// on one plane are then told apart by their layer.
const SAME_DISTANCE: f32 = 1e-4;
/// Half the side of the unit box meshes are made from.
const UNIT_HALF: f32 = 0.5;

/// A line from the camera into the scene. Points along it are
/// `origin + direction * distance`, distance 0 at the near plane and 1 at
/// the far one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Ray {
    pub(super) origin: Vec3,
    pub(super) direction: Vec3,
}

impl Ray {
    /// The ray seen at `screen`, from `(-1, -1)` at the bottom left of the
    /// view to `(1, 1)` at the top right, by a camera whose matrix is
    /// `view_projection`. `None` if that matrix cannot be undone.
    pub(crate) fn through(view_projection: Mat4, screen: Vec2) -> Option<Self> {
        let to_world = view_projection.inverse()?;
        let near = to_world.transform_point(Vec3::new(screen.x, screen.y, NEAR_DEPTH));
        let far = to_world.transform_point(Vec3::new(screen.x, screen.y, FAR_DEPTH));
        Some(Self {
            origin: near,
            direction: far - near,
        })
    }

    pub(super) fn at(self, distance: f32) -> Vec3 {
        self.origin + self.direction * distance
    }

    /// The same ray in the space that `to_space` takes world points to.
    /// Distances along it stay the same, since lines map to lines.
    fn moved_by(self, to_space: Mat4) -> Self {
        let origin = to_space.transform_point(self.origin);
        let end = to_space.transform_point(self.origin + self.direction);
        Self {
            origin,
            direction: end - origin,
        }
    }
}

/// The node `ray` meets first in `outline`, if any.
pub(crate) fn node_at(outline: &SceneOutline, ray: Ray) -> Option<usize> {
    let nodes = outline.nodes();
    nodes
        .iter()
        .zip(places(nodes))
        .enumerate()
        .flat_map(|(index, (node, place))| hits(node, place, ray, index))
        .max_by(Hit::compare)
        .map(|hit| hit.index)
}

/// Where a ray meets a node.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Hit {
    index: usize,
    distance: f32,
    /// Whether it met what the node shows, not just the cross marking a
    /// node that shows nothing.
    shown: bool,
    /// The sprite's layer: higher ones are drawn on top.
    layer: i32,
}

impl Hit {
    /// Orders hits from the worst to the best: crosses lose to what nodes
    /// show, farther hits to nearer ones, lower layers to higher ones, and
    /// earlier nodes to later ones, which are drawn over them.
    fn compare(first: &Self, second: &Self) -> Ordering {
        first
            .shown
            .cmp(&second.shown)
            .then_with(|| {
                if (first.distance - second.distance).abs() < SAME_DISTANCE {
                    Ordering::Equal
                } else {
                    second.distance.total_cmp(&first.distance)
                }
            })
            .then(first.layer.cmp(&second.layer))
            .then(first.index.cmp(&second.index))
    }
}

/// Where `ray` meets the node at `index`, in 2D and in 3D.
fn hits(node: &NodeOutline, place: Place, ray: Ray, index: usize) -> Vec<Hit> {
    let hit = |(distance, shown, layer)| Hit {
        index,
        distance,
        shown,
        layer,
    };
    [
        place.flat.and_then(|global| flat_hit(node, global, ray)),
        place.solid.and_then(|global| solid_hit(node, global, ray)),
    ]
    .into_iter()
    .flatten()
    .map(hit)
    .collect()
}

/// Where `ray` meets a 2D node placed at `global` on the `z = 0` plane:
/// over its sprite, or near its position if it has none.
fn flat_hit(node: &NodeOutline, global: Affine2, ray: Ray) -> Option<(f32, bool, i32)> {
    if ray.direction.z.abs() < PARALLEL_EPSILON {
        return None;
    }
    let distance = -ray.origin.z / ray.direction.z;
    let point = ray.at(distance);
    let point = Vec2::new(point.x, point.y);
    if let Some(sprite) = &node.sprite {
        let local = global.inverse()?.transform_point(point);
        return within_sprite(sprite, local).then_some((distance, true, sprite.layer));
    }
    let offset = point - global.translation();
    (offset.length() <= POINT_MARK_SIZE / 2.0).then_some((distance, false, 0))
}

/// Whether `local`, in the node's own space, falls on its sprite.
fn within_sprite(sprite: &SpriteOutline, local: Vec2) -> bool {
    let low = Vec2::new(
        -sprite.anchor.x * sprite.size.x,
        -sprite.anchor.y * sprite.size.y,
    );
    let high = low + sprite.size;
    (low.x..=high.x).contains(&local.x) && (low.y..=high.y).contains(&local.y)
}

/// Where `ray` meets a 3D node placed at `global`: its mesh, or a ball
/// around its position if it has none.
fn solid_hit(node: &NodeOutline, global: Mat4, ray: Ray) -> Option<(f32, bool, i32)> {
    let shape = node
        .mesh
        .as_ref()
        .and_then(|mesh| MeshFile::new(mesh.shape).unit_shape().ok());
    match shape {
        Some((unit, scale)) => {
            let to_unit = (global * Mat4::from_scale(scale)).inverse()?;
            let local = ray.moved_by(to_unit);
            let distance = match unit {
                MeshShape::Sphere { .. } => ball_hit(local, Vec3::ZERO, UNIT_HALF),
                MeshShape::Cube { .. } => box_hit(local, Vec3::ONE * UNIT_HALF),
                MeshShape::Plane { .. } => box_hit(local, Vec3::new(UNIT_HALF, 0.0, UNIT_HALF)),
            }?;
            Some((distance, true, 0))
        }
        None => ball_hit(ray, global.translation(), POINT_MARK_SIZE / 2.0)
            .map(|distance| (distance, false, 0)),
    }
}

/// The nearest distance, in front of the ray's origin, at which `ray`
/// enters the box from `-half` to `half`.
fn box_hit(ray: Ray, half: Vec3) -> Option<f32> {
    let axes = [
        (ray.origin.x, ray.direction.x, half.x),
        (ray.origin.y, ray.direction.y, half.y),
        (ray.origin.z, ray.direction.z, half.z),
    ];
    let mut enter = f32::NEG_INFINITY;
    let mut leave = f32::INFINITY;
    for (origin, direction, half) in axes {
        if direction.abs() < PARALLEL_EPSILON {
            if origin.abs() > half {
                return None;
            }
            continue;
        }
        let (near, far) = ((-half - origin) / direction, (half - origin) / direction);
        enter = enter.max(near.min(far));
        leave = leave.min(near.max(far));
    }
    (enter <= leave && leave >= 0.0).then_some(enter.max(0.0))
}

/// The nearest distance, in front of the ray's origin, at which `ray`
/// enters the ball of `radius` around `center`.
fn ball_hit(ray: Ray, center: Vec3, radius: f32) -> Option<f32> {
    let to_origin = ray.origin - center;
    let a = ray.direction.dot(ray.direction);
    let b = 2.0 * to_origin.dot(ray.direction);
    let c = to_origin.dot(to_origin) - radius * radius;
    let discriminant = b * b - 4.0 * a * c;
    if a < PARALLEL_EPSILON || discriminant < 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    let (near, far) = ((-b - root) / (2.0 * a), (-b + root) / (2.0 * a));
    (far >= 0.0).then_some(near.max(0.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outline(text: &str) -> SceneOutline {
        SceneOutline::from_text(&format!("format = 1\n{text}")).unwrap()
    }

    /// A ray going straight down the Z axis through `(x, y)`, as a 2D
    /// camera sees.
    fn down_at(x: f32, y: f32) -> Ray {
        Ray {
            origin: Vec3::new(x, y, 10.0),
            direction: Vec3::new(0.0, 0.0, -20.0),
        }
    }

    const SPRITES: &str = "[[node]]\nname = \"Root\"\n\
        [[node]]\nname = \"Back\"\nparent = 0\nposition = [0.0, 0.0]\n\
        [node.sprite]\ntexture = \"a.png\"\nsize = [4.0, 4.0]\nlayer = 2\n\
        [[node]]\nname = \"Front\"\nparent = 0\nposition = [1.0, 1.0]\nscale = [2.0, 2.0]\n\
        [node.sprite]\ntexture = \"b.png\"\nsize = [1.0, 1.0]\n\
        [[node]]\nname = \"Spot\"\nparent = 0\nposition = [10.0, 0.0]\n";

    #[test]
    fn sprites_are_hit_over_their_placed_rectangle() {
        let scene = outline(SPRITES);

        assert_eq!(node_at(&scene, down_at(-1.5, 1.5)), Some(1));
        assert_eq!(node_at(&scene, down_at(2.8, 0.2)), None);
        assert_eq!(node_at(&scene, down_at(3.5, 0.0)), None, "outside both");
    }

    #[test]
    fn higher_layers_win_where_sprites_overlap() {
        let scene = outline(SPRITES);

        assert_eq!(node_at(&scene, down_at(1.2, 1.2)), Some(1));
    }

    #[test]
    fn later_nodes_win_on_the_same_layer() {
        let scene = outline(&SPRITES.replace("layer = 2\n", ""));

        assert_eq!(node_at(&scene, down_at(1.2, 1.2)), Some(2));
    }

    #[test]
    fn nodes_that_show_nothing_are_hit_near_their_position() {
        let scene = outline(SPRITES);

        assert_eq!(node_at(&scene, down_at(10.1, 0.1)), Some(3));
        assert_eq!(node_at(&scene, down_at(11.0, 0.0)), None);
    }

    const SOLIDS: &str = "[[node]]\nname = \"Root\"\n[node.transform_3d]\n\
        [[node]]\nname = \"Near\"\nparent = 0\n[node.transform_3d]\nposition = [0.0, 0.0, 2.0]\n\
        [node.mesh]\nshape = \"cube\"\nsize = [1.0, 1.0, 1.0]\n\
        [[node]]\nname = \"Far\"\nparent = 0\n[node.transform_3d]\nposition = [0.0, 0.0, -2.0]\n\
        [node.mesh]\nshape = \"sphere\"\nradius = 1.5\n";

    #[test]
    fn the_nearest_mesh_is_hit() {
        let scene = outline(SOLIDS);

        assert_eq!(node_at(&scene, down_at(0.0, 0.0)), Some(1));
        assert_eq!(node_at(&scene, down_at(1.0, 1.0)), Some(2), "only the ball");
        assert_eq!(node_at(&scene, down_at(1.2, 1.2)), None, "past the ball");
    }

    #[test]
    fn rays_come_out_of_the_camera_through_the_point_clicked() {
        let view = Mat4::orthographic(-5.0, 5.0, -5.0, 5.0, 1.0, 11.0)
            * Mat4::look_at(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::UP).unwrap();

        let ray = Ray::through(view, Vec2::new(0.5, -1.0)).unwrap();

        let on_plane = ray.at(-ray.origin.z / ray.direction.z);
        assert!((on_plane - Vec3::new(2.5, -5.0, 0.0)).length() < 1e-4);
    }

    #[test]
    fn boxes_behind_the_ray_are_missed() {
        let away = Ray {
            origin: Vec3::new(0.0, 0.0, 5.0),
            direction: Vec3::new(0.0, 0.0, 1.0),
        };

        assert_eq!(box_hit(away, Vec3::ONE), None);
        assert_eq!(ball_hit(away, Vec3::ZERO, 1.0), None);
    }
}
