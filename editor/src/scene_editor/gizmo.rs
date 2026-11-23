//! The handles drawn over the node selected to move, turn and scale it
//! with the pointer, as the tools of other editors do: arrows along the
//! world's axes to move it, rings around its own axes to turn it, and
//! lines with boxes along its own axes to scale it. A square in the
//! middle moves it freely, or scales it the same on every axis.
//!
//! This module only does the geometry, in the view's pixels (from its top
//! left, Y downwards); the view paints the handles and reads the pointer.

use std::f32::consts::TAU;

use star_engine::prelude::{Affine2, Mat4, Quat, Transform2D, Transform3D, Vec2, Vec3};

use super::picking::Ray;

/// How long the handles are on screen, whatever the zoom.
const HANDLE_PIXELS: f32 = 90.0;
/// Half the side of the square in the middle.
const CENTER_HALF_PIXELS: f32 = 8.0;
/// How near the pointer must be to a handle's line to grab it.
const GRAB_PIXELS: f32 = 7.0;
/// Points along each ring of the rotate tool.
const RING_POINTS: usize = 64;
/// Pixels dragged, up and to the right, that double a node's size with
/// the square of the scale tool.
const UNIFORM_SCALE_PIXELS: f32 = 100.0;
/// The smallest a drag scales a node to, as a part of its size when the
/// drag began: a node scaled to nothing could not be scaled back.
const MIN_SCALE_FACTOR: f32 = 0.01;
/// Lines closer than this to parallel never cross a pointer's ray.
const PARALLEL_EPSILON: f32 = 1e-6;
/// Depths the GPU keeps, in its screen space: points outside are not seen.
const NEAR_DEPTH: f32 = 0.0;
const FAR_DEPTH: f32 = 1.0;

/// What a drag in the view does.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum Tool {
    /// Only moves the camera.
    View,
    #[default]
    Move,
    Rotate,
    Scale,
}

/// One part of the handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Handle {
    /// Along or around an axis: 0 for X, 1 for Y, 2 for Z.
    Axis(usize),
    /// The line between X and Y of the scale tool, which scales both.
    AxesXY,
    /// The square in the middle.
    Center,
}

/// How a handle is drawn: a line through `points`, closed for rings,
/// ending in an arrow or a box for axes.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct HandleShape {
    pub(crate) handle: Handle,
    pub(crate) points: Vec<Vec2>,
    pub(crate) closed: bool,
    pub(crate) tip: Tip,
}

/// The end of a handle's line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tip {
    None,
    Arrow,
    Box,
}

/// How the view's camera sees the world: where world points are on screen
/// and which ray each pixel looks along.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Lens {
    view_projection: Mat4,
    to_world: Mat4,
    /// The view's size, in the same pixels as the points given.
    size: Vec2,
}

impl Lens {
    /// The lens of a camera whose matrix is `view_projection`, seen in a
    /// view of `size`. `None` if the matrix cannot be undone or the view
    /// is empty.
    pub(crate) fn new(view_projection: Mat4, size: Vec2) -> Option<Self> {
        (size.x > 0.0 && size.y > 0.0).then_some(())?;
        Some(Self {
            view_projection,
            to_world: view_projection.inverse()?,
            size,
        })
    }

    /// Where `world` is seen, or `None` if it is behind the camera or out
    /// of its depth.
    pub(crate) fn screen_point(&self, world: Vec3) -> Option<Vec2> {
        let seen = self.view_projection.transform_point(world);
        (NEAR_DEPTH..=FAR_DEPTH).contains(&seen.z).then_some(())?;
        Some(Vec2::new(
            f32::midpoint(seen.x, 1.0) * self.size.x,
            f32::midpoint(1.0, -seen.y) * self.size.y,
        ))
    }

    /// The ray seen at `pixel`.
    pub(crate) fn ray(&self, pixel: Vec2) -> Ray {
        let screen = Vec3::new(
            pixel.x / self.size.x * 2.0 - 1.0,
            1.0 - pixel.y / self.size.y * 2.0,
            NEAR_DEPTH,
        );
        let near = self.to_world.transform_point(screen);
        let far = self
            .to_world
            .transform_point(Vec3::new(screen.x, screen.y, FAR_DEPTH));
        Ray {
            origin: near,
            direction: far - near,
        }
    }

