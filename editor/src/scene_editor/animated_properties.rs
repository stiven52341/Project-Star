//! A scene node's properties by the names `.anim` tracks give them (the
//! fields of StarScript's node classes, decision 79), to key them from the
//! inspector and to preview animations on the scene.

use star_engine::prelude::{
    Color, LightNode, MeshShape, Quat, TextureFilter, Transform2D, Transform3D,
};
use star_engine::star_assets::AssetPath;
use star_engine::star_render::SpriteSheet;
use star_engine::star_scene::{
    Camera2dNode, Camera3dNode, FieldValue, Light2dNode, MeshOutline, NodeOutline, SpriteOutline,
};

/// The value of `property` of `node`; `None` if it has no such property,
/// or an `export` field the scene gives no value.
pub(super) fn value(node: &NodeOutline, property: &str) -> Option<FieldValue> {
    place_value(node, property)
        .or_else(|| {
            node.sprite
                .as_ref()
                .and_then(|sprite| sprite_value(sprite, property))
        })
        .or_else(|| {
            node.mesh
                .as_ref()
                .and_then(|mesh| mesh_value(mesh, property))
        })
        .or_else(|| {
            node.light
                .as_ref()
                .and_then(|light| light_value(light, property))
        })
        .or_else(|| {
            node.light_2d
                .as_ref()
                .and_then(|light| light_2d_value(light, property))
        })
        .or_else(|| camera_value(node.camera.as_ref(), node.camera_3d.as_ref(), property))
        .or_else(|| node.fields.get(property).cloned())
}

/// Gives `property` of `node` the value `value`, as an animation playing
/// does. Returns whether it could: whether the node has the property and
/// the value fits it. Other names are `export` fields of its class.
pub(super) fn set_value(node: &mut NodeOutline, property: &str, value: &FieldValue) -> bool {
    let engine = set_place(node, property, value)
        .or_else(|| {
            node.sprite
                .as_mut()
                .and_then(|sprite| set_sprite(sprite, property, value))
        })
        .or_else(|| {
            node.mesh
                .as_mut()
                .and_then(|mesh| set_mesh(mesh, property, value))
        })
        .or_else(|| {
            node.light
                .as_mut()
                .and_then(|light| set_light(light, property, value))
        })
        .or_else(|| {
            node.light_2d
                .as_mut()
                .and_then(|light| set_light_2d(light, property, value))
        })
        .or_else(|| {
            set_camera(
                node.camera.as_mut(),
                node.camera_3d.as_mut(),
                property,
                value,
            )
        });
    engine.unwrap_or_else(|| {
        node.fields.insert(property.to_owned(), value.clone());
        true
    })
}

fn float(value: f32) -> FieldValue {
    FieldValue::Float(f64::from(value))
}

#[expect(
    clippy::cast_possible_truncation,
    reason = "the engine keeps its decimals as f32"
)]
fn number(value: &FieldValue) -> Option<f32> {
    match value {
        #[expect(
            clippy::cast_precision_loss,
            reason = "an animated number is far below where f32 loses whole numbers"
        )]
        FieldValue::Int(number) => Some(*number as f32),
        FieldValue::Float(number) => Some(*number as f32),
        _ => None,
    }
}

fn whole(value: &FieldValue) -> Option<i64> {
    match value {
        FieldValue::Int(number) => Some(*number),
        _ => None,
    }
}

fn color_value(color: Color) -> FieldValue {
    FieldValue::Color([color.r, color.g, color.b, color.a])
}

fn color(value: &FieldValue) -> Option<Color> {
    match value {
        FieldValue::Color([r, g, b, a]) => Some(Color {
            r: *r,
            g: *g,
            b: *b,
            a: *a,
        }),
        _ => None,
    }
}

/// Puts `new` in `field` if there is one: `None` is a value the property
/// cannot take. Returns whether it put it.
fn put<Value>(field: &mut Value, new: Option<Value>) -> bool {
    new.map(|new| *field = new).is_some()
}

/// The names `Node` and `Node3D` give their place: 2D nodes have `x`,
/// `y`, `rotation` and the scales; 3D nodes add `z`, `pitch`, `yaw` and
/// `scale_z`, and their `rotation` is the roll.
fn place_value(node: &NodeOutline, property: &str) -> Option<FieldValue> {
    if property == "visible" {
        return Some(FieldValue::Bool(node.visible));
    }
    if let Some(place) = &node.transform_3d {
        return place_3d_value(place, property);
    }
    let place = node.transform.unwrap_or(Transform2D::IDENTITY);
    let number = match property {
        "x" => place.position.x,
        "y" => place.position.y,
        "rotation" => place.rotation,
        "scale_x" => place.scale.x,
        "scale_y" => place.scale.y,
        _ => return None,
    };
    Some(float(number))
}

