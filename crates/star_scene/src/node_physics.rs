//! The 2D physics of the nodes: bodies that follow their nodes, and nodes
//! that follow their bodies.

use std::collections::HashMap;

use star_core::{EngineError, EngineResult};
use star_ecs::{Entity, World};
use star_math::{Affine2, Vec2};
use star_physics::{BodyId, BodyKind, ContactPhase, Physics2d};

use crate::{Body2D, GlobalTransform2D, Parent, Transform2D};

/// Two nodes whose bodies started or stopped touching during a frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeContact {
    /// One of the nodes.
    pub first: Entity,
    /// The other one.
    pub second: Entity,
    /// Whether it begins or ends.
    pub phase: ContactPhase,
    /// Whether one of them is a sensor, so they overlap instead of
    /// colliding.
    pub is_sensor: bool,
}

impl NodeContact {
    /// Whether `node` is one of the two.
    #[must_use]
    pub fn involves(&self, node: Entity) -> bool {
        self.first == node || self.second == node
    }

    /// The node touching `node`, if `node` is one of the two.
    #[must_use]
    pub fn other(&self, node: Entity) -> Option<Entity> {
        if self.first == node {
            Some(self.second)
        } else if self.second == node {
            Some(self.first)
        } else {
            None
        }
    }
}

/// Which node each body of the [`Physics2d`] belongs to, and the contacts
/// of the last frame between nodes. A resource of the world, kept by
/// [`step_node_physics`].
#[derive(Debug, Default)]
pub struct PhysicsNodes {
    nodes: HashMap<BodyId, Entity>,
    /// Bodies removed this frame, whose last contacts still name them.
    departed: Vec<(BodyId, Entity)>,
    contacts: Vec<NodeContact>,
}

impl PhysicsNodes {
    /// No bodies yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The node `body` belongs to.
    #[must_use]
    pub fn node(&self, body: BodyId) -> Option<Entity> {
        self.nodes.get(&body).copied()
    }

    /// The nodes whose bodies started or stopped touching during the last
    /// frame, in the order it happened. A removed node still appears in
    /// the contacts that ended with it.
    #[must_use]
    pub fn contacts(&self) -> &[NodeContact] {
        &self.contacts
    }

    fn node_or_departed(&self, body: BodyId) -> Option<Entity> {
        self.node(body).or_else(|| {
            self.departed
                .iter()
                .find(|(departed, _)| *departed == body)
                .map(|(_, node)| *node)
        })
    }
}

/// Runs the physics of the nodes for a frame that took `elapsed` seconds,
/// with the [`Physics2d`] and [`PhysicsNodes`] resources of `world` (without
/// them, nothing happens):
///
/// 1. removes the bodies of nodes that are gone or lost their [`Body2D`];
/// 2. adds the bodies of new [`Body2D`]s where their nodes are, and moves
///    to their nodes the bodies whose node the game moved;
/// 3. gives the bodies the velocities and impulses set on their
///    [`Body2D`]s;
/// 4. advances the simulation;
/// 5. writes where the dynamic and kinematic bodies ended up into their
///    nodes' [`Transform2D`], and their velocities into their [`Body2D`];
/// 6. turns the contacts of the bodies into contacts of their nodes.
///
/// It places bodies with the [`GlobalTransform2D`]s of their parents, so
/// those must be up to date.
///
/// # Errors
///
/// [`EngineError::Physics`], naming the node, if a body cannot be added;
/// [`EngineError::Ecs`] if a running query holds the components it uses.
pub fn step_node_physics(world: &World, elapsed: f32) -> EngineResult<()> {
    let (Some(mut physics), Some(mut nodes)) = (
        world.resource_mut::<Physics2d>(),
        world.resource_mut::<PhysicsNodes>(),
    ) else {
        return Ok(());
    };
    remove_departed_bodies(world, &mut physics, &mut nodes);
    place_bodies(world, &mut physics, &mut nodes)?;
    push_bodies(world, &mut physics)?;
    physics.advance(elapsed);
    follow_bodies(world, &physics)?;
    collect_contacts(&physics, &mut nodes);
    Ok(())
}