    /// How long one pixel is in the world, at the depth of `world`.
    fn world_per_pixel(&self, world: Vec3) -> Option<f32> {
        let seen = self.view_projection.transform_point(world);
        let next = Vec3::new(seen.x + 2.0 / self.size.x, seen.y, seen.z);
        Some((self.to_world.transform_point(next) - world).length())
            .filter(|length| length.is_finite() && *length > 0.0)
    }
}

/// The node the handles move: where its parent puts it, and its own
/// transform, in 2D or in 3D.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Target {
    Flat { parent: Affine2, local: Transform2D },
    Solid { parent: Mat4, local: Transform3D },
}

impl Target {
    /// Where the node is in the world.
    fn origin(&self) -> Vec3 {
        match self {
            Self::Flat { parent, local } => {
                let point = parent.transform_point(local.position);
                Vec3::new(point.x, point.y, 0.0)
            }
            Self::Solid { parent, local } => parent.transform_point(local.position),
        }
    }

    /// The axes of the tool: the world's for moving, the node's own,
    /// turned as it is, for turning and scaling.
    fn axes(&self, tool: Tool) -> Vec<Vec3> {
        let world = [Vec3::new(1.0, 0.0, 0.0), Vec3::UP, Vec3::new(0.0, 0.0, 1.0)];
        match (self, tool) {
            (Self::Flat { .. }, Tool::Rotate) => vec![world[2]],
            (Self::Flat { .. }, _) if tool == Tool::Move => world[..2].to_vec(),
            (Self::Flat { parent, local }, _) => {
                let global = *parent * local.to_affine();
                [Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)]
                    .into_iter()
                    .map(|axis| {
                        let along = global.transform_vector(axis);
                        unit(Vec3::new(along.x, along.y, 0.0))
                    })
                    .collect()
            }
            (Self::Solid { .. }, Tool::Move) => world.to_vec(),
            (Self::Solid { parent, local }, _) => {
                let global = *parent * local.to_mat4();
                world
                    .into_iter()
                    .map(|axis| unit(global.transform_point(axis) - global.translation()))
                    .collect()
            }
        }
    }

    /// The index of an axis among those of [`Target::axes`], as handles
    /// name them: 2D rings turn around Z.
    const fn axis_name(&self, tool: Tool, position: usize) -> usize {
        match (self, tool) {
            (Self::Flat { .. }, Tool::Rotate) => 2,
            _ => position,
        }
    }
}

/// The handles of `tool` for `target`, seen through `lens`. None for the
/// view tool, or when the node is out of sight.
pub(crate) fn shapes(tool: Tool, target: &Target, lens: &Lens) -> Vec<HandleShape> {
    if tool == Tool::View {
        return Vec::new();
    }
    let origin = target.origin();
    let (Some(center), Some(per_pixel)) = (lens.screen_point(origin), lens.world_per_pixel(origin))
    else {
        return Vec::new();
    };
    let length = HANDLE_PIXELS * per_pixel;
    let mut shapes: Vec<HandleShape> = target
        .axes(tool)
        .into_iter()
        .enumerate()
        .filter_map(|(position, axis)| {
            let handle = Handle::Axis(target.axis_name(tool, position));
            if tool == Tool::Rotate {
                let points = ring(origin, axis, length)
                    .filter_map(|point| lens.screen_point(point))
                    .collect();
                return Some(HandleShape {
                    handle,
                    points,
                    closed: true,
                    tip: Tip::None,
                });
            }
            let end = lens.screen_point(origin + axis * length)?;
            Some(HandleShape {
                handle,
                points: vec![center, end],
                closed: false,
                tip: if tool == Tool::Move {
                    Tip::Arrow
                } else {
                    Tip::Box
                },
            })
        })
        .collect();
    if tool == Tool::Scale
        && let Some(end) = diagonal(target, tool)
            .and_then(|direction| lens.screen_point(origin + direction * length))
    {
        shapes.push(HandleShape {
            handle: Handle::AxesXY,
            points: vec![center, end],
            closed: false,
            tip: Tip::Box,
        });
    }
    if tool != Tool::Rotate {
        shapes.push(HandleShape {
            handle: Handle::Center,
            points: square(center),
            closed: true,
            tip: Tip::None,
        });
    }
    shapes
}

