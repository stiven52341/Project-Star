//! Where nodes are in the world: the global positions of StarScript.
//!
//! A node's `x`, `y`, `rotation` and scales place it relative to its
//! parent, as the engine does: scaled, then turned, then moved. Its place
//! in the world is the places of its ancestors applied one after the
//! other. These are the plain affine transforms that compute it, so console
//! programs, which have no engine, compute the same.

/// A 2D affine transform: `linear` (by columns) then `translation`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Place2d {
    linear: [[f32; 2]; 2],
    translation: [f32; 2],
}

impl Place2d {
    /// No change: where the world is.
    pub const IDENTITY: Self = Self {
        linear: [[1.0, 0.0], [0.0, 1.0]],
        translation: [0.0, 0.0],
    };

    /// The place of a node at `x`, `y`, turned `rotation` radians
    /// counterclockwise and scaled by `scale_x`, `scale_y`, relative to its
    /// parent.
    #[must_use]
    pub fn of_node(x: f32, y: f32, rotation: f32, scale_x: f32, scale_y: f32) -> Self {
        let (sin, cos) = rotation.sin_cos();
        Self {
            linear: [
                [cos * scale_x, sin * scale_x],
                [-sin * scale_y, cos * scale_y],
            ],
            translation: [x, y],
        }
    }

    /// `self`, then `inner` inside it: the place of a child placed `inner`
    /// in a parent placed `self`.
    #[must_use]
    pub fn then(self, inner: Self) -> Self {
        let [x_axis, y_axis] = inner.linear;
        Self {
            linear: [self.vector(x_axis), self.vector(y_axis)],
            translation: self.point(inner.translation),
        }
    }

    /// Where `point` goes.
    #[must_use]
    pub fn point(self, point: [f32; 2]) -> [f32; 2] {
        let [x, y] = self.vector(point);
        [x + self.translation[0], y + self.translation[1]]
    }

    fn vector(self, [across, up]: [f32; 2]) -> [f32; 2] {
        let [x_axis, y_axis] = self.linear;
        [
            x_axis[0] * across + y_axis[0] * up,
            x_axis[1] * across + y_axis[1] * up,
        ]
    }

    /// The point placed at `point` in the world, in this place's own
    /// space; `None` if this place is scaled to nothing.
    #[must_use]
    pub fn local_point(self, point: [f32; 2]) -> Option<[f32; 2]> {
        let [x_axis, y_axis] = self.linear;
        let determinant = x_axis[0] * y_axis[1] - x_axis[1] * y_axis[0];
        if determinant == 0.0 || !determinant.is_finite() {
            return None;
        }
        let across = point[0] - self.translation[0];
        let up = point[1] - self.translation[1];
        Some([
            (y_axis[1] * across - y_axis[0] * up) / determinant,
            (x_axis[0] * up - x_axis[1] * across) / determinant,
        ])
    }
}

/// A 3D affine transform: `linear` (by columns) then `translation`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Place3d {
    linear: [[f32; 3]; 3],
    translation: [f32; 3],
}

impl Place3d {
    /// No change: where the world is.
    pub const IDENTITY: Self = Self {
        linear: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
        translation: [0.0, 0.0, 0.0],
    };

    /// The place of a 3D node at `position`, turned by `pitch`, `yaw` and
    /// `roll` radians (roll first, then pitch, then yaw, as the engine
    /// turns nodes) and scaled by `scale`, relative to its parent.
    #[must_use]
    pub fn of_node(position: [f32; 3], [pitch, yaw, roll]: [f32; 3], scale: [f32; 3]) -> Self {
        let turn = multiply(
            rotation_y(yaw),
            multiply(rotation_x(pitch), rotation_z(roll)),
        );
        let [x_axis, y_axis, z_axis] = turn;
        let scaled = |axis: [f32; 3], factor: f32| axis.map(|value| value * factor);
        Self {
            linear: [
                scaled(x_axis, scale[0]),
                scaled(y_axis, scale[1]),
                scaled(z_axis, scale[2]),
            ],
            translation: position,
        }
    }

    /// `self`, then `inner` inside it: the place of a child placed `inner`
    /// in a parent placed `self`.
    #[must_use]
    pub fn then(self, inner: Self) -> Self {
        Self {
            linear: multiply(self.linear, inner.linear),
            translation: self.point(inner.translation),
        }
    }

    /// Where `point` goes.
    #[must_use]
    pub fn point(self, point: [f32; 3]) -> [f32; 3] {
        let moved = apply(self.linear, point);
        [0, 1, 2].map(|axis| moved[axis] + self.translation[axis])
    }