fn place_3d_value(place: &Transform3D, property: &str) -> Option<FieldValue> {
    let angles = place.rotation.to_euler();
    let number = match property {
        "x" => place.position.x,
        "y" => place.position.y,
        "z" => place.position.z,
        "pitch" => angles.x,
        "yaw" => angles.y,
        "rotation" => angles.z,
        "scale_x" => place.scale.x,
        "scale_y" => place.scale.y,
        "scale_z" => place.scale.z,
        _ => return None,
    };
    Some(float(number))
}

fn set_place(node: &mut NodeOutline, property: &str, value: &FieldValue) -> Option<bool> {
    if property == "visible" {
        let truth = match value {
            FieldValue::Bool(truth) => Some(*truth),
            _ => None,
        };
        return Some(put(&mut node.visible, truth));
    }
    if let Some(place) = &mut node.transform_3d {
        return set_place_3d(place, property, value);
    }
    let mut place = node.transform.unwrap_or(Transform2D::IDENTITY);
    let field = match property {
        "x" => &mut place.position.x,
        "y" => &mut place.position.y,
        "rotation" => &mut place.rotation,
        "scale_x" => &mut place.scale.x,
        "scale_y" => &mut place.scale.y,
        _ => return None,
    };
    let fits = put(field, number(value));
    if fits {
        node.transform = Some(place);
    }
    Some(fits)
}

fn set_place_3d(place: &mut Transform3D, property: &str, value: &FieldValue) -> Option<bool> {
    let mut angles = place.rotation.to_euler();
    let field = match property {
        "x" => &mut place.position.x,
        "y" => &mut place.position.y,
        "z" => &mut place.position.z,
        "scale_x" => &mut place.scale.x,
        "scale_y" => &mut place.scale.y,
        "scale_z" => &mut place.scale.z,
        "pitch" => &mut angles.x,
        "yaw" => &mut angles.y,
        "rotation" => &mut angles.z,
        _ => return None,
    };
    let fits = put(field, number(value));
    if matches!(property, "pitch" | "yaw" | "rotation") && fits {
        place.rotation = Quat::from_euler(angles);
    }
    Some(fits)
}

fn sprite_value(sprite: &SpriteOutline, property: &str) -> Option<FieldValue> {
    Some(match property {
        "texture" => FieldValue::Text(sprite.texture.as_str().to_owned()),
        "width" => float(sprite.size.x),
        "height" => float(sprite.size.y),
        "layer" => FieldValue::Int(i64::from(sprite.layer)),
        "pixelated" => FieldValue::Bool(sprite.filter == TextureFilter::Pixelated),
        "color" => color_value(sprite.color),
        "columns" => FieldValue::Int(i64::from(sprite.sheet.columns())),
        "rows" => FieldValue::Int(i64::from(sprite.sheet.rows())),
        "frame" => FieldValue::Int(i64::from(sprite.sheet.frame())),
        _ => return None,
    })
}

fn set_sprite(sprite: &mut SpriteOutline, property: &str, value: &FieldValue) -> Option<bool> {
    let sheet = sprite.sheet;
    let (columns, rows, frame) = (sheet.columns(), sheet.rows(), sheet.frame());
    let small = |new: Option<i64>| new.and_then(|number| u16::try_from(number).ok());
    Some(match property {
        "texture" => {
            let path = match value {
                FieldValue::Text(text) => AssetPath::new(text).ok(),
                _ => None,
            };
            put(&mut sprite.texture, path)
        }
        "width" => put(&mut sprite.size.x, number(value)),
        "height" => put(&mut sprite.size.y, number(value)),
        "layer" => put(
            &mut sprite.layer,
            whole(value).and_then(|number| i32::try_from(number).ok()),
        ),
        "pixelated" => put(&mut sprite.filter, filter(value)),
        "color" => put(&mut sprite.color, color(value)),
        "columns" => put(
            &mut sprite.sheet,
            small(whole(value)).map(|columns| SpriteSheet::new(columns, rows, frame)),
        ),
        "rows" => put(
            &mut sprite.sheet,
            small(whole(value)).map(|rows| SpriteSheet::new(columns, rows, frame)),
        ),
        "frame" => put(
            &mut sprite.sheet,
            whole(value)
                .and_then(|number| u32::try_from(number).ok())
                .map(|frame| SpriteSheet::new(columns, rows, frame)),
        ),
        _ => return None,
    })
}

