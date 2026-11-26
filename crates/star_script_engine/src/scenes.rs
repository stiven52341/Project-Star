//! Scenes of the game's asset folder, read for StarScript's `Scene.load`.

use star_engine::script_nodes::{ScriptBody, ScriptLight, ScriptMesh};
use star_engine::star_assets::{AssetFolder, AssetPath};
use star_engine::star_render::TextureFilter;
use star_engine::star_scene::{FieldValue, NodeOutline, SceneOutline};
use star_script_runtime::scene_fields::SceneValue;
use star_script_runtime::scenes::{
    SceneAnimator, SceneBody, SceneCamera, SceneCamera3D, SceneLight, SceneMesh, SceneNode,
    SceneSprite, SceneTransform, SceneTransform3D,
};

/// Reads the scene at `path` of the asset `folder`. Parts of a scene that
/// scripts have no field for yet (a sprite's region and anchor, the
/// divisions of a sphere) are left out.
///
/// # Errors
///
/// Why the scene could not be read, for the warning the game writes.
pub fn read_scene(folder: &AssetFolder, path: &str) -> Result<Vec<SceneNode>, String> {
    let outline = AssetPath::new(path)
        .and_then(|path| folder.read_text(&path))
        .and_then(|text| SceneOutline::from_text(&text))
        .map_err(|error| error.to_string())?;
    Ok(outline.into_nodes().into_iter().map(scene_node).collect())
}

/// A node of a scene file, with the words of StarScript's classes.
fn scene_node(node: NodeOutline) -> SceneNode {
    SceneNode {
        visible: node.visible,
        transform: node.transform.map(|place| SceneTransform {
            x: place.position.x,
            y: place.position.y,
            rotation: place.rotation,
            scale_x: place.scale.x,
            scale_y: place.scale.y,
        }),
        sprite: node.sprite.map(|image| SceneSprite {
            texture: image.texture.as_str().to_owned(),
            width: image.size.x,
            height: image.size.y,
            layer: image.layer,
            pixelated: image.filter == TextureFilter::Pixelated,
            color: [image.color.r, image.color.g, image.color.b, image.color.a],
            columns: i32::from(image.sheet.columns()),
            rows: i32::from(image.sheet.rows()),
            frame: i32::try_from(image.sheet.frame()).unwrap_or(0),
            frames: image
                .frames
                .map(|path| path.as_str().to_owned())
                .unwrap_or_default(),
            animation: image.animation.unwrap_or_default(),
        }),
        body: match (node.transform_3d.is_some(), node.body_3d, node.body) {
            (true, Some(settings), _) => Some(scene_body(&ScriptBody::from_settings_3d(&settings))),
            (_, _, Some(settings)) => Some(scene_body(&ScriptBody::from_settings(&settings))),
            _ => None,
        },
        transform_3d: node.transform_3d.map(|place| {
            let angles = place.rotation.to_euler();
            SceneTransform3D {
                x: place.position.x,
                y: place.position.y,
                z: place.position.z,
                pitch: angles.x,
                yaw: angles.y,
                roll: angles.z,
                scale_x: place.scale.x,
                scale_y: place.scale.y,
                scale_z: place.scale.z,
            }
        }),
        mesh: node
            .mesh
            .as_ref()
            .map(|outline| scene_mesh(&ScriptMesh::from_outline(outline))),
        fields: node
            .fields
            .into_iter()
            .map(|(name, value)| (name, scene_value(value)))
            .collect(),
        animator: node.animator.map(|animator| SceneAnimator {
            animations: animator.animations.as_str().to_owned(),
            animation: animator.animation.unwrap_or_default(),
        }),
        camera: node.camera.map(|view| SceneCamera {
            view_height: view.view_height,
            main: view.main,
        }),
        camera_3d: node.camera_3d.map(|view| SceneCamera3D {
            fov: view.fov,
            main: view.main,
        }),
        light_2d: node
            .light_2d
            .map(|shone| scene_light(&ScriptLight::from_light_2d(&shone))),
        light: node
            .light
            .map(|shone| scene_light(&ScriptLight::from_light(&shone))),
        name: node.name,
        class: node.class,
        tags: node.tags,
        parent: node.parent,
    }
}

