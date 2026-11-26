//! 3D nodes the engine draws: meshes and lights.

use star_core::EngineResult;
use star_ecs::World;
use star_math::Vec3;
use star_render::{
    Color, DirectionalLight, Frame, Light, MAX_LIGHTS, Material, Mesh3dId, PointLight, SpotLight,
};

use crate::{GlobalTransform3D, is_shown};

/// Shows a 3D mesh at a 3D node: the engine draws it every frame where the
/// node's [`GlobalTransform3D`] places it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeshNode {
    /// The mesh shown, uploaded with `GameContext::create_mesh_3d`.
    pub mesh: Mesh3dId,
    /// How its surface looks.
    pub material: Material,
}

impl MeshNode {
    /// Shows `mesh` with `material`.
    #[must_use]
    pub const fn new(mesh: Mesh3dId, material: Material) -> Self {
        Self { mesh, material }
    }
}

/// Lights the 3D meshes from a 3D node.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LightNode {
    /// Light from far away, shining where the node looks: along its -z
    /// axis, which is forward for cameras and lights.
    Directional {
        /// Its color.
        color: Color,
        /// How bright it is.
        intensity: f32,
    },
    /// Light from the node's position, in every direction.
    Point {
        /// Its color.
        color: Color,
        /// How bright it is one world unit away.
        intensity: f32,
        /// Beyond this distance it lights nothing.
        range: f32,
    },
    /// Light from the node's position, in a cone along its -z axis.
    Spot {
        /// Its color.
        color: Color,
        /// How bright it is one world unit away.
        intensity: f32,
        /// Beyond this distance it lights nothing.
        range: f32,
        /// From the cone's axis to its edge, in radians.
        angle: f32,
        /// How much of the cone, from its edge inwards, fades: 0 to 1.
        blend: f32,
    },
}

impl LightNode {
    /// The light shining from a node placed at `global`.
    #[must_use]
    pub fn to_light(self, global: GlobalTransform3D) -> Light {
        let [_, _, backward, _] = global.matrix().to_columns();
        let forward = Vec3::new(-backward[0], -backward[1], -backward[2]);
        match self {
            Self::Directional { color, intensity } => DirectionalLight {
                direction: forward,
                color,
                intensity,
            }
            .into(),
            Self::Point {
                color,
                intensity,
                range,
            } => PointLight {
                position: global.position(),
                color,
                intensity,
                range,
            }
            .into(),
            Self::Spot {
                color,
                intensity,
                range,
                angle,
                blend,
            } => SpotLight {
                position: global.position(),
                direction: forward,
                color,
                intensity,
                range,
                angle,
                blend,
            }
            .into(),
        }
    }
}

/// Adds to `frame` the mesh of every node with a [`MeshNode`] and a
/// [`GlobalTransform3D`], and the light of every node with a
/// [`LightNode`] and a [`GlobalTransform3D`], unless they are hidden. A frame takes at most
/// [`MAX_LIGHTS`]: light nodes beyond them are left out.
///
/// # Errors
///
/// Returns [`EngineError::Ecs`](star_core::EngineError::Ecs) if a running
/// query holds the components it reads.
pub fn draw_3d_nodes(world: &World, frame: &mut Frame) -> EngineResult<()> {
    world.each::<(&MeshNode, &GlobalTransform3D)>(|entity, (node, global)| {
        if is_shown(world, entity) {
            frame.draw_mesh_3d(node.mesh, node.material, global.matrix());
        }
    })?;
    world.each::<(&LightNode, &GlobalTransform3D)>(|entity, (node, global)| {
        if is_shown(world, entity) && frame.lights().len() < MAX_LIGHTS {
            frame.add_light(node.to_light(*global));
        }
    })
}

#[cfg(test)]
mod tests {
    use star_math::Quat;

    use super::*;
    use crate::{Hierarchy, Transform3D, TransformPropagation};

    const EPSILON: f32 = 1e-5;