fn filter(value: &FieldValue) -> Option<TextureFilter> {
    match value {
        FieldValue::Bool(true) => Some(TextureFilter::Pixelated),
        FieldValue::Bool(false) => Some(TextureFilter::Smooth),
        _ => None,
    }
}

/// The sizes of a mesh, by shape: a plane's are its width and depth, a
/// sphere's its radius.
fn mesh_size<'shape>(shape: &'shape mut MeshShape, property: &str) -> Option<&'shape mut f32> {
    match (shape, property) {
        (MeshShape::Cube { size }, "width") => Some(&mut size.x),
        (MeshShape::Cube { size }, "height") => Some(&mut size.y),
        (MeshShape::Cube { size }, "depth") => Some(&mut size.z),
        (MeshShape::Plane { size }, "width") => Some(&mut size.x),
        (MeshShape::Plane { size }, "depth") => Some(&mut size.y),
        (MeshShape::Sphere { radius, .. }, "radius") => Some(radius),
        _ => None,
    }
}

fn mesh_value(mesh: &MeshOutline, property: &str) -> Option<FieldValue> {
    match property {
        "color" => Some(color_value(mesh.color)),
        "metallic" => Some(float(mesh.metallic)),
        "roughness" => Some(float(mesh.roughness)),
        "pixelated" => mesh
            .texture
            .as_ref()
            .map(|(_, filter)| FieldValue::Bool(*filter == TextureFilter::Pixelated)),
        _ => mesh_size(&mut mesh.shape.clone(), property).map(|size| float(*size)),
    }
}

fn set_mesh(mesh: &mut MeshOutline, property: &str, value: &FieldValue) -> Option<bool> {
    match property {
        "color" => Some(put(&mut mesh.color, color(value))),
        "metallic" => Some(put(&mut mesh.metallic, number(value))),
        "roughness" => Some(put(&mut mesh.roughness, number(value))),
        "pixelated" => mesh
            .texture
            .as_mut()
            .map(|(_, texture_filter)| put(texture_filter, filter(value))),
        _ => mesh_size(&mut mesh.shape, property).map(|size| put(size, number(value))),
    }
}

/// The values every light has, and those of point lights and spots.
struct LightParts<'light> {
    color: &'light mut Color,
    intensity: &'light mut f32,
    range: Option<&'light mut f32>,
    angle: Option<&'light mut f32>,
    blend: Option<&'light mut f32>,
}

fn light_parts(light: &mut LightNode) -> LightParts<'_> {
    match light {
        LightNode::Directional { color, intensity } => LightParts {
            color,
            intensity,
            range: None,
            angle: None,
            blend: None,
        },
        LightNode::Point {
            color,
            intensity,
            range,
        } => LightParts {
            color,
            intensity,
            range: Some(range),
            angle: None,
            blend: None,
        },
        LightNode::Spot {
            color,
            intensity,
            range,
            angle,
            blend,
        } => LightParts {
            color,
            intensity,
            range: Some(range),
            angle: Some(angle),
            blend: Some(blend),
        },
    }
}

fn light_2d_parts(light: &mut Light2dNode) -> LightParts<'_> {
    match light {
        Light2dNode::Point {
            color,
            intensity,
            range,
        } => LightParts {
            color,
            intensity,
            range: Some(range),
            angle: None,
            blend: None,
        },
        Light2dNode::Spot {
            color,
            intensity,
            range,
            angle,
            blend,
        } => LightParts {
            color,
            intensity,
            range: Some(range),
            angle: Some(angle),
            blend: Some(blend),
        },
    }
}

fn parts_value(parts: LightParts<'_>, property: &str) -> Option<FieldValue> {
    match property {
        "color" => Some(color_value(*parts.color)),
        "intensity" => Some(float(*parts.intensity)),
        "range" => parts.range.map(|range| float(*range)),
        "angle" => parts.angle.map(|angle| float(*angle)),
        "blend" => parts.blend.map(|blend| float(*blend)),
        _ => None,
    }
}