/// The handle under `pointer`: the square in the middle first, then the
/// line nearest the pointer, if near enough.
pub(crate) fn handle_at(shapes: &[HandleShape], pointer: Vec2) -> Option<Handle> {
    let center = shapes.iter().find(|shape| {
        shape.handle == Handle::Center
            && shape
                .points
                .first()
                .zip(shape.points.get(2))
                .is_some_and(|(low, high)| {
                    (low.x..=high.x).contains(&pointer.x) && (low.y..=high.y).contains(&pointer.y)
                })
    });
    if let Some(center) = center {
        return Some(center.handle);
    }
    shapes
        .iter()
        .filter(|shape| shape.handle != Handle::Center)
        .map(|shape| (shape.handle, distance_to_line(shape, pointer)))
        .filter(|(_, distance)| *distance <= GRAB_PIXELS)
        .min_by(|(_, first), (_, second)| first.total_cmp(second))
        .map(|(handle, _)| handle)
}

/// A handle being dragged: what it does, to which node as it was when the
/// drag began, and where the drag began.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Drag {
    pub(crate) tool: Tool,
    pub(crate) handle: Handle,
    pub(crate) start: Target,
    pub(crate) from: Vec2,
}

impl Drag {
    /// The node as the drag leaves it with the pointer at `to`, or `None`
    /// if the pointer says nothing (a ray along the axis moved).
    pub(crate) fn moved_to(&self, lens: &Lens, to: Vec2) -> Option<Target> {
        match self.tool {
            Tool::View => None,
            Tool::Move => self.moved(lens, to),
            Tool::Rotate => self.rotated(lens, to),
            Tool::Scale => self.scaled(lens, to),
        }
    }

    fn axis(&self) -> Option<(usize, Vec3)> {
        let Handle::Axis(name) = self.handle else {
            return None;
        };
        let position = match self.start {
            Target::Flat { .. } if self.tool == Tool::Rotate => 0,
            _ => name,
        };
        Some((name, *self.start.axes(self.tool).get(position)?))
    }

    fn moved(&self, lens: &Lens, to: Vec2) -> Option<Target> {
        let origin = self.start.origin();
        let (from, to) = (lens.ray(self.from), lens.ray(to));
        let offset = if let Some((_, axis)) = self.axis() {
            axis * (along_line(origin, axis, to)? - along_line(origin, axis, from)?)
        } else {
            let facing = match self.start {
                Target::Flat { .. } => Vec3::new(0.0, 0.0, 1.0),
                Target::Solid { .. } => -from.direction,
            };
            on_plane(origin, facing, to)? - on_plane(origin, facing, from)?
        };
        let world = origin + offset;
        Some(match self.start {
            Target::Flat { parent, local } => Target::Flat {
                parent,
                local: Transform2D {
                    position: parent
                        .inverse()?
                        .transform_point(Vec2::new(world.x, world.y)),
                    ..local
                },
            },
            Target::Solid { parent, local } => Target::Solid {
                parent,
                local: Transform3D {
                    position: parent.inverse()?.transform_point(world),
                    ..local
                },
            },
        })
    }

    fn rotated(&self, lens: &Lens, to: Vec2) -> Option<Target> {
        let (name, axis) = self.axis()?;
        let origin = self.start.origin();
        let center = lens.screen_point(origin)?;
        let turned = screen_angle(self.from - center, to - center);
        // Turning counterclockwise on screen turns around an axis that
        // points at the viewer counterclockwise too.
        let toward_viewer = -lens.ray(center).direction;
        let angle = if axis.dot(toward_viewer) >= 0.0 {
            turned
        } else {
            -turned
        };
        Some(match self.start {
            Target::Flat { parent, local } => Target::Flat {
                parent,
                local: Transform2D {
                    rotation: local.rotation + angle,
                    ..local
                },
            },
            Target::Solid { parent, local } => {
                let own_axis = [Vec3::new(1.0, 0.0, 0.0), Vec3::UP, Vec3::new(0.0, 0.0, 1.0)]
                    .get(name)
                    .copied()?;
                Target::Solid {
                    parent,
                    local: Transform3D {
                        rotation: local.rotation * Quat::from_axis_angle(own_axis, angle)?,
                        ..local
                    },
                }
            }
        })
    }