fn remove_departed_bodies(world: &World, physics: &mut Physics2d, nodes: &mut PhysicsNodes) {
    let PhysicsNodes {
        nodes, departed, ..
    } = nodes;
    departed.clear();
    nodes.retain(|&body, &mut node| {
        let kept = world
            .get::<Body2D>(node)
            .is_some_and(|component| component.id() == Some(body));
        if !kept {
            physics.remove_body(body);
            departed.push((body, node));
        }
        kept
    });
}

fn place_bodies(
    world: &World,
    physics: &mut Physics2d,
    nodes: &mut PhysicsNodes,
) -> EngineResult<()> {
    let mut failure = None;
    world.each::<(&mut Body2D, &Transform2D)>(|node, (body, local)| {
        if failure.is_some() || !body.was_moved(local) {
            return;
        }
        let global = parent_global(world, node) * local.to_affine();
        let (position, rotation) = (global.translation(), global.rotation());
        match body.id() {
            Some(id) => {
                physics.set_position(id, position);
                physics.set_rotation(id, rotation);
                body.wrote(*local);
            }
            None => match physics.add_body(body.settings(), position, rotation) {
                Ok(id) => {
                    body.placed(id, *local);
                    nodes.nodes.insert(id, node);
                }
                Err(error) => failure = Some(named_error(world, node, &error)),
            },
        }
    })?;
    failure.map_or(Ok(()), Err)
}

fn push_bodies(world: &World, physics: &mut Physics2d) -> EngineResult<()> {
    world.each::<&mut Body2D>(|_, body| {
        let Some(id) = body.id() else {
            return;
        };
        let (velocity, impulse) = body.take_motion();
        if let Some(velocity) = velocity {
            physics.set_velocity(id, velocity);
        }
        if impulse != Vec2::ZERO {
            physics.apply_impulse(id, impulse);
        }
    })
}

fn follow_bodies(world: &World, physics: &Physics2d) -> EngineResult<()> {
    world.each::<(&mut Body2D, &mut Transform2D)>(|node, (body, local)| {
        let Some(id) = body.id() else {
            return;
        };
        if body.settings().kind == BodyKind::Static {
            return;
        }
        let (Some(position), Some(rotation), Some(velocity)) = (
            physics.position(id),
            physics.rotation(id),
            physics.velocity(id),
        ) else {
            return;
        };
        body.measured(velocity);
        let parent = parent_global(world, node);
        // A parent scaled to nothing has no inside to place the node in.
        if let Some(to_parent) = parent.inverse() {
            local.position = to_parent.transform_point(position);
            local.rotation = rotation - parent.rotation();
            body.wrote(*local);
        }
    })
}

fn collect_contacts(physics: &Physics2d, nodes: &mut PhysicsNodes) {
    nodes.contacts.clear();
    for contact in physics.contacts() {
        if let (Some(first), Some(second)) = (
            nodes.node_or_departed(contact.first),
            nodes.node_or_departed(contact.second),
        ) {
            nodes.contacts.push(NodeContact {
                first,
                second,
                phase: contact.phase,
                is_sensor: contact.is_sensor,
            });
        }
    }
}

/// Where the space of `node`'s parent is in the world: the global transform
/// of its closest ancestor that has one.
fn parent_global(world: &World, node: Entity) -> Affine2 {
    let mut current = node;
    while let Some(parent) = world.get::<Parent>(current).map(|parent| parent.entity()) {
        if let Some(global) = world.get::<GlobalTransform2D>(parent) {
            return global.affine();
        }
        current = parent;
    }
    Affine2::IDENTITY
}

fn named_error(world: &World, node: Entity, error: &EngineError) -> EngineError {
    let name = world
        .get::<crate::Name>(node)
        .map_or_else(|| format!("{node:?}"), |name| name.as_str().to_owned());
    EngineError::Physics(format!("node \"{name}\": {error}"))
}

#[cfg(test)]
mod tests {
    use star_physics::{BodySettings, Shape2d};

    use super::*;
    use crate::{Hierarchy, TransformPropagation};

    /// Seconds of a test frame: a 60 Hz screen.
    const FRAME: f32 = 1.0 / 60.0;
    const TOLERANCE: f32 = 1e-3;

    fn world() -> World {
        let mut world = World::new();
        world.insert_resource(Physics2d::default());
        world.insert_resource(PhysicsNodes::new());
        world
    }

