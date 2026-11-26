//! The engine's world in step with a StarScript game's tree: each node of
//! the game is mirrored by a node of the world, and the bodies the physics
//! moved are copied back.

use std::collections::HashMap;

use star_engine::script_nodes::{
    ScriptBody, ScriptCamera, ScriptCamera3D, ScriptLight, ScriptMesh, ScriptNode, ScriptNodes,
    ScriptSprite,
};
use star_engine::star_ecs::World;
use star_engine::star_math::{Quat, Vec2, Vec3};
use star_engine::star_render::{Color, SpriteSheet};
use star_engine::star_scene::{Transform2D, Transform3D};
use star_script_runtime::bodies::Contact;
use star_script_runtime::nodes::{Host, NodeFields};

/// Describes every node of the tree of `root` to the engine, parents first,
/// so the world has a node in step with each one. `pending` is a reused
/// list, left empty.
pub(crate) fn mirror_nodes<H: Host>(
    world: &mut World,
    mirror: &mut ScriptNodes,
    root: &H::Node,
    pending: &mut Vec<(H::Node, Option<u64>)>,
) {
    mirror.begin_frame();
    pending.push((root.clone(), None));
    while let Some((node, parent)) = pending.pop() {
        let fields = H::node(&node);
        describe::<H>(&node, parent, |described| mirror.mirror(world, described));
        for child in fields.children().iter().rev() {
            pending.push((child.clone(), Some(fields.id())));
        }
    }
    mirror.finish_frame(world);
}

