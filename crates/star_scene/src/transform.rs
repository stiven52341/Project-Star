//! Where nodes are: their own transform and the one in the world.

use star_core::EngineResult;
use star_ecs::{Entity, World};
use star_math::{Affine2, Mat4, Quat, Vec2, Vec3};

use crate::{Children, Parent};

/// Where a 2D node is relative to its parent: moved by `position`, rotated
/// `rotation` radians counterclockwise and scaled by `scale`. Its children
/// move, rotate and scale with it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform2D {
    /// Offset from the parent's origin, in world units.
    pub position: Vec2,
    /// Counterclockwise angle, in radians.
    pub rotation: f32,
    /// Stretch along each axis; a negative factor mirrors.
    pub scale: Vec2,
}

impl Transform2D {
    /// No offset, rotation or scaling: where the parent is.
    pub const IDENTITY: Self = Self {
        position: Vec2::ZERO,
        rotation: 0.0,
        scale: Vec2::ONE,
    };

    /// Moved to `position`, without rotation or scaling.
    #[must_use]
    pub const fn from_position(position: Vec2) -> Self {
        Self {
            position,
            ..Self::IDENTITY
        }
    }

    /// The same transform rotated `rotation` radians counterclockwise.
    #[must_use]
    pub const fn with_rotation(mut self, rotation: f32) -> Self {
        self.rotation = rotation;
        self
    }

    /// The same transform scaled by `scale`.
    #[must_use]
    pub const fn with_scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }

    /// The transform as an affine transform from the node's space to its parent's.
    #[must_use]
    pub fn to_affine(self) -> Affine2 {
        Affine2::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

impl Default for Transform2D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Where a 2D node ends up in the world, after the transforms of all its
/// ancestors. The engine computes it from the [`Transform2D`]s at the end of
/// every frame, before drawing: change [`Transform2D`], not this.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GlobalTransform2D(Affine2);

impl GlobalTransform2D {
    /// The transform from the node's space to the world's.
    #[must_use]
    pub const fn affine(self) -> Affine2 {
        self.0
    }

    /// Where the node's origin is in the world.
    #[must_use]
    pub const fn position(self) -> Vec2 {
        self.0.translation()
    }

    /// The node's angle in the world, counterclockwise in radians.
    #[must_use]
    pub fn rotation(self) -> f32 {
        self.0.rotation()
    }

    /// The node's stretch in the world (exact unless an ancestor's
    /// non-uniform scale skews it).
    #[must_use]
    pub fn scale(self) -> Vec2 {
        self.0.scale()
    }
}

/// Where a 3D node is relative to its parent: scaled by `scale`, turned by
/// `rotation`, then moved by `position`. Its children move, turn and scale
/// with it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transform3D {
    /// Offset from the parent's origin, in world units.
    pub position: Vec3,
    /// How it is turned.
    pub rotation: Quat,
    /// Stretch along each axis; a negative factor mirrors.
    pub scale: Vec3,
}

impl Transform3D {
    /// No offset, rotation or scaling: where the parent is.
    pub const IDENTITY: Self = Self {
        position: Vec3::ZERO,
        rotation: Quat::IDENTITY,
        scale: Vec3::ONE,
    };

    /// Moved to `position`, without rotation or scaling.
    #[must_use]
    pub const fn from_position(position: Vec3) -> Self {
        Self {
            position,
            ..Self::IDENTITY
        }
    }

    /// The same transform turned by `rotation`.
    #[must_use]
    pub const fn with_rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    /// The same transform scaled by `scale`.
    #[must_use]
    pub const fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    /// The transform as a matrix from the node's space to its parent's.
    #[must_use]
    pub fn to_mat4(self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rotation, self.position)
    }
}

impl Default for Transform3D {
    fn default() -> Self {
        Self::IDENTITY
    }
}

/// Where a 3D node ends up in the world, after the transforms of all its
/// ancestors. The engine computes it from the [`Transform3D`]s at the end of
/// every frame, before drawing: change [`Transform3D`], not this.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GlobalTransform3D(Mat4);

impl GlobalTransform3D {
    /// The transform from the node's space to the world's.
    #[must_use]
    pub const fn matrix(self) -> Mat4 {
        self.0
    }

    /// Where the node's origin is in the world.
    #[must_use]
    pub const fn position(self) -> Vec3 {
        self.0.translation()
    }
}

