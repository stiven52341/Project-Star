//! The 3D physics of the nodes: bodies that follow their 3D nodes, and
//! nodes that follow their bodies.

use star_core::EngineResult;
use star_ecs::{Entity, World};
use star_math::{Mat4, Quat};
use star_physics::{BodyKind, BodySettings, Physics3d};

use crate::node_physics::{named_error, rescaled};
use crate::{Body3D, GlobalTransform3D, Parent, PhysicsNodes3d, Transform3D};

/// Runs the 3D physics of the nodes for a frame that took `elapsed`
/// seconds, with the [`Physics3d`] and [`PhysicsNodes3d`] resources of
/// `world` (without them, nothing happens). The same as
/// [`step_node_physics`](crate::step_node_physics) does in 2D, for the
/// nodes with a [`Body3D`] and a [`Transform3D`]:
///
/// 1. removes the bodies of nodes that are gone or lost their [`Body3D`];
/// 2. adds the bodies of new [`Body3D`]s where their nodes are, and moves
///    to their nodes the bodies whose node the game moved;
/// 3. gives the bodies the velocities and impulses set on their
///    [`Body3D`]s;
/// 4. advances the simulation;
/// 5. writes where the dynamic and kinematic bodies ended up into their
///    nodes' [`Transform3D`], and their velocities into their [`Body3D`];
/// 6. turns the contacts of the bodies into contacts of their nodes.
///
/// It places bodies with the [`GlobalTransform3D`]s of their parents, so
/// those must be up to date. Under a parent scaled differently along each
/// axis and turned, a node's rotation is only approximate.
///
/// # Errors
///
/// [`EngineError::Physics`](star_core::EngineError::Physics), naming the
/// node, if a body cannot be added;
/// [`EngineError::Ecs`](star_core::EngineError::Ecs) if a running query
/// holds the components it uses.
pub fn step_node_physics_3d(world: &World, elapsed: f32) -> EngineResult<()> {
    let (Some(mut physics), Some(mut nodes)) = (
        world.resource_mut::<Physics3d>(),
        world.resource_mut::<PhysicsNodes3d>(),
    ) else {
        return Ok(());
    };
    nodes.remove_departed(
        |node| world.get::<Body3D>(node).and_then(|body| body.id()),
        |body| physics.remove_body(body),
    );
    place_bodies(world, &mut physics, &mut nodes)?;
    push_bodies(world, &mut physics)?;
    physics.advance(elapsed);
    follow_bodies(world, &physics)?;
    nodes.collect_contacts(physics.contacts());
    Ok(())
}

fn place_bodies(
    world: &World,
    physics: &mut Physics3d,
    nodes: &mut PhysicsNodes3d,
) -> EngineResult<()> {
    let mut failure = None;
    world.each::<(&mut Body3D, &Transform3D)>(|node, (body, local)| {
        if failure.is_some() {
            return;
        }
        let global = parent_global(world, node) * local.to_mat4();
        let scale = global.scale();
        let rescaled = rescaled(
            body.shaped_scale().map(|old| [old.x, old.y, old.z]),
            [scale.x, scale.y, scale.z],
        );
        if !rescaled && !body.was_moved(local) {
            return;
        }
        let shape = match body.settings().shape.scaled(scale) {
            Ok(shape) => shape,
            Err(error) => {
                failure = Some(named_error(world, node, &error));
                return;
            }
        };
        let position = global.translation();
        // A node scaled to nothing keeps no rotation.
        let rotation = Quat::from_mat4(global).unwrap_or(Quat::IDENTITY);
        if let Some(id) = body.id() {
            physics.set_position(id, position);
            physics.set_rotation(id, rotation);
            if rescaled {
                physics.set_shape(id, &shape);
            }
            body.wrote(*local);
        } else {
            let settings = BodySettings {
                shape,
                ..*body.settings()
            };
            match physics.add_body(&settings, position, rotation) {
                Ok(id) => {
                    body.placed(id, *local);
                    nodes.add(id, node);
                }
                Err(error) => {
                    failure = Some(named_error(world, node, &error));
                    return;
                }
            }
        }
        body.shaped(scale);
    })?;
    failure.map_or(Ok(()), Err)
}

fn push_bodies(world: &World, physics: &mut Physics3d) -> EngineResult<()> {
    world.each::<&mut Body3D>(|_, body| {
        let Some(id) = body.id() else {
            return;
        };
        let (velocity, impulse) = body.take_motion();
        if let Some(velocity) = velocity {
            physics.set_velocity(id, velocity);
        }
        if let Some(impulse) = impulse {
            physics.apply_impulse(id, impulse);
        }
    })
}

fn follow_bodies(world: &World, physics: &Physics3d) -> EngineResult<()> {
    world.each::<(&mut Body3D, &mut Transform3D)>(|node, (body, local)| {
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
        if let (Some(to_parent), Some(parent_rotation)) =
            (parent.inverse(), Quat::from_mat4(parent))
        {
            local.position = to_parent.transform_point(position);
            local.rotation = parent_rotation.inverse() * rotation;
            body.wrote(*local);
        }
    })
}