/// Runs `then` with what the engine needs to know about `node`, whose
/// parent is `parent`. The impulses given to its body are forgotten.
fn describe<H: Host>(node: &H::Node, parent: Option<u64>, then: impl FnOnce(&ScriptNode<'_>)) {
    let fields = H::node(node);
    let transform = Transform2D {
        position: Vec2::new(fields.x.get(), fields.y.get()),
        rotation: fields.rotation.get(),
        scale: Vec2::new(fields.scale_x.get(), fields.scale_y.get()),
    };
    let transform_3d = H::as_node_3d(node).map(|node_3d| {
        let depth = H::node_3d(&node_3d);
        let angles = Vec3::new(depth.pitch.get(), depth.yaw.get(), fields.rotation.get());
        Transform3D {
            position: Vec3::new(fields.x.get(), fields.y.get(), depth.z.get()),
            rotation: Quat::from_euler(angles),
            scale: Vec3::new(
                fields.scale_x.get(),
                fields.scale_y.get(),
                depth.scale_z.get(),
            ),
        }
    });
    fields.name.read(|name| {
        with_sprite::<H>(node, |sprite| {
            with_body::<H>(fields, |body| {
                with_mesh::<H>(node, |mesh| {
                    with_light::<H>(node, false, |light| {
                        with_light::<H>(node, true, |light_2d| {
                            then(&ScriptNode {
                                id: fields.id(),
                                parent,
                                name,
                                transform,
                                transform_3d,
                                sprite,
                                body,
                                mesh,
                                light,
                                light_2d,
                            });
                        });
                    });
                });
            });
        });
    });
}

/// Runs `then` with the image `node` shows, if it is a `Sprite`.
fn with_sprite<H: Host>(node: &H::Node, then: impl FnOnce(Option<ScriptSprite<'_>>)) {
    let mut then = Some(then);
    H::with_sprite(node, |look| {
        look.texture.read(|texture| {
            if let Some(then) = then.take() {
                then(Some(ScriptSprite {
                    texture,
                    size: Vec2::new(look.width.get(), look.height.get()),
                    layer: look.layer.get(),
                    pixelated: look.pixelated.get(),
                    color: look.color.read(engine_color::<H>),
                    sheet: sheet(look.columns.get(), look.rows.get(), look.frame.get()),
                }));
            }
        });
    });
    if let Some(then) = then {
        then(None);
    }
}

/// The sprite sheet of a `Sprite`'s `columns`, `rows` and `frame`, whatever
/// the script set them to: fewer than one column or row is one, and frames
/// before the first or past the last go round the sheet, so `frame += 1`
/// and `frame -= 1` loop over its cells.
fn sheet(columns: i32, rows: i32, frame: i32) -> SpriteSheet {
    let cells = |count: i32| u16::try_from(count.clamp(1, i32::from(u16::MAX))).unwrap_or(1);
    let (columns, rows) = (cells(columns), cells(rows));
    let frames = i64::from(columns) * i64::from(rows);
    let frame = u32::try_from(i64::from(frame).rem_euclid(frames)).unwrap_or(0);
    SpriteSheet::new(columns, rows, frame)
}

/// Runs `then` with the body of the node of `fields`, if it has one, and
/// forgets the impulses given to it.
fn with_body<H: Host>(fields: &NodeFields<H>, then: impl FnOnce(Option<ScriptBody<'_>>)) {
    let Some(body) = fields.body.get() else {
        return then(None);
    };
    let body = H::body(&body);
    let [impulse_x, impulse_y, impulse_z] = body.take_impulse();
    let [velocity_x, velocity_y, velocity_z] = body.velocity();
    body.kind.read(|kind| {
        body.shape.read(|shape| {
            then(Some(ScriptBody {
                kind,
                shape,
                radius: body.radius.get(),
                width: body.width.get(),
                height: body.height.get(),
                depth: body.depth.get(),
                gravity_scale: body.gravity_scale.get(),
                fixed_rotation: body.fixed_rotation.get(),
                friction: body.friction.get(),
                bounciness: body.bounciness.get(),
                density: body.density.get(),
                sensor: body.sensor.get(),
                velocity: Vec3::new(velocity_x, velocity_y, velocity_z),
                impulse: Vec3::new(impulse_x, impulse_y, impulse_z),
            }));
        });
    });
}

/// Runs `then` with the mesh `node` shows, if it is a `Mesh`.
fn with_mesh<H: Host>(node: &H::Node, then: impl FnOnce(Option<ScriptMesh<'_>>)) {
    let mut then = Some(then);
    H::with_mesh(node, |look| {
        look.shape.read(|shape| {
            look.texture.read(|texture| {
                if let Some(then) = then.take() {
                    then(Some(ScriptMesh {
                        shape,
                        width: look.width.get(),
                        height: look.height.get(),
                        depth: look.depth.get(),
                        radius: look.radius.get(),
                        color: look.color.read(engine_color::<H>),
                        metallic: look.metallic.get(),
                        roughness: look.roughness.get(),
                        texture,
                        pixelated: look.pixelated.get(),
                    }));
                }
            });
        });
    });
    if let Some(then) = then {
        then(None);
    }
}

/// Runs `then` with the light `node` shines, if it is a `Light2D` (when
/// `on_sprites`) or a `Light`.
fn with_light<H: Host>(
    node: &H::Node,
    on_sprites: bool,
    then: impl FnOnce(Option<ScriptLight<'_>>),
) {
    let mut then = Some(then);
    let describe = |look: &star_script_runtime::node_classes::LightFields<H>| {
        look.kind.read(|kind| {
            if let Some(then) = then.take() {
                then(Some(ScriptLight {
                    kind,
                    color: look.color.read(engine_color::<H>),
                    intensity: look.intensity.get(),
                    range: look.range.get(),
                    angle: look.angle.get(),
                    blend: look.blend.get(),
                }));
            }
        });
    };
    if on_sprites {
        H::with_light_2d(node, describe);
    } else {
        H::with_light(node, describe);
    }
    if let Some(then) = then {
        then(None);
    }
}

/// Tells the nodes with a body of the tree of `root` what the simulation
/// did in the last frame: where they are, how fast they move and what they
/// touch. `known` and `queue` are reused, left empty.
pub(crate) fn sync_bodies<H: Host>(
    world: &World,
    mirror: &mut ScriptNodes,
    root: &H::Node,
    known: &mut HashMap<u64, H::Node>,
    queue: &mut Vec<H::Node>,
) {
    mirror.read_bodies(world);
    let mut next = Some(root.clone());
    while let Some(node) = next {
        let fields = H::node(&node);
        queue.extend(fields.children().iter().cloned());
        known.insert(fields.id(), node.clone());
        next = queue.pop();
    }
    for (id, node) in known.iter() {
        let fields = H::node(node);
        let Some(body) = fields.body.get() else {
            continue;
        };
        let Some(state) = mirror.body_state(world, *id) else {
            continue;
        };
        fields.x.set(state.position.x);
        fields.y.set(state.position.y);
        fields.rotation.set(state.angles.z);
        if let (true, Some(node_3d)) = (state.is_3d, H::as_node_3d(node)) {
            let depth = H::node_3d(&node_3d);
            depth.z.set(state.position.z);
            depth.pitch.set(state.angles.x);
            depth.yaw.set(state.angles.y);
        }
        let body = H::body(&body);
        body.set_velocity([state.velocity.x, state.velocity.y, state.velocity.z]);
        let contacts = [
            (Contact::Touching, state.touching),
            (Contact::JustTouched, state.just_touched),
            (Contact::JustSeparated, state.just_separated),
        ];
        for (which, ids) in contacts {
            body.set_contacts(which, ids.iter().filter_map(|other| known.get(other)));
        }
    }
    known.clear();
}

/// The engine's value of a script color.
pub(crate) fn engine_color<H: Host>(color: &H::Color) -> Color {
    let [r, g, b, a] = H::channels(color);
    Color::rgba(r, g, b, a)
}

/// What the engine needs to know about a `Camera2D`.
pub(crate) fn script_camera<H: Host>(camera: &H::Camera2D) -> ScriptCamera {
    let node = H::camera_2d_node(camera);
    let fields = H::node(&node);
    ScriptCamera {
        id: fields.id(),
        position: Vec2::new(fields.x.get(), fields.y.get()),
        view_height: H::camera_2d(camera).view_height.get(),
    }
}

/// What the engine needs to know about a `Camera3D`; one that is not
/// placed in 3D is at depth 0, not turned.
pub(crate) fn script_camera_3d<H: Host>(camera: &H::Camera3D) -> ScriptCamera3D {
    let node = H::camera_3d_node(camera);
    let fields = H::node(&node);
    let (depth, pitch, yaw) = H::as_node_3d(&node).map_or((0.0, 0.0, 0.0), |node_3d| {
        let depth = H::node_3d(&node_3d);
        (depth.z.get(), depth.pitch.get(), depth.yaw.get())
    });
    ScriptCamera3D {
        id: fields.id(),
        position: Vec3::new(fields.x.get(), fields.y.get(), depth),
        angles: Vec3::new(pitch, yaw, fields.rotation.get()),
        field_of_view: H::camera_3d(camera).fov.get(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_sheets_always_show_a_cell() {
        assert_eq!(sheet(4, 2, 5), SpriteSheet::new(4, 2, 5));
        assert_eq!(sheet(0, -3, 9), SpriteSheet::SINGLE);
        assert_eq!(sheet(100_000, 1, 1), SpriteSheet::new(u16::MAX, 1, 1));
    }

    #[test]
    fn script_frames_go_round_the_sheet_both_ways() {
        assert_eq!(sheet(4, 1, 4).frame(), 0);
        assert_eq!(sheet(4, 1, -1).frame(), 3);
        assert_eq!(sheet(2, 2, i32::MIN).frame(), 0);
    }
}