    fn scaled(&self, lens: &Lens, to: Vec2) -> Option<Target> {
        let factors = match self.handle {
            Handle::Axis(name) => {
                let (_, axis) = self.axis()?;
                let mut factors = [1.0; 3];
                *factors.get_mut(name)? = self.ratio_along(lens, axis, to)?;
                factors
            }
            Handle::AxesXY => {
                let factor = self.ratio_along(lens, diagonal(&self.start, self.tool)?, to)?;
                [factor, factor, 1.0]
            }
            Handle::Center => {
                let dragged = to - self.from;
                let factor =
                    (1.0 + (dragged.x - dragged.y) / UNIFORM_SCALE_PIXELS).max(MIN_SCALE_FACTOR);
                [factor; 3]
            }
        };
        Some(match self.start {
            Target::Flat { parent, local } => Target::Flat {
                parent,
                local: Transform2D {
                    scale: Vec2::new(local.scale.x * factors[0], local.scale.y * factors[1]),
                    ..local
                },
            },
            Target::Solid { parent, local } => Target::Solid {
                parent,
                local: Transform3D {
                    scale: Vec3::new(
                        local.scale.x * factors[0],
                        local.scale.y * factors[1],
                        local.scale.z * factors[2],
                    ),
                    ..local
                },
            },
        })
    }
}

impl Drag {
    /// How much farther from the node, along `axis`, the pointer at `to`
    /// is than it was when the drag began: the factor it scales by.
    fn ratio_along(&self, lens: &Lens, axis: Vec3, to: Vec2) -> Option<f32> {
        let origin = self.start.origin();
        let from = along_line(origin, axis, lens.ray(self.from))?;
        let to = along_line(origin, axis, lens.ray(to))?;
        (from.abs() >= PARALLEL_EPSILON).then(|| (to / from).max(MIN_SCALE_FACTOR))
    }
}

/// The direction halfway between the X and Y axes of `tool` for `target`.
fn diagonal(target: &Target, tool: Tool) -> Option<Vec3> {
    let axes = target.axes(tool);
    let (x, y) = (axes.first()?, axes.get(1)?);
    (*x + *y).normalized()
}

fn unit(vector: Vec3) -> Vec3 {
    vector.normalized().unwrap_or(vector)
}

/// Points around `center`, `radius` away, on the plane across `axis`.
fn ring(center: Vec3, axis: Vec3, radius: f32) -> impl Iterator<Item = Vec3> {
    // Any direction not along the axis gives two across it.
    let helper = if axis.x.abs() < axis.y.abs().max(axis.z.abs()) {
        Vec3::new(1.0, 0.0, 0.0)
    } else {
        Vec3::UP
    };
    let first = unit(axis.cross(helper));
    let second = unit(axis.cross(first));
    (0..RING_POINTS).map(move |step| {
        let angle = TAU * fraction(step, RING_POINTS);
        let (sin, cos) = angle.sin_cos();
        center + (first * cos + second * sin) * radius
    })
}

/// `part / whole`, as a number.
// Rings have a few dozen points, far below where `f32` loses whole numbers.
#[allow(clippy::cast_precision_loss)]
fn fraction(part: usize, whole: usize) -> f32 {
    part as f32 / whole as f32
}

/// The corners of the square in the middle, from its top left clockwise.
fn square(center: Vec2) -> Vec<Vec2> {
    let half = CENTER_HALF_PIXELS;
    vec![
        Vec2::new(center.x - half, center.y - half),
        Vec2::new(center.x + half, center.y - half),
        Vec2::new(center.x + half, center.y + half),
        Vec2::new(center.x - half, center.y + half),
    ]
}

/// How far `pointer` is from the line of `shape`.
fn distance_to_line(shape: &HandleShape, pointer: Vec2) -> f32 {
    let points = &shape.points;
    let closing = shape
        .closed
        .then(|| points.last().zip(points.first()))
        .flatten();
    points
        .windows(2)
        .filter_map(|pair| pair.first().zip(pair.get(1)))
        .chain(closing)
        .map(|(start, end)| distance_to_segment(pointer, *start, *end))
        .fold(f32::INFINITY, f32::min)
}