    fn node_3d(world: &mut World, transform: Transform3D) -> star_ecs::Entity {
        let node = world.spawn_node("Node", None).unwrap();
        world.insert(node, transform).unwrap();
        node
    }

    #[test]
    fn mesh_nodes_are_drawn_where_their_nodes_are() {
        let mut world = World::new();
        let placed = node_3d(&mut world, Transform3D::from_position(Vec3::UP));
        world
            .insert(
                placed,
                MeshNode::new(Mesh3dId::from_index(0), Material::default()),
            )
            .unwrap();
        let unplaced = world.spawn_node("Unplaced", None).unwrap();
        world
            .insert(
                unplaced,
                MeshNode::new(Mesh3dId::from_index(0), Material::default()),
            )
            .unwrap();
        TransformPropagation::new().run(&mut world).unwrap();
        let mut frame = Frame::new();

        draw_3d_nodes(&world, &mut frame).unwrap();

        assert_eq!(frame.mesh_3d_count(), 1);
    }

    #[test]
    fn lights_shine_from_their_nodes() {
        let mut world = World::new();
        let sun = node_3d(
            &mut world,
            Transform3D::IDENTITY
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        );
        world
            .insert(
                sun,
                LightNode::Directional {
                    color: Color::WHITE,
                    intensity: 1.0,
                },
            )
            .unwrap();
        let lamp = node_3d(
            &mut world,
            Transform3D::from_position(Vec3::new(1.0, 2.0, 3.0)),
        );
        let bulb = LightNode::Point {
            color: Color::WHITE,
            intensity: 1.0,
            range: 5.0,
        };
        world.insert(lamp, bulb).unwrap();
        TransformPropagation::new().run(&mut world).unwrap();
        let mut frame = Frame::new();

        draw_3d_nodes(&world, &mut frame).unwrap();

        let mut found = (false, false);
        for light in frame.lights() {
            match light {
                Light::Directional(sun) => {
                    // Looking down after pitching -90°.
                    assert!((sun.direction - Vec3::new(0.0, -1.0, 0.0)).length() < EPSILON);
                    found.0 = true;
                }
                Light::Point(lamp) => {
                    assert_eq!(lamp.position, Vec3::new(1.0, 2.0, 3.0));
                    found.1 = true;
                }
                Light::Spot(_) => panic!("there are no spots"),
            }
        }
        assert_eq!(found, (true, true));
    }

    #[test]
    fn spots_shine_from_their_nodes_along_their_minus_z() {
        let mut world = World::new();
        let node = node_3d(
            &mut world,
            Transform3D::from_position(Vec3::new(0.0, 4.0, 0.0))
                .with_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
        );
        let lamp = LightNode::Spot {
            color: Color::WHITE,
            intensity: 1.0,
            range: 5.0,
            angle: 0.5,
            blend: 0.2,
        };
        world.insert(node, lamp).unwrap();
        TransformPropagation::new().run(&mut world).unwrap();
        let mut frame = Frame::new();

        draw_3d_nodes(&world, &mut frame).unwrap();

        let [Light::Spot(spot)] = frame.lights() else {
            panic!("one spot shines");
        };
        assert!((spot.position - Vec3::new(0.0, 4.0, 0.0)).length() < EPSILON);
        assert!((spot.direction - Vec3::new(0.0, -1.0, 0.0)).length() < EPSILON);
        assert!((spot.angle - 0.5).abs() < EPSILON);
    }

    #[test]
    fn light_nodes_beyond_the_limit_are_left_out() {
        let mut world = World::new();
        for _ in 0..=MAX_LIGHTS {
            let node = node_3d(&mut world, Transform3D::IDENTITY);
            world
                .insert(
                    node,
                    LightNode::Point {
                        color: Color::WHITE,
                        intensity: 1.0,
                        range: 1.0,
                    },
                )
                .unwrap();
        }
        TransformPropagation::new().run(&mut world).unwrap();
        let mut frame = Frame::new();

        draw_3d_nodes(&world, &mut frame).unwrap();

        assert_eq!(frame.lights().len(), MAX_LIGHTS);
    }
}
