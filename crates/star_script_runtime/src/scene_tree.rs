//! `Scene.load`: the nodes of a scene, built as objects of the classes it
//! names, with what the scene writes on top of what their `init()` leaves.

use crate::bodies::BodyFields;
use crate::nodes::{self, Host};
use crate::scenes::{self, SceneNode};

/// The class of a node a scene asks for, with what its `init` needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewNode<'scene> {
    /// `Node`
    Node,
    /// `Sprite` showing `texture`.
    Sprite {
        /// The image it shows.
        texture: &'scene str,
    },
    /// `Camera2D`
    Camera2D,
    /// `Light2D` of the kind `kind`.
    Light2D {
        /// The kind of light.
        kind: &'scene str,
    },
    /// `Node3D`
    Node3D,
    /// `Mesh` of the shape `shape`.
    Mesh {
        /// The shape it shows.
        shape: &'scene str,
    },
    /// `Light` of the kind `kind`.
    Light {
        /// The kind of light.
        kind: &'scene str,
    },
    /// `Camera3D`
    Camera3D,
    /// `Animator`
    Animator,
    /// `Bone2D`
    Bone2D,
    /// `Model` showing `file`.
    Model {
        /// The model file it shows.
        file: &'scene str,
    },
    /// A class of the program, created with `init()`.
    Script(&'scene str),
}

/// `static func load`: a new tree with the nodes of the scene at `path`,
/// out of the game until it is added; `None` if it cannot be loaded (the
/// reason is written once).
#[must_use]
pub fn load<H: Host>(path: &str) -> Option<H::Node> {
    let nodes = scenes::load(path)?;
    match build::<H>(&nodes) {
        Ok(root) => Some(root),
        Err(reason) => {
            scenes::reject(path, &reason);
            None
        }
    }
}

/// The objects of `nodes`, each one added to its parent; the root first.
///
/// # Errors
///
/// Why a node could not be built, naming it.
pub fn build<H: Host>(nodes: &[SceneNode]) -> Result<H::Node, String> {
    let mut built: Vec<H::Node> = Vec::with_capacity(nodes.len());
    for node in nodes {
        let object =
            build_node::<H>(node).map_err(|reason| format!("node \"{}\": {reason}", node.name))?;
        if let Some(parent) = node.parent.and_then(|index| built.get(index)) {
            nodes::add_child::<H>(parent, object.clone());
        }
        built.push(object);
    }
    built
        .into_iter()
        .next()
        .ok_or_else(|| String::from("it has no nodes"))
}

/// The object of one node: of its class, created as `init()` leaves it,
/// with what the scene writes on top.
fn build_node<H: Host>(node: &SceneNode) -> Result<H::Node, String> {
    let created = node.class.as_deref().unwrap_or_else(|| default_class(node));
    let object = H::create(node.name.clone(), new_node(created, node)?)?;
    // What a node without a class can be is the scene's choice: it is a
    // `Node` for what scripts may write.
    let class = node.class.as_deref().unwrap_or(NODE_CLASS);
    let fields = H::node(&object);
    fields.name.set(node.name.clone());
    if !node.tags.is_empty() {
        fields.tags.set(H::new_tags(node.tags.clone()));
    }
    fields.visible.set(node.visible);
    if !node.fields.is_empty() {
        H::set_exported(class, &object, &node.fields)?;
    }
    place::<H>(&object, node, class)?;
    show::<H>(&object, node, class)?;
    view::<H>(&object, node, class)?;
    animate::<H>(&object, node, class)?;
    joint::<H>(&object, node, class)?;
    if let Some(solid) = &node.body {
        let body = BodyFields::new(
            solid.kind.clone(),
            &solid.shape,
            solid.radius,
            [solid.width, solid.height, solid.depth],
        );
        body.gravity_scale.set(solid.gravity_scale);
        body.fixed_rotation.set(solid.fixed_rotation);
        body.friction.set(solid.friction);
        body.bounciness.set(solid.bounciness);
        body.density.set(solid.density);
        body.sensor.set(solid.sensor);
        fields.body.set(Some(H::new_body(body)));
    }
    Ok(object)
}