fn distance_to_segment(point: Vec2, start: Vec2, end: Vec2) -> f32 {
    let along = end - start;
    let length_squared = along.x * along.x + along.y * along.y;
    let part = if length_squared > 0.0 {
        let to_point = point - start;
        ((to_point.x * along.x + to_point.y * along.y) / length_squared).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (point - (start + along * part)).length()
}

/// How far along the line through `origin` in direction `axis` (a unit
/// vector) is the point nearest to `ray`.
fn along_line(origin: Vec3, axis: Vec3, ray: Ray) -> Option<f32> {
    let between = origin - ray.origin;
    let across = axis.dot(ray.direction);
    let ray_length = ray.direction.dot(ray.direction);
    let denominator = across * across - ray_length;
    if denominator.abs() < PARALLEL_EPSILON * ray_length {
        return None;
    }
    let offset_on_axis = axis.dot(between);
    let offset_on_ray = ray.direction.dot(between);
    Some((ray_length * offset_on_axis - across * offset_on_ray) / denominator)
}

/// Where `ray` meets the plane through `origin` facing `normal`.
fn on_plane(origin: Vec3, normal: Vec3, ray: Ray) -> Option<Vec3> {
    let facing = normal.dot(ray.direction);
    if facing.abs() < PARALLEL_EPSILON {
        return None;
    }
    Some(ray.at(normal.dot(origin - ray.origin) / facing))
}

/// The angle, counterclockwise as seen, from `from` to `to`, two
/// directions in pixels (Y downwards).
fn screen_angle(from: Vec2, to: Vec2) -> f32 {
    let (from, to) = (Vec2::new(from.x, -from.y), Vec2::new(to.x, -to.y));
    let cross = from.x * to.y - from.y * to.x;
    let dot = from.x * to.x + from.y * to.y;
    cross.atan2(dot)
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    const EPSILON: f32 = 1e-3;
    /// A 2D view 10 units tall, centered on the origin, 200 pixels square:
    /// 20 pixels a unit.
    const VIEW: f32 = 200.0;

    fn lens_2d() -> Lens {
        let view_projection = Mat4::orthographic(-5.0, 5.0, -5.0, 5.0, 1.0, 20.0)
            * Mat4::look_at(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::UP).unwrap();
        Lens::new(view_projection, Vec2::new(VIEW, VIEW)).unwrap()
    }

    fn flat(position: Vec2) -> Target {
        Target::Flat {
            parent: Affine2::from_scale_rotation_translation(Vec2::new(2.0, 2.0), 0.0, Vec2::ZERO),
            local: Transform2D::from_position(position),
        }
    }

    fn drag(tool: Tool, handle: Handle, start: Target, from: Vec2, to: Vec2) -> Transform2D {
        let drag = Drag {
            tool,
            handle,
            start,
            from,
        };
        match drag.moved_to(&lens_2d(), to).unwrap() {
            Target::Flat { local, .. } => local,
            Target::Solid { .. } => panic!("a 2D node stays 2D"),
        }
    }

    #[test]
    fn world_points_are_seen_where_the_camera_puts_them() {
        let lens = lens_2d();

        let seen = lens.screen_point(Vec3::new(1.0, 1.0, 0.0)).unwrap();

        assert!((seen - Vec2::new(120.0, 80.0)).length() < EPSILON);
        assert!((lens.world_per_pixel(Vec3::ZERO).unwrap() - 0.05).abs() < EPSILON);
    }

    #[test]
    fn the_move_tool_has_two_arrows_and_a_square_in_2d() {
        let shapes = shapes(Tool::Move, &flat(Vec2::ZERO), &lens_2d());

        let handles: Vec<_> = shapes.iter().map(|shape| shape.handle).collect();
        assert_eq!(handles, [Handle::Axis(0), Handle::Axis(1), Handle::Center]);
    }

    #[test]
    fn the_scale_tool_has_a_line_between_x_and_y() {
        let shapes = shapes(Tool::Scale, &flat(Vec2::ZERO), &lens_2d());

        let handles: Vec<_> = shapes.iter().map(|shape| shape.handle).collect();
        assert_eq!(
            handles,
            [
                Handle::Axis(0),
                Handle::Axis(1),
                Handle::AxesXY,
                Handle::Center
            ]
        );
        let x_end = shapes[0].points[1];
        assert!((x_end - Vec2::new(100.0 + HANDLE_PIXELS, 100.0)).length() < EPSILON);
    }

    #[test]
    fn handles_are_grabbed_near_their_lines() {
        let shapes = shapes(Tool::Move, &flat(Vec2::ZERO), &lens_2d());

        assert_eq!(
            handle_at(&shapes, Vec2::new(100.0, 100.0)),
            Some(Handle::Center)
        );
        assert_eq!(
            handle_at(&shapes, Vec2::new(150.0, 103.0)),
            Some(Handle::Axis(0))
        );
        assert_eq!(
            handle_at(&shapes, Vec2::new(98.0, 40.0)),
            Some(Handle::Axis(1))
        );
        assert_eq!(handle_at(&shapes, Vec2::new(150.0, 150.0)), None);
    }

    #[test]
    fn moving_along_an_axis_ignores_the_rest_of_the_drag() {
        let moved = drag(
            Tool::Move,
            Handle::Axis(0),
            flat(Vec2::ZERO),
            Vec2::new(150.0, 100.0),
            Vec2::new(190.0, 130.0),
        );

        // 40 pixels are 2 world units, 1 inside a parent scaled by 2.
        assert!((moved.position - Vec2::new(1.0, 0.0)).length() < EPSILON);
    }

    #[test]
    fn the_square_moves_freely() {
        let moved = drag(
            Tool::Move,
            Handle::Center,
            flat(Vec2::new(1.0, 1.0)),
            Vec2::new(140.0, 60.0),
            Vec2::new(160.0, 100.0),
        );

        assert!((moved.position - Vec2::new(1.5, 0.0)).length() < EPSILON);
    }

    #[test]
    fn turning_counterclockwise_turns_the_node_counterclockwise() {
        let turned = drag(
            Tool::Rotate,
            Handle::Axis(2),
            flat(Vec2::ZERO),
            Vec2::new(150.0, 100.0),
            Vec2::new(100.0, 50.0),
        );

        assert!((turned.rotation - FRAC_PI_2).abs() < EPSILON);
    }

    #[test]
    fn scaling_along_an_axis_follows_the_pointer() {
        let scaled = drag(
            Tool::Scale,
            Handle::Axis(0),
            flat(Vec2::ZERO),
            Vec2::new(150.0, 100.0),
            Vec2::new(200.0, 100.0),
        );

        assert!((scaled.scale - Vec2::new(2.0, 1.0)).length() < EPSILON);
    }

    #[test]
    fn the_line_between_x_and_y_scales_both() {
        let scaled = drag(
            Tool::Scale,
            Handle::AxesXY,
            flat(Vec2::ZERO),
            Vec2::new(130.0, 70.0),
            Vec2::new(160.0, 40.0),
        );

        assert!((scaled.scale - Vec2::new(2.0, 2.0)).length() < EPSILON);
    }

    #[test]
    fn scaling_never_reaches_zero() {
        let scaled = drag(
            Tool::Scale,
            Handle::Center,
            flat(Vec2::ZERO),
            Vec2::new(100.0, 100.0),
            Vec2::new(0.0, 300.0),
        );

        assert!((scaled.scale.x - MIN_SCALE_FACTOR).abs() < EPSILON);
    }

    #[test]
    fn three_d_rings_turn_around_the_nodes_own_axes() {
        let view_projection = Mat4::perspective(1.0, 1.0, 0.1, 100.0)
            * Mat4::look_at(Vec3::new(0.0, 0.0, 10.0), Vec3::ZERO, Vec3::UP).unwrap();
        let lens = Lens::new(view_projection, Vec2::new(VIEW, VIEW)).unwrap();
        let start = Target::Solid {
            parent: Mat4::IDENTITY,
            local: Transform3D::IDENTITY,
        };

        let rings = shapes(Tool::Rotate, &start, &lens);
        let drag = Drag {
            tool: Tool::Rotate,
            handle: Handle::Axis(2),
            start,
            from: Vec2::new(150.0, 100.0),
        };
        let Some(Target::Solid { local, .. }) = drag.moved_to(&lens, Vec2::new(100.0, 50.0)) else {
            panic!("a 3D node stays 3D");
        };

        assert_eq!(rings.len(), 3);
        let turned = local.rotation.rotate(Vec3::new(1.0, 0.0, 0.0));
        assert!((turned - Vec3::UP).length() < EPSILON);
    }
}
