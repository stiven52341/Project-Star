//! Drawing the shapes of bodies in the view: outlines of what each body
//! collides with, so bodies that show nothing can still be seen and
//! placed. They are made of a few line meshes of one unit, placed and
//! sized for each shape.

use std::f32::consts::{PI, TAU};

use star_engine::prelude::{
    Affine2, BodySettings, Color, Frame, GlobalTransform2D, Mat4, MeshId, Shape2d, Shape3d,
    ShapeKind, ShapeKind3d, Vec3, Vertex,
};
use star_engine::star_core::EngineResult;

/// The color of the shapes of bodies, as the scene's tree shows them.
const BODY_COLOR: Color = Color::rgb(0.7, 0.47, 0.86);
/// How many segments make a whole circle; half circles take half.
const CIRCLE_SEGMENTS: usize = 32;

/// The line meshes the shapes of bodies are drawn with, made once by the
/// view, all in the color of bodies.
#[derive(Debug, Clone, Copy)]
pub(crate) struct BodyMarks {
    /// A square from `(-0.5, -0.5)` to `(0.5, 0.5)`, on the XY plane.
    square: MeshId,
    /// A circle of radius 1 around the origin, on the XY plane.
    circle: MeshId,
    /// The upper half of [`BodyMarks::circle`].
    half_circle: MeshId,
    /// Two segments, at `x = -1` and `x = 1`, from `y = -0.5` to `0.5`.
    sides: MeshId,
    /// A cube from `-0.5` to `0.5` on every axis.
    cube: MeshId,
    /// Three circles of radius 1 around the origin, one on each plane.
    sphere: MeshId,
    /// The upper half of [`BodyMarks::sphere`], with its whole equator.
    half_sphere: MeshId,
    /// Four segments around the y axis, one unit from it, from `y = -0.5`
    /// to `0.5`.
    sides_3d: MeshId,
}

impl BodyMarks {
    /// Makes every mesh with `create_lines`.
    ///
    /// # Errors
    ///
    /// Whatever keeps `create_lines` from making one.
    pub(crate) fn create(
        mut create_lines: impl FnMut(&[Vertex]) -> EngineResult<MeshId>,
    ) -> EngineResult<Self> {
        Ok(Self {
            square: create_lines(&square_lines())?,
            circle: create_lines(&arc_lines(Plane::Xy, TAU))?,
            half_circle: create_lines(&arc_lines(Plane::Xy, PI))?,
            sides: create_lines(&side_lines(&[Vec3::new(1.0, 0.0, 0.0)]))?,
            cube: create_lines(&cube_lines())?,
            sphere: create_lines(&sphere_lines(TAU))?,
            half_sphere: create_lines(&sphere_lines(PI))?,
            sides_3d: create_lines(&side_lines(&[
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            ]))?,
        })
    }
}

/// Draws the shape of the 2D `body` of a node placed at `global`: where
/// the node is and turned as it is, but not scaled, as the physics has it.
pub(super) fn draw_body_2d(
    body: &BodySettings,
    global: Affine2,
    marks: &BodyMarks,
    frame: &mut Frame,
) {
    let global = GlobalTransform2D::from_affine(global);
    let position = global.position();
    let base = Mat4::from_translation(Vec3::new(position.x, position.y, 0.0))
        * Mat4::from_rotation_z(global.rotation());
    for (mesh, place) in pieces_2d(&body.shape, marks) {
        frame.draw(mesh, base * place);
    }
}

/// Draws the shape of the 3D `body` of a node placed at `global`: where
/// the node is and turned as it is, but not scaled, as the physics has it.
pub(super) fn draw_body_3d(
    body: &BodySettings<Shape3d>,
    global: Mat4,
    marks: &BodyMarks,
    frame: &mut Frame,
) {
    let base = super::drawing::without_scale(global);
    for (mesh, place) in pieces_3d(&body.shape, marks) {
        frame.draw(mesh, base * place);
    }
}

/// The meshes that draw `shape`, each placed around the body's center.
fn pieces_2d(shape: &Shape2d, marks: &BodyMarks) -> Vec<(MeshId, Mat4)> {
    match shape.kind() {
        ShapeKind::Circle { radius } => vec![(marks.circle, scaled(radius, radius, 1.0))],
        ShapeKind::Rectangle { size } => vec![(marks.square, scaled(size.x, size.y, 1.0))],
        ShapeKind::Capsule { height, radius } => {
            capsule(height, radius, [marks.half_circle, marks.sides], 1.0)
        }
    }
}

