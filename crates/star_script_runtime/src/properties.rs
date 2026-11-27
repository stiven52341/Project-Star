//! A node's properties read and changed by name, as an animation's tracks
//! and tweens change them: the fields of the engine's node classes, and
//! else the `export` fields of the node's script class.

use crate::Field;
use crate::node_classes::{LightFields, MeshFields, SpriteFields};
use crate::nodes::{Host, Node3dFields, NodeFields};
use crate::scene_fields::{self, SceneValue};

/// Gives the property `name` of `node` the value `value`.
///
/// # Errors
///
/// Why it cannot: the node has no such property, or the value does not fit
/// it.
pub fn set_property<H: Host>(node: &H::Node, name: &str, value: &SceneValue) -> Result<(), String> {
    let engine = set_node(H::node(node), name, value)
        .or_else(|| {
            H::as_node_3d(node).and_then(|node_3d| set_node_3d(H::node_3d(&node_3d), name, value))
        })
        .or_else(|| H::with_sprite(node, |sprite| set_sprite(sprite, name, value)).flatten())
        .or_else(|| H::with_mesh(node, |mesh| set_mesh(mesh, name, value)).flatten())
        .or_else(|| H::with_light(node, |light| set_light(light, name, value)).flatten())
        .or_else(|| H::with_light_2d(node, |light| set_light(light, name, value)).flatten())
        .or_else(|| {
            H::with_model(node, |model| match name {
                "file" => Some(set_text(&model.file, name, value)),
                _ => None,
            })
            .flatten()
        })
        .or_else(|| {
            H::with_sound_player(node, |player| match name {
                "sound" => Some(set_text(&player.sound, name, value)),
                "volume" => Some(set_float(&player.volume, name, value)),
                "looping" => Some(set_bool(&player.looping, name, value)),
                "play_on_start" => Some(set_bool(&player.play_on_start, name, value)),
                "group" => Some(set_text(&player.group, name, value)),
                "spatial" => Some(set_bool(&player.spatial, name, value)),
                "near" => Some(set_float(&player.near, name, value)),
                "far" => Some(set_float(&player.far, name, value)),
                _ => None,
            })
            .flatten()
        })
        .or_else(|| {
            H::with_bone(node, |bone| match name {
                "length" => Some(set_float(&bone.length, name, value)),
                _ => None,
            })
            .flatten()
        })
        .or_else(|| {
            H::as_camera_2d(node).and_then(|camera| match name {
                "view_height" => Some(set_float(&H::camera_2d(&camera).view_height, name, value)),
                _ => None,
            })
        })
        .or_else(|| {
            H::as_camera_3d(node).and_then(|camera| match name {
                "fov" => Some(set_float(&H::camera_3d(&camera).fov, name, value)),
                _ => None,
            })
        });
    engine.unwrap_or_else(|| H::set_exported_field(node, name, value))
}

/// The value of the property `name` of `node`.
///
/// # Errors
///
/// Why it cannot be read: the node has no such property.
pub fn property<H: Host>(node: &H::Node, name: &str) -> Result<SceneValue, String> {
    let engine = node_value(H::node(node), name)
        .or_else(|| {
            H::as_node_3d(node).and_then(|node_3d| node_3d_value(H::node_3d(&node_3d), name))
        })
        .or_else(|| H::with_sprite(node, |sprite| sprite_value::<H>(sprite, name)).flatten())
        .or_else(|| H::with_mesh(node, |mesh| mesh_value::<H>(mesh, name)).flatten())
        .or_else(|| H::with_light(node, |light| light_value::<H>(light, name)).flatten())
        .or_else(|| H::with_light_2d(node, |light| light_value::<H>(light, name)).flatten())
        .or_else(|| {
            H::with_model(node, |model| match name {
                "file" => Some(SceneValue::Text(model.file.get())),
                _ => None,
            })
            .flatten()
        })
        .or_else(|| {
            H::with_sound_player(node, |player| match name {
                "sound" => Some(SceneValue::Text(player.sound.get())),
                "volume" => Some(float(&player.volume)),
                "looping" => Some(SceneValue::Bool(player.looping.get())),
                "play_on_start" => Some(SceneValue::Bool(player.play_on_start.get())),
                "group" => Some(SceneValue::Text(player.group.get())),
                "spatial" => Some(SceneValue::Bool(player.spatial.get())),
                "near" => Some(float(&player.near)),
                "far" => Some(float(&player.far)),
                _ => None,
            })
            .flatten()
        })
        .or_else(|| {
            H::with_bone(node, |bone| match name {
                "length" => Some(float(&bone.length)),
                _ => None,
            })
            .flatten()
        })
        .or_else(|| {
            H::as_camera_2d(node).and_then(|camera| match name {
                "view_height" => Some(float(&H::camera_2d(&camera).view_height)),
                _ => None,
            })
        })
        .or_else(|| {
            H::as_camera_3d(node).and_then(|camera| match name {
                "fov" => Some(float(&H::camera_3d(&camera).fov)),
                _ => None,
            })
        });
    engine.map_or_else(|| H::exported_field(node, name), Ok)
}