/// Where the space of `node`'s parent is in the world: the global transform
/// of its closest ancestor that has one.
fn parent_global(world: &World, node: Entity) -> Mat4 {
    let mut current = node;
    while let Some(parent) = world.get::<Parent>(current).map(|parent| parent.entity()) {
        if let Some(global) = world.get::<GlobalTransform3D>(parent) {
            return global.matrix();
        }
        current = parent;
    }
    Mat4::IDENTITY
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use star_math::Vec3;
    use star_physics::{ContactPhase, Shape3d};

    use super::*;
    use crate::{Hierarchy, NodeContact, TransformPropagation};

    /// Seconds of a test frame: a 60 Hz screen.
    const FRAME: f32 = 1.0 / 60.0;
    const TOLERANCE: f32 = 1e-3;

    fn world() -> World {
        let mut world = World::new();
        world.insert_resource(Physics3d::default());
        world.insert_resource(PhysicsNodes3d::new());
        world
    }

    fn body_node(world: &mut World, name: &str, kind: BodyKind, position: Vec3) -> Entity {
        let node = world.spawn_node(name, None).unwrap();
        world
            .insert(node, Transform3D::from_position(position))
            .unwrap();
        let shape = Shape3d::cuboid(Vec3::ONE).unwrap();
        world
            .insert(node, Body3D::new(BodySettings::new(kind, shape)))
            .unwrap();
        node
    }

    fn frames(world: &mut World, count: usize) -> Vec<NodeContact> {
        let mut propagation = TransformPropagation::new();
        let mut contacts = Vec::new();
        for _ in 0..count {
            propagation.run(world).unwrap();
            step_node_physics_3d(world, FRAME).unwrap();
            contacts.extend_from_slice(world.resource::<PhysicsNodes3d>().unwrap().contacts());
        }
        contacts
    }

    fn position(world: &World, node: Entity) -> Vec3 {
        world.get::<Transform3D>(node).unwrap().position
    }

    #[test]
    fn nodes_follow_their_falling_bodies() {
        let mut world = world();
        let crate_node = body_node(
            &mut world,
            "Crate",
            BodyKind::Dynamic,
            Vec3::new(0.0, 10.0, 0.0),
        );

        frames(&mut world, 30);

        assert!(position(&world, crate_node).y < 10.0);
        let body = world.get::<Body3D>(crate_node).unwrap().id().unwrap();
        let physics = world.resource::<Physics3d>().unwrap();
        assert_eq!(physics.position(body), Some(position(&world, crate_node)));
    }

    #[test]
    fn contacts_name_the_nodes() {
        let mut world = world();
        let floor = body_node(&mut world, "Floor", BodyKind::Static, Vec3::ZERO);
        let crate_node = body_node(
            &mut world,
            "Crate",
            BodyKind::Dynamic,
            Vec3::new(0.0, 2.0, 0.0),
        );

        let contacts = frames(&mut world, 60);

        assert!(contacts.iter().any(|contact| {
            contact.phase == ContactPhase::Started && contact.other(floor) == Some(crate_node)
        }));
        // The crate rests on the floor: 0.5 + 0.5 above its center.
        assert!((position(&world, crate_node).y - 1.0).abs() < 0.05);
    }

    #[test]
    fn shapes_stretch_with_their_nodes() {
        let mut world = world();
        let floor = body_node(&mut world, "Floor", BodyKind::Static, Vec3::ZERO);
        world.get_mut::<Transform3D>(floor).unwrap().scale = Vec3::new(10.0, 1.0, 10.0);
        // Over the stretched floor, far from where an unscaled one ends.
        let crate_node = body_node(
            &mut world,
            "Crate",
            BodyKind::Dynamic,
            Vec3::new(3.0, 2.0, -3.0),
        );

        frames(&mut world, 60);

        assert!((position(&world, crate_node).y - 1.0).abs() < 0.05);
    }

    #[test]
    fn shapes_follow_the_scale_of_their_parents() {
        let mut world = world();
        let level = world.spawn_node("Level", None).unwrap();
        world.insert(level, Transform3D::IDENTITY).unwrap();
        let floor = body_node(&mut world, "Floor", BodyKind::Static, Vec3::ZERO);
        world.set_parent(floor, Some(level)).unwrap();
        let crate_node = body_node(
            &mut world,
            "Crate",
            BodyKind::Dynamic,
            Vec3::new(0.0, 3.0, 0.0),
        );
        frames(&mut world, 60);
        let resting = position(&world, crate_node).y;

        // Wider than the crate, so it is pushed out upwards.
        world.get_mut::<Transform3D>(level).unwrap().scale = Vec3::new(4.0, 3.0, 4.0);
        frames(&mut world, 60);

        assert!((resting - 1.0).abs() < 0.05, "{resting}");
        let risen = position(&world, crate_node).y;
        assert!((risen - 2.0).abs() < 0.05, "{risen}");
    }

    #[test]
    fn moving_a_node_moves_its_body() {
        let mut world = world();
        let platform = body_node(&mut world, "Platform", BodyKind::Static, Vec3::ZERO);
        frames(&mut world, 1);

        world.get_mut::<Transform3D>(platform).unwrap().position = Vec3::new(4.0, 2.0, -1.0);
        frames(&mut world, 1);

        let body = world.get::<Body3D>(platform).unwrap().id().unwrap();
        let physics = world.resource::<Physics3d>().unwrap();
        assert_eq!(physics.position(body), Some(Vec3::new(4.0, 2.0, -1.0)));
    }

    #[test]
    fn children_are_placed_in_their_turned_parents_space() {
        let mut world = world();
        let level = world.spawn_node("Level", None).unwrap();
        // Turned a quarter around y: its x axis points to -z in the world.
        let turned = Quat::from_rotation_y(FRAC_PI_2);
        world
            .insert(
                level,
                Transform3D::from_position(Vec3::new(10.0, 0.0, 0.0)).with_rotation(turned),
            )
            .unwrap();
        let ball = body_node(
            &mut world,
            "Ball",
            BodyKind::Kinematic,
            Vec3::new(1.0, 0.0, 0.0),
        );
        world.set_parent(ball, Some(level)).unwrap();
        frames(&mut world, 1);
        let body = world.get::<Body3D>(ball).unwrap().id().unwrap();
        let physics = world.resource::<Physics3d>().unwrap();
        let placed = physics.position(body).unwrap();
        assert!(
            (placed - Vec3::new(10.0, 0.0, -1.0)).length() < TOLERANCE,
            "{placed:?}"
        );
        drop(physics);

        world
            .get_mut::<Body3D>(ball)
            .unwrap()
            .set_velocity(Vec3::new(0.0, 6.0, 0.0));
        frames(&mut world, 10);

        let local = world.get::<Transform3D>(ball).unwrap();
        assert!((local.position.x - 1.0).abs() < TOLERANCE, "{local:?}");
        assert!(local.position.y > 0.5, "{local:?}");
        // The body keeps the world's rotation of the parent, which is no
        // rotation inside it.
        let point = Vec3::new(1.0, 0.0, 0.0);
        assert!((local.rotation.rotate(point) - point).length() < TOLERANCE);
    }

    #[test]
    fn removed_nodes_remove_their_bodies_and_end_their_contacts() {
        let mut world = world();
        let floor = body_node(&mut world, "Floor", BodyKind::Static, Vec3::ZERO);
        let crate_node = body_node(
            &mut world,
            "Crate",
            BodyKind::Dynamic,
            Vec3::new(0.0, 1.0, 0.0),
        );
        frames(&mut world, 30);

        world.despawn_node(crate_node);
        let contacts = frames(&mut world, 2);

        assert_eq!(world.resource::<Physics3d>().unwrap().body_count(), 1);
        assert!(contacts.iter().any(|contact| {
            contact.phase == ContactPhase::Ended && contact.other(floor) == Some(crate_node)
        }));
    }

    #[test]
    fn velocities_and_impulses_reach_bodies_not_added_yet() {
        let mut world = world();
        world
            .resource_mut::<Physics3d>()
            .unwrap()
            .set_gravity(Vec3::ZERO);
        let paddle = body_node(&mut world, "Paddle", BodyKind::Kinematic, Vec3::ZERO);
        let ball = body_node(
            &mut world,
            "Ball",
            BodyKind::Dynamic,
            Vec3::new(0.0, 5.0, 0.0),
        );
        world
            .get_mut::<Body3D>(paddle)
            .unwrap()
            .set_velocity(Vec3::new(0.0, 0.0, 2.0));
        world
            .get_mut::<Body3D>(ball)
            .unwrap()
            .apply_impulse(Vec3::new(0.0, 3.0, 0.0));

        frames(&mut world, 30);

        let paddle_body = world.get::<Body3D>(paddle).unwrap();
        assert!((paddle_body.velocity().z - 2.0).abs() < TOLERANCE);
        assert!((position(&world, paddle).z - 1.0).abs() < TOLERANCE);
        assert!(world.get::<Body3D>(ball).unwrap().velocity().y > 0.0);
        assert!(position(&world, ball).y > 5.0);
    }

    #[test]
    fn invalid_bodies_name_their_node() {
        let mut world = world();
        let node = body_node(&mut world, "Broken", BodyKind::Dynamic, Vec3::ZERO);
        let mut settings = *world.get::<Body3D>(node).unwrap().settings();

        settings.friction = -1.0;
        world.insert(node, Body3D::new(settings)).unwrap();

        let error = step_node_physics_3d(&world, FRAME).unwrap_err();

        assert!(error.to_string().contains("\"Broken\""), "{error}");
    }

    #[test]
    fn worlds_without_physics_do_nothing() {
        let mut world = World::new();
        let node = body_node(&mut world, "Crate", BodyKind::Dynamic, Vec3::ZERO);

        step_node_physics_3d(&world, FRAME).unwrap();

        assert_eq!(world.get::<Body3D>(node).unwrap().id(), None);
    }
}