/// The meshes that draw `shape`, each placed around the body's center.
fn pieces_3d(shape: &Shape3d, marks: &BodyMarks) -> Vec<(MeshId, Mat4)> {
    match shape.kind() {
        ShapeKind3d::Sphere { radius } => vec![(marks.sphere, scaled(radius, radius, radius))],
        ShapeKind3d::Box { size } => vec![(marks.cube, scaled(size.x, size.y, size.z))],
        ShapeKind3d::Capsule { height, radius } => {
            capsule(height, radius, [marks.half_sphere, marks.sides_3d], radius)
        }
    }
}

/// A vertical capsule of `height` and `radius`: the `half` round end on
/// top, the same one upside down below, and the `sides` between them.
/// `depth` is how the meshes are scaled along z.
fn capsule(
    height: f32,
    radius: f32,
    [half, sides]: [MeshId; 2],
    depth: f32,
) -> Vec<(MeshId, Mat4)> {
    let straight = (height - 2.0 * radius).max(0.0);
    let end = |y: f32, up: f32| {
        Mat4::from_translation(Vec3::new(0.0, y, 0.0)) * scaled(radius, radius * up, depth)
    };
    vec![
        (half, end(straight / 2.0, 1.0)),
        (half, end(-straight / 2.0, -1.0)),
        (sides, scaled(radius, straight, depth)),
    ]
}

fn scaled(x: f32, y: f32, z: f32) -> Mat4 {
    Mat4::from_scale(Vec3::new(x, y, z))
}

/// A plane through the origin, by the two axes it spans.
#[derive(Clone, Copy)]
enum Plane {
    Xy,
    Xz,
    Yz,
}

impl Plane {
    /// The point of the circle of radius 1 on this plane at `angle`,
    /// counted from its first axis towards its second.
    fn point(self, angle: f32) -> Vec3 {
        let (along, across) = (angle.cos(), angle.sin());
        match self {
            Self::Xy => Vec3::new(along, across, 0.0),
            Self::Xz => Vec3::new(along, 0.0, across),
            Self::Yz => Vec3::new(0.0, across, along),
        }
    }
}

/// An arc of radius 1 on `plane`, from its first axis through `turn`
/// radians.
fn arc_lines(plane: Plane, turn: f32) -> Vec<Vertex> {
    // At least one segment; a whole turn takes them all.
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    let segments = ((turn / TAU) * CIRCLE_SEGMENTS as f32).ceil().max(1.0) as usize;
    #[allow(clippy::cast_precision_loss)]
    let angle = |step: usize| turn * step as f32 / segments as f32;
    (0..segments)
        .flat_map(|step| {
            [
                Vertex::new(plane.point(angle(step)), BODY_COLOR),
                Vertex::new(plane.point(angle(step + 1)), BODY_COLOR),
            ]
        })
        .collect()
}

/// Circles of radius 1 on the vertical planes, through `turn` radians from
/// the side upwards, and the whole equator.
fn sphere_lines(turn: f32) -> Vec<Vertex> {
    let mut lines = arc_lines(Plane::Xy, turn);
    lines.extend(arc_lines(Plane::Yz, turn));
    lines.extend(arc_lines(Plane::Xz, TAU));
    lines
}

/// Vertical segments from `y = -0.5` to `0.5` at each of `offsets` from
/// the y axis and at the opposite side.
fn side_lines(offsets: &[Vec3]) -> Vec<Vertex> {
    let half = Vec3::new(0.0, 0.5, 0.0);
    offsets
        .iter()
        .flat_map(|offset| [*offset, *offset * -1.0])
        .flat_map(|foot| {
            [
                Vertex::new(foot - half, BODY_COLOR),
                Vertex::new(foot + half, BODY_COLOR),
            ]
        })
        .collect()
}

fn square_lines() -> Vec<Vertex> {
    let half = 0.5;
    let corners = [(-half, -half), (half, -half), (half, half), (-half, half)]
        .map(|(x, y)| Vec3::new(x, y, 0.0));
    closed_loop(&corners)
}

