//! Hiding nodes: a [`Hidden`] node is not drawn, and neither are its
//! children. Hidden nodes still move, collide and run their scripts; only
//! what they show and the light they shine are left out.

use star_core::EngineResult;
use star_ecs::{Entity, World};

/// Marks a node as hidden: neither it nor any node under it is drawn or
/// lights the scene.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Hidden;

/// Marks a node that is [`Hidden`], or under a hidden node, so it is not
/// drawn. Kept by [`TransformPropagation`](crate::TransformPropagation)
/// along with the global transforms; it is not set by hand.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct HiddenInTree;

/// Whether `entity` is drawn: neither it nor its ancestors are hidden, as
/// of the last [`TransformPropagation`](crate::TransformPropagation).
#[must_use]
pub fn is_shown(world: &World, entity: Entity) -> bool {
    !world.has::<HiddenInTree>(entity)
}

/// Marks `entity` as hidden in the tree if its parent is (`parent_hidden`)
/// or it is [`Hidden`] itself, and returns whether it is, for its children.
pub(crate) fn update_hidden(
    world: &mut World,
    entity: Entity,
    parent_hidden: bool,
) -> EngineResult<bool> {
    let hidden = parent_hidden || world.has::<Hidden>(entity);
    if hidden != world.has::<HiddenInTree>(entity) {
        if hidden {
            world.insert(entity, HiddenInTree)?;
        } else {
            world.remove::<HiddenInTree>(entity);
        }
    }
    Ok(hidden)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Hierarchy, Transform2D, TransformPropagation};

    #[test]
    fn hidden_nodes_hide_their_children_until_shown_again() {
        let mut world = World::new();
        let ship = world.spawn_node("Ship", None).unwrap();
        let cannon = world.spawn_node("Cannon", Some(ship)).unwrap();
        let other = world.spawn_node("Other", None).unwrap();
        for node in [ship, cannon, other] {
            world.insert(node, Transform2D::default()).unwrap();
        }
        let mut propagation = TransformPropagation::new();

        world.insert(ship, Hidden).unwrap();
        propagation.run(&mut world).unwrap();
        assert!(!is_shown(&world, ship));
        assert!(!is_shown(&world, cannon));
        assert!(is_shown(&world, other));

        world.remove::<Hidden>(ship);
        propagation.run(&mut world).unwrap();
        assert!(is_shown(&world, ship));
        assert!(is_shown(&world, cannon));
    }

    #[test]
    fn hidden_children_leave_their_parent_shown() {
        let mut world = World::new();
        let ship = world.spawn_node("Ship", None).unwrap();
        let cannon = world.spawn_node("Cannon", Some(ship)).unwrap();
        world.insert(ship, Transform2D::default()).unwrap();
        world.insert(cannon, Hidden).unwrap();

        TransformPropagation::new().run(&mut world).unwrap();

        assert!(is_shown(&world, ship));
        assert!(!is_shown(&world, cannon));
    }
}