    /// The point placed at `point` in the world, in this place's own
    /// space; `None` if this place is scaled to nothing.
    #[must_use]
    pub fn local_point(self, point: [f32; 3]) -> Option<[f32; 3]> {
        let inverse = inverse(self.linear)?;
        let offset = [0, 1, 2].map(|axis| point[axis] - self.translation[axis]);
        Some(apply(inverse, offset))
    }
}

/// A 3x3 matrix by columns.
type Matrix3 = [[f32; 3]; 3];

fn apply(matrix: Matrix3, [x, y, z]: [f32; 3]) -> [f32; 3] {
    [0, 1, 2].map(|row| matrix[0][row] * x + matrix[1][row] * y + matrix[2][row] * z)
}

fn multiply(left: Matrix3, right: Matrix3) -> Matrix3 {
    right.map(|column| apply(left, column))
}

fn rotation_x(angle: f32) -> Matrix3 {
    let (sin, cos) = angle.sin_cos();
    [[1.0, 0.0, 0.0], [0.0, cos, sin], [0.0, -sin, cos]]
}

fn rotation_y(angle: f32) -> Matrix3 {
    let (sin, cos) = angle.sin_cos();
    [[cos, 0.0, -sin], [0.0, 1.0, 0.0], [sin, 0.0, cos]]
}

fn rotation_z(angle: f32) -> Matrix3 {
    let (sin, cos) = angle.sin_cos();
    [[cos, sin, 0.0], [-sin, cos, 0.0], [0.0, 0.0, 1.0]]
}

/// The inverse of `matrix`, by its cofactors; `None` if it has none.
fn inverse(matrix: Matrix3) -> Option<Matrix3> {
    let [a, b, c] = matrix;
    let cross = |u: [f32; 3], v: [f32; 3]| {
        [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]
    };
    let rows = [cross(b, c), cross(c, a), cross(a, b)];
    let determinant = a[0] * rows[0][0] + a[1] * rows[0][1] + a[2] * rows[0][2];
    if determinant == 0.0 || !determinant.is_finite() {
        return None;
    }
    // The inverse's rows are the cross products, over the determinant.
    Some([0, 1, 2].map(|column| [0, 1, 2].map(|row| rows[row][column] / determinant)))
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;

    const EPSILON: f32 = 1e-5;

    fn close<const N: usize>(found: [f32; N], expected: [f32; N]) -> bool {
        found
            .iter()
            .zip(expected)
            .all(|(found, expected)| (found - expected).abs() < EPSILON)
    }

    #[test]
    fn children_are_placed_inside_their_turned_and_scaled_parent() {
        let parent = Place2d::of_node(10.0, 0.0, FRAC_PI_2, 2.0, 2.0);
        let child = Place2d::of_node(1.0, 0.0, 0.0, 1.0, 1.0);

        let global = parent.then(child).point([0.0, 0.0]);

        assert!(close(global, [10.0, 2.0]), "{global:?}");
    }

    #[test]
    fn world_points_go_back_to_local_ones() {
        let place = Place2d::of_node(3.0, -1.0, 0.7, 2.0, 0.5);

        let local = place.local_point(place.point([1.5, 4.0])).unwrap();

        assert!(close(local, [1.5, 4.0]), "{local:?}");
        assert_eq!(
            Place2d::of_node(0.0, 0.0, 0.0, 0.0, 1.0).local_point([1.0, 1.0]),
            None
        );
    }

    #[test]
    fn three_d_nodes_turn_as_the_engine_turns_them() {
        // Yaw a quarter turn: the node's -z (forward) points to -x.
        let place = Place3d::of_node([1.0, 2.0, 3.0], [0.0, FRAC_PI_2, 0.0], [1.0; 3]);

        let ahead = place.point([0.0, 0.0, -1.0]);

        assert!(close(ahead, [0.0, 2.0, 3.0]), "{ahead:?}");
    }

    #[test]
    fn three_d_world_points_go_back_to_local_ones() {
        let parent = Place3d::of_node([1.0, 2.0, 3.0], [0.3, -0.8, 0.2], [2.0, 1.0, 0.5]);
        let child = Place3d::of_node([0.5, 0.0, -1.0], [0.1, 0.2, 0.3], [1.0; 3]);
        let global = parent.then(child);

        let world = global.point([0.0; 3]);
        let local = parent.local_point(world).unwrap();

        assert!(close(local, [0.5, 0.0, -1.0]), "{local:?}");
    }
}