/// The class of the nodes a scene gives no class.
const NODE_CLASS: &str = "Node";

/// What to create for a node of `class`: the engine's classes take what
/// their `init` needs from the node, which must have it.
fn new_node<'scene>(
    class: &'scene str,
    node: &'scene SceneNode,
) -> Result<NewNode<'scene>, String> {
    let missing = |part: &str| Err(format!("a `{class}` needs a `[node.{part}]`"));
    Ok(match class {
        "Node" => NewNode::Node,
        "Sprite" => match &node.sprite {
            Some(image) => NewNode::Sprite {
                texture: &image.texture,
            },
            None => return missing("sprite"),
        },
        "Camera2D" => NewNode::Camera2D,
        "Light2D" => match &node.light_2d {
            Some(light) => NewNode::Light2D { kind: &light.kind },
            None => return missing("light_2d"),
        },
        "Node3D" => NewNode::Node3D,
        "Mesh" => match &node.mesh {
            Some(mesh) => NewNode::Mesh { shape: &mesh.shape },
            None => return missing("mesh"),
        },
        "Light" => match &node.light {
            Some(light) => NewNode::Light { kind: &light.kind },
            None => return missing("light"),
        },
        "Camera3D" => NewNode::Camera3D,
        "Animator" => NewNode::Animator,
        "Bone2D" => NewNode::Bone2D,
        "Model" => match &node.model {
            Some(file) => NewNode::Model { file },
            None => return missing("model"),
        },
        script => NewNode::Script(script),
    })
}

/// The class of a node whose class the scene does not name: the engine
/// class that can show what it has.
fn default_class(node: &SceneNode) -> &'static str {
    if node.sprite.is_some() {
        "Sprite"
    } else if node.model.is_some() {
        "Model"
    } else if node.mesh.is_some() {
        "Mesh"
    } else if node.light.is_some() {
        "Light"
    } else if node.light_2d.is_some() {
        "Light2D"
    } else if node.camera_3d.is_some() {
        "Camera3D"
    } else if node.camera.is_some() {
        "Camera2D"
    } else if node.animator.is_some() {
        "Animator"
    } else if node.bone.is_some() {
        "Bone2D"
    } else if node.transform_3d.is_some() {
        "Node3D"
    } else {
        "Node"
    }
}

/// Places `object` where the scene says, in 2D or in 3D.
fn place<H: Host>(object: &H::Node, node: &SceneNode, class: &str) -> Result<(), String> {
    let fields = H::node(object);
    if let Some(place) = &node.transform {
        fields.x.set(place.x);
        fields.y.set(place.y);
        fields.rotation.set(place.rotation);
        fields.scale_x.set(place.scale_x);
        fields.scale_y.set(place.scale_y);
    }
    if let Some(place) = &node.transform_3d {
        let node_3d = H::as_node_3d(object)
            .ok_or_else(|| format!("it is placed in 3D, but `{class}` is not a `Node3D`"))?;
        fields.x.set(place.x);
        fields.y.set(place.y);
        fields.rotation.set(place.roll);
        fields.scale_x.set(place.scale_x);
        fields.scale_y.set(place.scale_y);
        let depth = H::node_3d(&node_3d);
        depth.z.set(place.z);
        depth.pitch.set(place.pitch);
        depth.yaw.set(place.yaw);
        depth.scale_z.set(place.scale_z);
    }
    Ok(())
}

