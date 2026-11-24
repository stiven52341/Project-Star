//! Nodes moved to another parent in the tree stay where they are in the
//! world: their transform is recomputed relative to the new parent.

use star_engine::prelude::{Affine2, Mat4, Quat, Transform2D, Transform3D, Vec3};

/// The 2D transform that places a node at the same spot under a parent
/// that places its children at `to` as it was under one at `from`.
/// Parents scaled unevenly and rotated can skew a child, which a
/// transform cannot hold: the closest one is kept. If `to` cannot be
/// undone (a scale of zero), `local` is kept as it is.
pub(super) fn keep_in_place_2d(local: Transform2D, from: Affine2, to: Affine2) -> Transform2D {
    let Some(undo_parent) = to.inverse() else {
        return local;
    };
    let placed = undo_parent * from * local.to_affine();
    Transform2D {
        position: placed.translation(),
        rotation: placed.rotation(),
        scale: placed.scale(),
    }
}

/// The 3D transform that places a node at the same spot under a parent
/// that places its children at `to` as it was under one at `from`. If
/// `to` cannot be undone or the result mirrors (which a rotation cannot
/// hold), `local` is kept as it is.
pub(super) fn keep_in_place_3d(local: Transform3D, from: Mat4, to: Mat4) -> Transform3D {
    let Some(undo_parent) = to.inverse() else {
        return local;
    };
    let placed = undo_parent * from * local.to_mat4();
    let Some(rotation) = Quat::from_mat4(placed) else {
        return local;
    };
    let [x_axis, y_axis, z_axis, _] = placed.to_columns();
    let length = |[x, y, z, _]: [f32; 4]| Vec3::new(x, y, z).length();
    Transform3D {
        position: placed.translation(),
        rotation,
        scale: Vec3::new(length(x_axis), length(y_axis), length(z_axis)),
    }
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use star_engine::prelude::Vec2;

    use super::*;

    const EPSILON: f32 = 1e-4;

    fn close_2d(a: Vec2, b: Vec2) -> bool {
        (a - b).length() < EPSILON
    }

    fn close_3d(a: Vec3, b: Vec3) -> bool {
        (a - b).length() < EPSILON
    }

    #[test]
    fn nodes_moved_in_2d_keep_their_place_rotation_and_size() {
        let local = Transform2D {
            position: Vec2::new(1.0, 0.0),
            rotation: 0.0,
            scale: Vec2::new(1.0, 1.0),
        };
        let from = Affine2::from_scale_rotation_translation(
            Vec2::new(2.0, 2.0),
            FRAC_PI_2,
            Vec2::new(5.0, 0.0),
        );

        let kept = keep_in_place_2d(local, from, Affine2::IDENTITY);

        assert!(close_2d(kept.position, Vec2::new(5.0, 2.0)));
        assert!((kept.rotation - FRAC_PI_2).abs() < EPSILON);
        assert!(close_2d(kept.scale, Vec2::new(2.0, 2.0)));
        let world = |transform: Transform2D, parent: Affine2| {
            (parent * transform.to_affine()).translation()
        };
        assert!(close_2d(world(kept, Affine2::IDENTITY), world(local, from)));
    }

    #[test]
    fn nodes_moved_in_3d_keep_their_place_rotation_and_size() {
        let local = Transform3D {
            position: Vec3::new(0.0, 0.0, 1.0),
            ..Transform3D::IDENTITY
        };
        let from = Mat4::from_scale_rotation_translation(
            Vec3::new(3.0, 3.0, 3.0),
            Quat::from_rotation_y(FRAC_PI_2),
            Vec3::new(0.0, 1.0, 0.0),
        );
        let to = Mat4::from_translation(Vec3::new(0.0, 1.0, 0.0));

        let kept = keep_in_place_3d(local, from, to);

        assert!(close_3d(kept.position, Vec3::new(3.0, 0.0, 0.0)));
        assert!(close_3d(kept.scale, Vec3::new(3.0, 3.0, 3.0)));
        let turned = kept.rotation.rotate(Vec3::new(0.0, 0.0, 1.0));
        assert!(close_3d(turned, Vec3::new(1.0, 0.0, 0.0)));
    }

    #[test]
    fn parents_scaled_to_nothing_keep_the_transform() {
        let local = Transform2D::from_position(Vec2::new(1.0, 2.0));
        let flat = Affine2::from_scale_rotation_translation(Vec2::new(0.0, 1.0), 0.0, Vec2::ZERO);

        assert_eq!(keep_in_place_2d(local, Affine2::IDENTITY, flat), local);
    }
}