/// The global transforms a node's children are under.
#[derive(Debug, Clone, Copy)]
struct Parents {
    flat: Affine2,
    solid: Mat4,
}

impl Parents {
    const WORLD: Self = Self {
        flat: Affine2::IDENTITY,
        solid: Mat4::IDENTITY,
    };
}

/// Computes the [`GlobalTransform2D`] of every node with a [`Transform2D`]
/// and the [`GlobalTransform3D`] of every node with a [`Transform3D`],
/// root first. A node without one passes its parent's on to its children
/// unchanged, so 2D and 3D nodes can share a tree without affecting each
/// other.
///
/// Keeps its work list between frames, so it allocates nothing once warm.
#[derive(Debug, Default)]
pub struct TransformPropagation {
    pending: Vec<(Entity, Parents)>,
}

impl TransformPropagation {
    /// A propagation without a work list yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Updates the global transforms of `world`.
    ///
    /// # Errors
    ///
    /// Returns [`EngineError::Ecs`](star_core::EngineError::Ecs) if the
    /// transforms or the tree are borrowed by a running query.
    pub fn run(&mut self, world: &mut World) -> EngineResult<()> {
        self.pending.clear();
        let pending = &mut self.pending;
        // Roots: nodes without a parent that have a transform or children.
        world.each::<&Transform2D>(|entity, _| {
            if !world.has::<Parent>(entity) {
                pending.push((entity, Parents::WORLD));
            }
        })?;
        world.each::<&Transform3D>(|entity, _| {
            if !world.has::<Parent>(entity) && !world.has::<Transform2D>(entity) {
                pending.push((entity, Parents::WORLD));
            }
        })?;
        world.each::<&Children>(|entity, _| {
            let placed = world.has::<Transform2D>(entity) || world.has::<Transform3D>(entity);
            if !world.has::<Parent>(entity) && !placed {
                pending.push((entity, Parents::WORLD));
            }
        })?;
        while let Some((entity, parents)) = self.pending.pop() {
            let globals = Parents {
                flat: update_global_2d(world, entity, parents.flat)?,
                solid: update_global_3d(world, entity, parents.solid)?,
            };
            if let Some(children) = world.get::<Children>(entity) {
                self.pending
                    .extend(children.as_slice().iter().map(|&child| (child, globals)));
            }
        }
        Ok(())
    }
}

/// Stores the global transform of `entity` under `parent_global`, and
/// returns the transform its children are under.
fn update_global_2d(
    world: &mut World,
    entity: Entity,
    parent_global: Affine2,
) -> EngineResult<Affine2> {
    let Some(local) = world.get::<Transform2D>(entity).map(|local| *local) else {
        return Ok(parent_global);
    };
    let global = parent_global * local.to_affine();
    let updated = world
        .get_mut::<GlobalTransform2D>(entity)
        .map(|mut stored| stored.0 = global)
        .is_some();
    if !updated {
        world.insert(entity, GlobalTransform2D(global))?;
    }
    Ok(global)
}