/// Gives `object` the image, mesh and lights the scene says it has.
fn show<H: Host>(object: &H::Node, node: &SceneNode, class: &str) -> Result<(), String> {
    let not_a = |what: &str, wanted: &str| format!("it {what}, but `{class}` is not a `{wanted}`");
    let color = |[r, g, b, a]: [f32; 4]| H::color(r, g, b, a);
    if let Some(image) = &node.sprite {
        H::with_sprite(object, |look| {
            look.texture.set(image.texture.clone());
            look.width.set(image.width);
            look.height.set(image.height);
            look.layer.set(image.layer);
            look.pixelated.set(image.pixelated);
            look.color.set(color(image.color));
            look.columns.set(image.columns);
            look.rows.set(image.rows);
            look.frame.set(image.frame);
            look.frames.set(image.frames.clone());
        })
        .ok_or_else(|| not_a("shows an image", "Sprite"))?;
        if !image.animation.is_empty() {
            crate::animations::play::<H>(object, &image.animation);
        }
    }
    if let Some(shown) = &node.mesh {
        H::with_mesh(object, |look| {
            look.shape.set(shown.shape.clone());
            look.width.set(shown.width);
            look.height.set(shown.height);
            look.depth.set(shown.depth);
            look.radius.set(shown.radius);
            look.color.set(color(shown.color));
            look.metallic.set(shown.metallic);
            look.roughness.set(shown.roughness);
            look.texture.set(shown.texture.clone());
            look.pixelated.set(shown.pixelated);
        })
        .ok_or_else(|| not_a("shows a mesh", "Mesh"))?;
    }
    let shine = |look: &crate::node_classes::LightFields<H>, shone: &scenes::SceneLight| {
        look.kind.set(shone.kind.clone());
        look.color.set(color(shone.color));
        look.intensity.set(shone.intensity);
        look.range.set(shone.range);
        look.angle.set(shone.angle);
        look.blend.set(shone.blend);
    };
    if let Some(shone) = &node.light {
        H::with_light(object, |look| shine(look, shone))
            .ok_or_else(|| not_a("shines a light", "Light"))?;
    }
    if let Some(shone) = &node.light_2d {
        H::with_light_2d(object, |look| shine(look, shone))
            .ok_or_else(|| not_a("lights the sprites", "Light2D"))?;
    }
    Ok(())
}

/// Gives `object` the animations the scene says it plays, and plays the
/// first one.
fn animate<H: Host>(object: &H::Node, node: &SceneNode, class: &str) -> Result<(), String> {
    let Some(animator) = &node.animator else {
        return Ok(());
    };
    H::with_animator(object, |fields| {
        fields.animations.set(animator.animations.clone());
        fields.states.set(animator.states.clone());
    })
    .ok_or_else(|| format!("it plays animations, but `{class}` is not an `Animator`"))?;
    if !animator.animation.is_empty() {
        crate::tracks::play::<H>(object, &animator.animation);
    }
    Ok(())
}

/// Gives `object` the length of the bone the scene says it is.
fn joint<H: Host>(object: &H::Node, node: &SceneNode, class: &str) -> Result<(), String> {
    let Some(bone) = &node.bone else {
        return Ok(());
    };
    H::with_bone(object, |fields| fields.length.set(bone.length))
        .ok_or_else(|| format!("it is a bone, but `{class}` is not a `Bone2D`"))
}

/// Gives `object` the camera settings the scene says it has.
fn view<H: Host>(object: &H::Node, node: &SceneNode, class: &str) -> Result<(), String> {
    if let Some(seen) = &node.camera {
        let camera = H::as_camera_2d(object)
            .ok_or_else(|| format!("it is a 2D camera, but `{class}` is not a `Camera2D`"))?;
        let look = H::camera_2d(&camera);
        look.view_height.set(seen.view_height);
        look.main.set(seen.main);
    }
    if let Some(seen) = &node.camera_3d {
        let camera = H::as_camera_3d(object)
            .ok_or_else(|| format!("it is a 3D camera, but `{class}` is not a `Camera3D`"))?;
        let look = H::camera_3d(&camera);
        look.fov.set(seen.fov);
        look.main.set(seen.main);
    }
    Ok(())
}

#[cfg(test)]
mod tests;