    fn body_node(world: &mut World, name: &str, kind: BodyKind, position: Vec2) -> Entity {
        let node = world.spawn_node(name, None).unwrap();
        world
            .insert(node, Transform2D::from_position(position))
            .unwrap();
        let shape = Shape2d::rectangle(Vec2::new(1.0, 1.0)).unwrap();
        world
            .insert(node, Body2D::new(BodySettings::new(kind, shape)))
            .unwrap();
        node
    }

    fn frames(world: &mut World, count: usize) -> Vec<NodeContact> {
        let mut propagation = TransformPropagation::new();
        let mut contacts = Vec::new();
        for _ in 0..count {
            propagation.run(world).unwrap();
            step_node_physics(world, FRAME).unwrap();
            contacts.extend_from_slice(world.resource::<PhysicsNodes>().unwrap().contacts());
        }
        contacts
    }

    fn position(world: &World, node: Entity) -> Vec2 {
        world.get::<Transform2D>(node).unwrap().position
    }

    #[test]
    fn nodes_follow_their_falling_bodies() {
        let mut world = world();
        let crate_node = body_node(&mut world, "Crate", BodyKind::Dynamic, Vec2::new(0.0, 10.0));

        frames(&mut world, 30);

        assert!(position(&world, crate_node).y < 10.0);
        let body = world.get::<Body2D>(crate_node).unwrap().id().unwrap();
        let physics = world.resource::<Physics2d>().unwrap();
        assert_eq!(physics.position(body), Some(position(&world, crate_node)));
    }

    #[test]
    fn contacts_name_the_nodes() {
        let mut world = world();
        let floor = body_node(&mut world, "Floor", BodyKind::Static, Vec2::ZERO);
        let crate_node = body_node(&mut world, "Crate", BodyKind::Dynamic, Vec2::new(0.0, 2.0));

        let contacts = frames(&mut world, 60);

        assert!(contacts.iter().any(|contact| {
            contact.phase == ContactPhase::Started && contact.other(floor) == Some(crate_node)
        }));
        // The crate rests on the floor: 0.5 + 0.5 above its center.
        assert!((position(&world, crate_node).y - 1.0).abs() < 0.05);
    }

    #[test]
    fn moving_a_node_moves_its_body() {
        let mut world = world();
        let platform = body_node(&mut world, "Platform", BodyKind::Static, Vec2::ZERO);
        frames(&mut world, 1);

        world.get_mut::<Transform2D>(platform).unwrap().position = Vec2::new(4.0, 2.0);
        frames(&mut world, 1);

        let body = world.get::<Body2D>(platform).unwrap().id().unwrap();
        let physics = world.resource::<Physics2d>().unwrap();
        assert_eq!(physics.position(body), Some(Vec2::new(4.0, 2.0)));
    }

    #[test]
    fn children_are_placed_in_their_parents_space() {
        let mut world = world();
        let level = world.spawn_node("Level", None).unwrap();
        world
            .insert(level, Transform2D::from_position(Vec2::new(10.0, 0.0)))
            .unwrap();
        let ball = body_node(&mut world, "Ball", BodyKind::Kinematic, Vec2::new(1.0, 0.0));
        world.set_parent(ball, Some(level)).unwrap();
        frames(&mut world, 1);
        let body = world.get::<Body2D>(ball).unwrap().id().unwrap();

        world
            .resource_mut::<Physics2d>()
            .unwrap()
            .set_velocity(body, Vec2::new(0.0, 6.0));
        frames(&mut world, 10);

        let physics = world.resource::<Physics2d>().unwrap();
        assert!((physics.position(body).unwrap().x - 11.0).abs() < TOLERANCE);
        let local = position(&world, ball);
        assert!((local.x - 1.0).abs() < TOLERANCE, "{local:?}");
        assert!(local.y > 0.5, "{local:?}");
    }

    #[test]
    fn removed_nodes_remove_their_bodies_and_end_their_contacts() {
        let mut world = world();
        let floor = body_node(&mut world, "Floor", BodyKind::Static, Vec2::ZERO);
        let crate_node = body_node(&mut world, "Crate", BodyKind::Dynamic, Vec2::new(0.0, 1.0));
        frames(&mut world, 30);

        world.despawn_node(crate_node);
        let contacts = frames(&mut world, 2);

        assert_eq!(world.resource::<Physics2d>().unwrap().body_count(), 1);
        assert!(contacts.iter().any(|contact| {
            contact.phase == ContactPhase::Ended && contact.other(floor) == Some(crate_node)
        }));
    }