/// The body of a scene node, with the words of StarScript's `Body`.
#[must_use]
pub fn scene_body(body: &ScriptBody<'_>) -> SceneBody {
    SceneBody {
        kind: String::from(body.kind),
        shape: String::from(body.shape),
        radius: body.radius,
        width: body.width,
        height: body.height,
        depth: body.depth,
        gravity_scale: body.gravity_scale,
        fixed_rotation: body.fixed_rotation,
        friction: body.friction,
        bounciness: body.bounciness,
        density: body.density,
        sensor: body.sensor,
    }
}

/// The mesh of a scene node, with the words of StarScript's `Mesh`.
fn scene_mesh(mesh: &ScriptMesh<'_>) -> SceneMesh {
    SceneMesh {
        shape: String::from(mesh.shape),
        width: mesh.width,
        height: mesh.height,
        depth: mesh.depth,
        radius: mesh.radius,
        color: [mesh.color.r, mesh.color.g, mesh.color.b, mesh.color.a],
        metallic: mesh.metallic,
        roughness: mesh.roughness,
        texture: String::from(mesh.texture),
        pixelated: mesh.pixelated,
    }
}

/// A light of a scene node, with the words of StarScript's lights.
fn scene_light(light: &ScriptLight<'_>) -> SceneLight {
    SceneLight {
        kind: String::from(light.kind),
        color: [light.color.r, light.color.g, light.color.b, light.color.a],
        intensity: light.intensity,
        range: light.range,
        angle: light.angle,
        blend: light.blend,
    }
}

/// The value of an exported field set in a scene.
pub(crate) fn scene_value(value: FieldValue) -> SceneValue {
    match value {
        FieldValue::Bool(truth) => SceneValue::Bool(truth),
        FieldValue::Int(number) => SceneValue::Int(number),
        FieldValue::Float(number) => SceneValue::Float(number),
        FieldValue::Text(text) => SceneValue::Text(text),
        FieldValue::Color(color) => SceneValue::Color(color),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use super::*;

    const SCENE: &str = r#"format = 1

[[node]]
name = "World"

[[node]]
name = "Ball"
class = "Ball"
parent = 0
tags = ["round"]
position = [1.0, 2.0]

[node.sprite]
texture = "sprites/ball.png"
size = [0.5, 0.5]

[node.body]
kind = "dynamic"
shape = "circle"
radius = 0.25

[node.fields]
speed = 3.5
"#;

    /// An asset folder of its own with `scenes/test.scene`.
    fn folder_with_scene(name: &str) -> AssetFolder {
        let root: PathBuf = std::env::temp_dir().join(format!("star_script_engine_{name}"));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("scenes")).expect("folder created");
        fs::write(root.join("scenes").join("test.scene"), SCENE).expect("scene written");
        AssetFolder::new(root)
    }

    #[test]
    fn scenes_are_read_with_their_nodes() {
        let nodes = read_scene(&folder_with_scene("read"), "scenes/test.scene").expect("read");

        assert_eq!(nodes.len(), 2);
        let ball = &nodes[1];
        assert_eq!(ball.name, "Ball");
        assert_eq!(ball.class.as_deref(), Some("Ball"));
        assert_eq!(ball.parent, Some(0));
        assert_eq!(ball.tags, ["round"]);
        let place = ball.transform.as_ref().expect("placed");
        assert!((place.x - 1.0).abs() < f32::EPSILON && (place.y - 2.0).abs() < f32::EPSILON);
        let sprite = ball.sprite.as_ref().expect("sprite");
        assert_eq!(sprite.texture, "sprites/ball.png");
        let body = ball.body.as_ref().expect("body");
        assert_eq!(
            (body.kind.as_str(), body.shape.as_str()),
            ("dynamic", "circle")
        );
        assert_eq!(ball.fields.len(), 1);
    }

    #[test]
    fn missing_scenes_are_explained() {
        let read = read_scene(&folder_with_scene("missing"), "scenes/other.scene");

        assert!(read.is_err());
    }
}