fn float(field: &Field<f32>) -> SceneValue {
    SceneValue::Float(f64::from(field.get()))
}

fn whole(field: &Field<i32>) -> SceneValue {
    SceneValue::Int(i64::from(field.get()))
}

fn color<H: Host>(field: &Field<H::Color>) -> SceneValue {
    SceneValue::Color(field.read(|color| H::channels(color)))
}

fn node_value<H: Host>(node: &NodeFields<H>, name: &str) -> Option<SceneValue> {
    let field = match name {
        "x" => &node.x,
        "y" => &node.y,
        "rotation" => &node.rotation,
        "scale_x" => &node.scale_x,
        "scale_y" => &node.scale_y,
        "visible" => return Some(SceneValue::Bool(node.visible.get())),
        _ => return None,
    };
    Some(float(field))
}

fn node_3d_value(node: &Node3dFields, name: &str) -> Option<SceneValue> {
    let field = match name {
        "z" => &node.z,
        "pitch" => &node.pitch,
        "yaw" => &node.yaw,
        "scale_z" => &node.scale_z,
        _ => return None,
    };
    Some(float(field))
}

fn sprite_value<H: Host>(sprite: &SpriteFields<H>, name: &str) -> Option<SceneValue> {
    Some(match name {
        "texture" => SceneValue::Text(sprite.texture.get()),
        "width" => float(&sprite.width),
        "height" => float(&sprite.height),
        "layer" => whole(&sprite.layer),
        "pixelated" => SceneValue::Bool(sprite.pixelated.get()),
        "color" => color::<H>(&sprite.color),
        "columns" => whole(&sprite.columns),
        "rows" => whole(&sprite.rows),
        "frame" => whole(&sprite.frame),
        _ => return None,
    })
}

fn mesh_value<H: Host>(mesh: &MeshFields<H>, name: &str) -> Option<SceneValue> {
    let field = match name {
        "width" => &mesh.width,
        "height" => &mesh.height,
        "depth" => &mesh.depth,
        "radius" => &mesh.radius,
        "metallic" => &mesh.metallic,
        "roughness" => &mesh.roughness,
        "color" => return Some(color::<H>(&mesh.color)),
        "pixelated" => return Some(SceneValue::Bool(mesh.pixelated.get())),
        _ => return None,
    };
    Some(float(field))
}

fn light_value<H: Host>(light: &LightFields<H>, name: &str) -> Option<SceneValue> {
    let field = match name {
        "intensity" => &light.intensity,
        "range" => &light.range,
        "angle" => &light.angle,
        "blend" => &light.blend,
        "color" => return Some(color::<H>(&light.color)),
        _ => return None,
    };
    Some(float(field))
}

/// The fields of `Node`; `None` if it has no field `name` an animation can
/// change.
fn set_node<H: Host>(
    node: &NodeFields<H>,
    name: &str,
    value: &SceneValue,
) -> Option<Result<(), String>> {
    let field = match name {
        "x" => &node.x,
        "y" => &node.y,
        "rotation" => &node.rotation,
        "scale_x" => &node.scale_x,
        "scale_y" => &node.scale_y,
        "visible" => return Some(set_bool(&node.visible, name, value)),
        _ => return None,
    };
    Some(set_float(field, name, value))
}

fn set_node_3d(node: &Node3dFields, name: &str, value: &SceneValue) -> Option<Result<(), String>> {
    let field = match name {
        "z" => &node.z,
        "pitch" => &node.pitch,
        "yaw" => &node.yaw,
        "scale_z" => &node.scale_z,
        _ => return None,
    };
    Some(set_float(field, name, value))
}

fn set_sprite<H: Host>(
    sprite: &SpriteFields<H>,
    name: &str,
    value: &SceneValue,
) -> Option<Result<(), String>> {
    Some(match name {
        "texture" => set_text(&sprite.texture, name, value),
        "width" => set_float(&sprite.width, name, value),
        "height" => set_float(&sprite.height, name, value),
        "layer" => set_int(&sprite.layer, name, value),
        "pixelated" => set_bool(&sprite.pixelated, name, value),
        "color" => set_color::<H>(&sprite.color, name, value),
        "columns" => set_int(&sprite.columns, name, value),
        "rows" => set_int(&sprite.rows, name, value),
        "frame" => set_int(&sprite.frame, name, value),
        _ => return None,
    })
}

fn set_mesh<H: Host>(
    mesh: &MeshFields<H>,
    name: &str,
    value: &SceneValue,
) -> Option<Result<(), String>> {
    let field = match name {
        "width" => &mesh.width,
        "height" => &mesh.height,
        "depth" => &mesh.depth,
        "radius" => &mesh.radius,
        "metallic" => &mesh.metallic,
        "roughness" => &mesh.roughness,
        "color" => return Some(set_color::<H>(&mesh.color, name, value)),
        "pixelated" => return Some(set_bool(&mesh.pixelated, name, value)),
        _ => return None,
    };
    Some(set_float(field, name, value))
}