fn set_parts(parts: LightParts<'_>, property: &str, value: &FieldValue) -> Option<bool> {
    let field = match property {
        "color" => return Some(put(parts.color, color(value))),
        "intensity" => Some(parts.intensity),
        "range" => parts.range,
        "angle" => parts.angle,
        "blend" => parts.blend,
        _ => None,
    };
    field.map(|field| put(field, number(value)))
}

fn light_value(light: &LightNode, property: &str) -> Option<FieldValue> {
    parts_value(light_parts(&mut light.clone()), property)
}

fn set_light(light: &mut LightNode, property: &str, value: &FieldValue) -> Option<bool> {
    set_parts(light_parts(light), property, value)
}

fn light_2d_value(light: &Light2dNode, property: &str) -> Option<FieldValue> {
    parts_value(light_2d_parts(&mut light.clone()), property)
}

fn set_light_2d(light: &mut Light2dNode, property: &str, value: &FieldValue) -> Option<bool> {
    set_parts(light_2d_parts(light), property, value)
}

fn camera_value(
    camera: Option<&Camera2dNode>,
    camera_3d: Option<&Camera3dNode>,
    property: &str,
) -> Option<FieldValue> {
    match property {
        "view_height" => camera.map(|camera| float(camera.view_height)),
        "fov" => camera_3d.map(|camera| float(camera.fov)),
        _ => None,
    }
}

fn set_camera(
    camera: Option<&mut Camera2dNode>,
    camera_3d: Option<&mut Camera3dNode>,
    property: &str,
    value: &FieldValue,
) -> Option<bool> {
    match property {
        "view_height" => camera.map(|camera| put(&mut camera.view_height, number(value))),
        "fov" => camera_3d.map(|camera| put(&mut camera.fov, number(value))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use star_engine::prelude::{Vec2, Vec3};

    use super::*;
    use crate::project::ProjectDimension;
    use crate::scene_editor::components::NodeTemplate;

    fn sprite_node() -> NodeOutline {
        NodeTemplate::Sprite.build(
            "Hero".to_owned(),
            ProjectDimension::TwoD,
            &["hero.png".to_owned()],
        )
    }

    #[test]
    fn places_are_read_and_set_by_their_script_names() {
        let mut hero = sprite_node();
        hero.transform = None;

        assert_eq!(value(&hero, "scale_x"), Some(FieldValue::Float(1.0)));
        assert!(set_value(&mut hero, "x", &FieldValue::Int(3)));
        assert!(set_value(&mut hero, "visible", &FieldValue::Bool(false)));

        assert_eq!(hero.transform.unwrap().position, Vec2::new(3.0, 0.0));
        assert!(!hero.visible);
        assert!(!set_value(&mut hero, "x", &FieldValue::Bool(true)));
        assert_eq!(value(&hero, "z"), None);
    }

    #[test]
    fn three_d_nodes_turn_by_pitch_yaw_and_roll() {
        let mut cube = NodeTemplate::Cube.build("Cube".to_owned(), ProjectDimension::ThreeD, &[]);

        assert!(set_value(&mut cube, "yaw", &FieldValue::Float(0.5)));
        assert!(set_value(&mut cube, "z", &FieldValue::Float(2.0)));

        let place = cube.transform_3d.unwrap();
        assert!((place.rotation.to_euler() - Vec3::new(0.0, 0.5, 0.0)).length() < 1e-5);
        assert_eq!(place.position.z, 2.0);
        assert!(matches!(value(&cube, "width"), Some(FieldValue::Float(_))));
    }

    #[test]
    fn sprites_take_colors_frames_and_images() {
        let mut hero = sprite_node();
        let red = FieldValue::Color([1.0, 0.0, 0.0, 1.0]);

        assert!(set_value(&mut hero, "color", &red));
        assert!(set_value(&mut hero, "columns", &FieldValue::Int(4)));
        assert!(set_value(&mut hero, "frame", &FieldValue::Int(2)));
        assert!(set_value(
            &mut hero,
            "texture",
            &FieldValue::Text("knight.png".to_owned())
        ));

        assert_eq!(value(&hero, "color"), Some(red));
        assert_eq!(value(&hero, "frame"), Some(FieldValue::Int(2)));
        assert_eq!(hero.sprite.unwrap().texture.as_str(), "knight.png");
    }

    #[test]
    fn other_names_are_exported_fields() {
        let mut hero = sprite_node();

        assert_eq!(value(&hero, "speed"), None);
        assert!(set_value(&mut hero, "speed", &FieldValue::Float(2.0)));

        assert_eq!(value(&hero, "speed"), Some(FieldValue::Float(2.0)));
    }
}