/// Stores the 3D global transform of `entity` under `parent_global`, and
/// returns the one its children are under.
fn update_global_3d(world: &mut World, entity: Entity, parent_global: Mat4) -> EngineResult<Mat4> {
    let Some(local) = world.get::<Transform3D>(entity).map(|local| *local) else {
        return Ok(parent_global);
    };
    let global = parent_global * local.to_mat4();
    let updated = world
        .get_mut::<GlobalTransform3D>(entity)
        .map(|mut stored| stored.0 = global)
        .is_some();
    if !updated {
        world.insert(entity, GlobalTransform3D(global))?;
    }
    Ok(global)
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;
    use crate::Hierarchy;

    const EPSILON: f32 = 1e-5;

    fn assert_close(actual: Vec2, expected: Vec2) {
        assert!(
            (actual - expected).length() < EPSILON,
            "expected {expected:?}, got {actual:?}"
        );
    }

    fn global_position(world: &World, node: Entity) -> Vec2 {
        world.get::<GlobalTransform2D>(node).unwrap().position()
    }

    #[test]
    fn children_move_rotate_and_scale_with_their_parent() {
        let mut world = World::new();
        let ship = world.spawn_node("Ship", None).unwrap();
        let cannon = world.spawn_node("Cannon", Some(ship)).unwrap();
        world
            .insert(
                ship,
                Transform2D::from_position(Vec2::new(10.0, 0.0))
                    .with_rotation(FRAC_PI_2)
                    .with_scale(Vec2::new(2.0, 2.0)),
            )
            .unwrap();
        world
            .insert(cannon, Transform2D::from_position(Vec2::new(1.0, 0.0)))
            .unwrap();

        TransformPropagation::new().run(&mut world).unwrap();

        assert_close(global_position(&world, ship), Vec2::new(10.0, 0.0));
        assert_close(global_position(&world, cannon), Vec2::new(10.0, 2.0));
        let cannon_global = *world.get::<GlobalTransform2D>(cannon).unwrap();
        assert!((cannon_global.rotation() - FRAC_PI_2).abs() < EPSILON);
        assert_close(cannon_global.scale(), Vec2::new(2.0, 2.0));
    }

    #[test]
    fn nodes_without_transform_pass_their_parents_on() {
        let mut world = World::new();
        let level = world.spawn_node("Level", None).unwrap();
        let group = world.spawn_node("Enemies", Some(level)).unwrap();
        let enemy = world.spawn_node("Enemy", Some(group)).unwrap();
        world
            .insert(level, Transform2D::from_position(Vec2::new(0.0, 5.0)))
            .unwrap();
        world
            .insert(enemy, Transform2D::from_position(Vec2::new(1.0, 0.0)))
            .unwrap();

        TransformPropagation::new().run(&mut world).unwrap();

        assert_close(global_position(&world, enemy), Vec2::new(1.0, 5.0));
        assert!(!world.has::<GlobalTransform2D>(group));
    }

    #[test]
    fn roots_without_transform_still_place_their_children() {
        let mut world = World::new();
        let level = world.spawn_node("Level", None).unwrap();
        let hero = world.spawn_node("Hero", Some(level)).unwrap();
        world
            .insert(hero, Transform2D::from_position(Vec2::new(3.0, 4.0)))
            .unwrap();

        TransformPropagation::new().run(&mut world).unwrap();

        assert_close(global_position(&world, hero), Vec2::new(3.0, 4.0));
    }

    #[test]
    fn global_transforms_follow_later_changes() {
        let mut world = World::new();
        let hero = world.spawn_node("Hero", None).unwrap();
        world.insert(hero, Transform2D::IDENTITY).unwrap();
        let mut propagation = TransformPropagation::new();
        propagation.run(&mut world).unwrap();

        world.get_mut::<Transform2D>(hero).unwrap().position = Vec2::new(7.0, 0.0);
        propagation.run(&mut world).unwrap();

        assert_close(global_position(&world, hero), Vec2::new(7.0, 0.0));
    }

    #[test]
    fn children_3d_move_turn_and_scale_with_their_parent() {
        let mut world = World::new();
        let ship = world.spawn_node("Ship", None).unwrap();
        let group = world.spawn_node("Group", Some(ship)).unwrap();
        let cannon = world.spawn_node("Cannon", Some(group)).unwrap();
        world
            .insert(
                ship,
                Transform3D::from_position(Vec3::new(0.0, 0.0, -10.0))
                    .with_rotation(Quat::from_rotation_y(FRAC_PI_2))
                    .with_scale(Vec3::ONE * 2.0),
            )
            .unwrap();
        world
            .insert(cannon, Transform3D::from_position(Vec3::new(1.0, 0.0, 0.0)))
            .unwrap();

        TransformPropagation::new().run(&mut world).unwrap();

        let global = world.get::<GlobalTransform3D>(cannon).unwrap().position();
        assert!(
            (global - Vec3::new(0.0, 0.0, -12.0)).length() < EPSILON,
            "{global:?}"
        );
        assert!(!world.has::<GlobalTransform3D>(group));
    }

    #[test]
    fn flat_and_solid_transforms_do_not_mix() {
        let mut world = World::new();
        let level = world.spawn_node("Level", None).unwrap();
        let flat = world.spawn_node("Flat", Some(level)).unwrap();
        world
            .insert(level, Transform3D::from_position(Vec3::new(5.0, 5.0, 5.0)))
            .unwrap();
        world.insert(flat, Transform2D::IDENTITY).unwrap();

        TransformPropagation::new().run(&mut world).unwrap();

        assert_close(global_position(&world, flat), Vec2::ZERO);
        assert!(!world.has::<GlobalTransform2D>(level));
    }
}