    #[test]
    fn a_replaced_component_replaces_the_body() {
        let mut world = world();
        let node = body_node(&mut world, "Crate", BodyKind::Dynamic, Vec2::ZERO);
        frames(&mut world, 1);
        let first = world.get::<Body2D>(node).unwrap().id().unwrap();

        let shape = Shape2d::circle(1.0).unwrap();
        world
            .insert(
                node,
                Body2D::new(BodySettings::new(BodyKind::Static, shape)),
            )
            .unwrap();
        frames(&mut world, 1);

        let second = world.get::<Body2D>(node).unwrap().id().unwrap();
        let physics = world.resource::<Physics2d>().unwrap();
        assert!(!physics.contains(first));
        assert!(physics.contains(second));
        assert_eq!(physics.body_count(), 1);
        let nodes = world.resource::<PhysicsNodes>().unwrap();
        assert_eq!(nodes.node(second), Some(node));
    }

    #[test]
    fn a_cloned_component_is_a_new_body() {
        let mut world = world();
        let first = body_node(&mut world, "First", BodyKind::Static, Vec2::ZERO);
        frames(&mut world, 1);

        let copy = world.get::<Body2D>(first).unwrap().clone();
        let second = world.spawn_node("Second", None).unwrap();
        world
            .insert(second, Transform2D::from_position(Vec2::new(5.0, 0.0)))
            .unwrap();
        world.insert(second, copy).unwrap();
        frames(&mut world, 1);

        let ids = [first, second].map(|node| world.get::<Body2D>(node).unwrap().id().unwrap());
        assert_ne!(ids[0], ids[1]);
        assert_eq!(world.resource::<Physics2d>().unwrap().body_count(), 2);
    }

    #[test]
    fn velocities_and_impulses_reach_bodies_not_added_yet() {
        let mut world = world();
        world
            .resource_mut::<Physics2d>()
            .unwrap()
            .set_gravity(Vec2::ZERO);
        let paddle = body_node(&mut world, "Paddle", BodyKind::Kinematic, Vec2::ZERO);
        let ball = body_node(&mut world, "Ball", BodyKind::Dynamic, Vec2::new(0.0, 5.0));
        world
            .get_mut::<Body2D>(paddle)
            .unwrap()
            .set_velocity(Vec2::new(2.0, 0.0));
        world
            .get_mut::<Body2D>(ball)
            .unwrap()
            .apply_impulse(Vec2::new(0.0, 3.0));
        assert_eq!(
            world.get::<Body2D>(paddle).unwrap().velocity(),
            Vec2::new(2.0, 0.0)
        );

        frames(&mut world, 30);

        let paddle_body = world.get::<Body2D>(paddle).unwrap();
        assert!((paddle_body.velocity().x - 2.0).abs() < TOLERANCE);
        assert!((position(&world, paddle).x - 1.0).abs() < TOLERANCE);
        let ball_body = world.get::<Body2D>(ball).unwrap();
        assert!(ball_body.velocity().y > 0.0);
        assert!(position(&world, ball).y > 5.0);
    }

    #[test]
    fn invalid_bodies_name_their_node() {
        let mut world = world();
        let node = body_node(&mut world, "Broken", BodyKind::Dynamic, Vec2::ZERO);
        let mut settings = *world.get::<Body2D>(node).unwrap().settings();

        settings.density = -1.0;
        world.insert(node, Body2D::new(settings)).unwrap();

        let error = step_node_physics(&world, FRAME).unwrap_err();

        assert!(error.to_string().contains("\"Broken\""), "{error}");
    }

    #[test]
    fn worlds_without_physics_do_nothing() {
        let mut world = World::new();
        let node = body_node(&mut world, "Crate", BodyKind::Dynamic, Vec2::ZERO);

        step_node_physics(&world, FRAME).unwrap();

        assert_eq!(world.get::<Body2D>(node).unwrap().id(), None);
    }
}