fn set_light<H: Host>(
    light: &LightFields<H>,
    name: &str,
    value: &SceneValue,
) -> Option<Result<(), String>> {
    let field = match name {
        "intensity" => &light.intensity,
        "range" => &light.range,
        "angle" => &light.angle,
        "blend" => &light.blend,
        "color" => return Some(set_color::<H>(&light.color, name, value)),
        _ => return None,
    };
    Some(set_float(field, name, value))
}

fn set_float(field: &Field<f32>, name: &str, value: &SceneValue) -> Result<(), String> {
    field.set(scene_fields::float(name, value)?);
    Ok(())
}

fn set_int(field: &Field<i32>, name: &str, value: &SceneValue) -> Result<(), String> {
    field.set(scene_fields::int(name, value)?);
    Ok(())
}

fn set_bool(field: &Field<bool>, name: &str, value: &SceneValue) -> Result<(), String> {
    field.set(scene_fields::boolean(name, value)?);
    Ok(())
}

fn set_text(field: &Field<String>, name: &str, value: &SceneValue) -> Result<(), String> {
    let text = scene_fields::text(name, value)?;
    // Unchanged texts are not copied again, frame after frame.
    if field.read(|current| *current != text) {
        field.set(text);
    }
    Ok(())
}

/// Gives `field` a new color: colors are shared objects, so the one it had
/// is left as it was for whoever else holds it.
fn set_color<H: Host>(
    field: &Field<H::Color>,
    name: &str,
    value: &SceneValue,
) -> Result<(), String> {
    let [r, g, b, a] = scene_fields::color(name, value)?;
    field.set(H::color(r, g, b, a));
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::test_host::{TestHost, TestNode};

    #[test]
    fn engine_fields_are_set_by_name() {
        let mut hero = TestNode::new("Hero");
        hero.sprite = Some(SpriteFields::new("hero.png".to_owned()));
        let hero = Rc::new(hero);

        set_property::<TestHost>(&hero, "x", &SceneValue::Float(2.5)).unwrap();
        set_property::<TestHost>(&hero, "rotation", &SceneValue::Int(1)).unwrap();
        set_property::<TestHost>(&hero, "visible", &SceneValue::Bool(false)).unwrap();
        set_property::<TestHost>(&hero, "frame", &SceneValue::Int(3)).unwrap();
        let red = SceneValue::Color([1.0, 0.0, 0.0, 1.0]);
        set_property::<TestHost>(&hero, "color", &red).unwrap();

        assert_eq!(hero.node.x.get(), 2.5);
        assert_eq!(hero.node.rotation.get(), 1.0);
        assert!(!hero.node.visible.get());
        let sprite = hero.sprite.as_ref().unwrap();
        assert_eq!(sprite.frame.get(), 3);
        assert_eq!(sprite.color.get(), [1.0, 0.0, 0.0, 1.0]);
    }

    #[test]
    fn values_must_fit_their_property() {
        let hero = Rc::new(TestNode::new("Hero"));

        let error = set_property::<TestHost>(&hero, "x", &SceneValue::Bool(true)).unwrap_err();

        assert!(error.contains("float"), "{error}");
        let error = set_property::<TestHost>(&hero, "z", &SceneValue::Float(1.0)).unwrap_err();
        assert!(error.contains("z`"), "2D nodes have no depth: {error}");
    }

    #[test]
    fn engine_fields_are_read_by_name() {
        let mut hero = TestNode::new("Hero");
        hero.sprite = Some(SpriteFields::new("hero.png".to_owned()));
        let hero = Rc::new(hero);
        hero.node.x.set(1.5);
        hero.sprite.as_ref().unwrap().frame.set(2);

        assert_eq!(property::<TestHost>(&hero, "x"), Ok(SceneValue::Float(1.5)));
        assert_eq!(property::<TestHost>(&hero, "frame"), Ok(SceneValue::Int(2)));
        assert_eq!(
            property::<TestHost>(&hero, "texture"),
            Ok(SceneValue::Text("hero.png".to_owned()))
        );
        assert_eq!(
            property::<TestHost>(&hero, "color"),
            Ok(SceneValue::Color([1.0, 1.0, 1.0, 1.0]))
        );
        assert!(property::<TestHost>(&hero, "z").is_err());
    }

    #[test]
    fn other_properties_are_the_script_class_exported_fields() {
        let enemy = Rc::new(TestNode::new("Enemy"));

        set_property::<TestHost>(&enemy, "speed", &SceneValue::Float(2.0)).unwrap();

        assert_eq!(enemy.calls(), ["speed 2"]);
    }
}