fn cube_lines() -> Vec<Vertex> {
    let half = 0.5;
    let face = |y: f32| {
        [(-half, -half), (half, -half), (half, half), (-half, half)]
            .map(|(x, z)| Vec3::new(x, y, z))
    };
    let (bottom, top) = (face(-half), face(half));
    let mut lines = closed_loop(&bottom);
    lines.extend(closed_loop(&top));
    for (low, high) in bottom.iter().zip(&top) {
        lines.extend([
            Vertex::new(*low, BODY_COLOR),
            Vertex::new(*high, BODY_COLOR),
        ]);
    }
    lines
}

/// Segments joining `corners` in order, and the last to the first.
fn closed_loop(corners: &[Vec3]) -> Vec<Vertex> {
    corners
        .iter()
        .zip(corners.iter().cycle().skip(1))
        .flat_map(|(from, to)| [Vertex::new(*from, BODY_COLOR), Vertex::new(*to, BODY_COLOR)])
        .collect()
}

/// Meshes of made-up numbers, for tests that only count what is drawn.
#[cfg(test)]
pub(super) const TEST_MARKS: BodyMarks = BodyMarks {
    square: MeshId::from_index(10),
    circle: MeshId::from_index(11),
    half_circle: MeshId::from_index(12),
    sides: MeshId::from_index(13),
    cube: MeshId::from_index(14),
    sphere: MeshId::from_index(15),
    half_sphere: MeshId::from_index(16),
    sides_3d: MeshId::from_index(17),
};

#[cfg(test)]
mod tests {
    use star_engine::prelude::{BodyKind, Vec2};

    use super::*;

    const EPSILON: f32 = 1e-5;

    const MARKS: BodyMarks = TEST_MARKS;

    fn ends(lines: &[Vertex]) -> Vec<Vec3> {
        lines.iter().map(|vertex| vertex.position).collect()
    }

    #[test]
    fn every_mesh_is_made_of_segments() {
        for lines in [
            square_lines(),
            arc_lines(Plane::Xy, TAU),
            arc_lines(Plane::Xy, PI),
            side_lines(&[Vec3::new(1.0, 0.0, 0.0)]),
            cube_lines(),
            sphere_lines(TAU),
            sphere_lines(PI),
        ] {
            assert!(!lines.is_empty() && lines.len() % 2 == 0);
        }
    }

    #[test]
    fn circles_have_radius_one_and_half_circles_stay_above() {
        for point in ends(&arc_lines(Plane::Xy, TAU)) {
            assert!((point.length() - 1.0).abs() < EPSILON);
        }
        assert!(
            ends(&arc_lines(Plane::Xy, PI))
                .iter()
                .all(|point| point.y > -EPSILON)
        );
        assert_eq!(arc_lines(Plane::Xy, TAU).len(), 2 * CIRCLE_SEGMENTS);
    }

    #[test]
    fn capsules_have_their_round_ends_at_the_top_and_bottom() {
        let shape = Shape2d::capsule(3.0, 0.5).unwrap();

        let pieces = pieces_2d(&shape, &MARKS);

        let tops: Vec<Vec3> = pieces
            .iter()
            .filter(|(mesh, _)| *mesh == MARKS.half_circle)
            .map(|(_, place)| place.transform_point(Vec3::new(0.0, 1.0, 0.0)))
            .collect();
        assert_eq!(tops.len(), 2);
        assert!((tops[0] - Vec3::new(0.0, 1.5, 0.0)).length() < EPSILON);
        assert!((tops[1] - Vec3::new(0.0, -1.5, 0.0)).length() < EPSILON);
    }

    #[test]
    fn bodies_are_drawn_where_their_nodes_are_but_not_scaled() {
        let shape = Shape2d::rectangle(Vec2::new(2.0, 1.0)).unwrap();
        let body = BodySettings::new(BodyKind::Static, shape);
        let global =
            Affine2::from_scale_rotation_translation(Vec2::new(3.0, 3.0), 0.0, Vec2::new(4.0, 1.0));
        let mut frame = Frame::new();

        draw_body_2d(&body, global, &MARKS, &mut frame);

        assert_eq!(frame.draw_count(), 1);
        let [(_, place)] = pieces_2d(&body.shape, &MARKS)[..] else {
            panic!("a rectangle is one square");
        };
        let corner = place.transform_point(Vec3::new(0.5, 0.5, 0.0));
        assert!((corner - Vec3::new(1.0, 0.5, 0.0)).length() < EPSILON);
    }
}
