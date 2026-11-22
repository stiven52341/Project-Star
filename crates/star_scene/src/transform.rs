//! Where nodes are: their own transform and the one in the world.

use star_core::EngineResult;
use star_ecs::{Entity, World};
use star_math::{Affine2, Vec2};

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

/// Computes the [`GlobalTransform2D`] of every node with a [`Transform2D`],
/// root first. A node without [`Transform2D`] passes its parent's transform
/// on to its children unchanged.
///
/// Keeps its work list between frames, so it allocates nothing once warm.
#[derive(Debug, Default)]
pub struct TransformPropagation {
    pending: Vec<(Entity, Affine2)>,
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
                pending.push((entity, Affine2::IDENTITY));
            }
        })?;
        world.each::<&Children>(|entity, _| {
            if !world.has::<Parent>(entity) && !world.has::<Transform2D>(entity) {
                pending.push((entity, Affine2::IDENTITY));
            }
        })?;
        while let Some((entity, parent_global)) = self.pending.pop() {
            let global = update_global(world, entity, parent_global)?;
            if let Some(children) = world.get::<Children>(entity) {
                self.pending
                    .extend(children.as_slice().iter().map(|&child| (child, global)));
            }
        }
        Ok(())
    }
}

/// Stores the global transform of `entity` under `parent_global`, and
/// returns the transform its children are under.
fn update_global(
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
}
