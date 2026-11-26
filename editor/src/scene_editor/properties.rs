//! The inspector of a scene node: every part it has, in its own section,
//! with its values to edit, and a menu to add the parts it lacks.

use eframe::egui;
use star_engine::prelude::{
    BodyKind, BodySettings, LightNode, MeshShape, Quat, Shape2d, Shape3d, ShapeKind, ShapeKind3d,
    TextureFilter, Transform2D, Transform3D, Vec2, Vec3,
};
use star_engine::star_assets::AssetPath;
use star_engine::star_render::{
    DEFAULT_SPHERE_RINGS, DEFAULT_SPHERE_SEGMENTS, MAX_SPHERE_DIVISIONS, MIN_SPHERE_RINGS,
    MIN_SPHERE_SEGMENTS, SpriteSheet,
};
use star_engine::star_scene::{
    Camera2dNode, Camera3dNode, Light2dNode, MeshOutline, NodeOutline, SpriteOutline,
};

use super::animation_rows::{AnimationFiles, animation_rows};
use super::components::{
    DEFAULT_CAPSULE_HEIGHT, DEFAULT_RADIUS, LightKind, add_component_menu, new_light, new_light_2d,
};
use super::exported_fields::{edit_exported, exported_fields};
use super::fields::{
    MIN_SIZE, angle_row, angle_slider_row, angles_row, checkbox_hint_row, checkbox_row, choice_row,
    color_row, fraction_row, number_from_row, number_row, size2_row, size3_row, tags_row, text_row,
    vec2_row, vec3_row, whole_row,
};
use star_script::semantic::ClassTable;

use crate::project::ProjectDimension;
use crate::style::PROPERTY_SPACING;
use crate::texts;

/// What the inspector edits a node with: the project's kind and the
/// images of the assets folder, to choose from.
pub(super) struct NodeContext<'context> {
    pub(super) dimension: ProjectDimension,
    pub(super) images: &'context [String],
    /// The `.frames` files of the assets folder, for sprites.
    pub(super) animations: AnimationFiles<'context>,
    /// The classes of the scripts, once checked, for `export` fields.
    pub(super) classes: Option<&'context ClassTable>,
}

/// Shows the parts of `node` to edit. Returns whether it changed.
pub(super) fn edit_node(
    ui: &mut egui::Ui,
    node: &mut NodeOutline,
    context: &NodeContext<'_>,
) -> bool {
    let texts = texts::current();
    let mut changed = section(ui, texts.node_section, None, |ui| edit_identity(ui, node));
    changed |= exported_section(ui, node, context.classes);
    changed |= optional_section(
        ui,
        texts.transform_section,
        &mut node.transform,
        edit_transform,
    );
    changed |= optional_section(
        ui,
        texts.transform_3d_section,
        &mut node.transform_3d,
        edit_transform_3d,
    );
    changed |= optional_section(ui, texts.sprite_section, &mut node.sprite, |ui, sprite| {
        edit_sprite(ui, sprite, context.images) | animation_rows(ui, sprite, context.animations)
    });
    changed |= optional_section(ui, texts.mesh_section, &mut node.mesh, |ui, mesh| {
        edit_mesh(ui, mesh, context.images)
    });
    changed |= optional_section(ui, texts.light_section, &mut node.light, edit_light);
    changed |= optional_section(
        ui,
        texts.light_2d_section,
        &mut node.light_2d,
        edit_light_2d,
    );
    changed |= optional_section(ui, texts.body_section, &mut node.body, edit_body);
    changed |= optional_section(ui, texts.body_3d_section, &mut node.body_3d, edit_body_3d);
    changed |= optional_section(ui, texts.camera_2d, &mut node.camera, edit_camera_2d);
    changed |= optional_section(ui, texts.camera_3d, &mut node.camera_3d, edit_camera_3d);
    ui.separator();
    changed | add_component_menu(ui, node, context.dimension, context.images)
}

/// The `export` fields of the node's class, if the scripts are checked and
/// it has any, or values the scene gives fields.
fn exported_section(
    ui: &mut egui::Ui,
    node: &mut NodeOutline,
    classes: Option<&ClassTable>,
) -> bool {
    let (Some(class), Some(classes)) = (node.class.clone(), classes) else {
        return false;
    };
    let fields = exported_fields(classes, &class).unwrap_or_default();
    if fields.is_empty() && node.fields.is_empty() {
        return false;
    }
    let title = texts::current().exported_section(&class);
    section(ui, &title, None, |ui| {
        edit_exported(ui, &mut node.fields, &fields)
    })
}

/// A part the node may lack: its section has a button that removes it.
fn optional_section<Part>(
    ui: &mut egui::Ui,
    title: &str,
    part: &mut Option<Part>,
    edit: impl FnOnce(&mut egui::Ui, &mut Part) -> bool,
) -> bool {
    let Some(value) = part.as_mut() else {
        return false;
    };
    let mut remove = false;
    let changed = section(ui, title, Some(&mut remove), |ui| edit(ui, value));
    if remove {
        *part = None;
    }
    changed || remove
}

/// A section that starts open, with its rows in a grid and, if `remove`
/// is given, a button that sets it. Returns whether a row changed.
fn section(
    ui: &mut egui::Ui,
    title: &str,
    remove: Option<&mut bool>,
    add_rows: impl FnOnce(&mut egui::Ui) -> bool,
) -> bool {
    egui::CollapsingHeader::new(title)
        .default_open(true)
        .show(ui, |ui| {
            let changed = egui::Grid::new(title)
                .num_columns(2)
                .spacing(PROPERTY_SPACING)
                .show(ui, add_rows)
                .inner;
            if let Some(remove) = remove {
                *remove = ui.small_button(texts::current().remove).clicked();
            }
            changed
        })
        .body_returned
        .unwrap_or(false)
}

fn edit_identity(ui: &mut egui::Ui, node: &mut NodeOutline) -> bool {
    let texts = texts::current();
    let mut changed = text_row(ui, texts.name, &mut node.name, "");
    let mut class = node.class.clone().unwrap_or_default();
    if text_row(ui, texts.class, &mut class, texts.no_class) {
        // A class of blanks is no class: scene files reject it.
        node.class = Some(class).filter(|class| !class.trim().is_empty());
        changed = true;
    }
    changed |= tags_row(ui, texts.tags, &mut node.tags, texts.new_tag_hint);
    changed | checkbox_hint_row(ui, texts.visible, &mut node.visible, texts.visible_hint)
}

fn edit_transform(ui: &mut egui::Ui, transform: &mut Transform2D) -> bool {
    let texts = texts::current();
    vec2_row(ui, texts.position, &mut transform.position)
        | angle_row(ui, texts.rotation, &mut transform.rotation)
        | vec2_row(ui, texts.scale, &mut transform.scale)
}

fn edit_transform_3d(ui: &mut egui::Ui, transform: &mut Transform3D) -> bool {
    let texts = texts::current();
    let moved = vec3_row(ui, texts.position, &mut transform.position);
    let mut angles = transform.rotation.to_euler();
    let turned = angles_row(ui, texts.rotation, &mut angles);
    if turned {
        transform.rotation = Quat::from_euler(angles);
    }
    moved | turned | vec3_row(ui, texts.scale, &mut transform.scale)
}

fn edit_sprite(ui: &mut egui::Ui, sprite: &mut SpriteOutline, images: &[String]) -> bool {
    let texts = texts::current();
    let mut texture = Some(sprite.texture.clone());
    let mut changed = image_row(ui, &mut texture, images, false);
    if let Some(texture) = texture {
        sprite.texture = texture;
    }
    changed |= filter_row(ui, &mut sprite.filter);
    changed |= size2_row(ui, texts.size, &mut sprite.size);
    changed |= vec2_row(ui, texts.anchor, &mut sprite.anchor);
    changed |= vec2_row(ui, texts.region, &mut sprite.region.min);
    changed |= vec2_row(ui, "", &mut sprite.region.max);
    changed |= sheet_rows(ui, &mut sprite.sheet);
    changed |= color_row(ui, texts.color, &mut sprite.color);
    changed | whole_row(ui, texts.layer, &mut sprite.layer, i32::MIN..=i32::MAX)
}

/// How a sprite's region is cut into cells, and the one it shows.
fn sheet_rows(ui: &mut egui::Ui, sheet: &mut SpriteSheet) -> bool {
    let texts = texts::current();
    let (mut columns, mut rows, mut frame) = (sheet.columns(), sheet.rows(), sheet.frame());
    let mut changed = whole_row(ui, texts.sheet_columns, &mut columns, 1..=u16::MAX);
    changed |= whole_row(ui, texts.sheet_rows, &mut rows, 1..=u16::MAX);
    let last = u32::from(columns) * u32::from(rows) - 1;
    changed |= whole_row(ui, texts.sheet_frame, &mut frame, 0..=last);
    if changed {
        *sheet = SpriteSheet::new(columns, rows, frame);
    }
    changed
}

/// The image of a sprite or a mesh, among those of the assets folder;
/// `optional` offers “No image” too.
fn image_row(
    ui: &mut egui::Ui,
    texture: &mut Option<AssetPath>,
    images: &[String],
    optional: bool,
) -> bool {
    let texts = texts::current();
    let current = texture.as_ref().map(|path| path.as_str().to_owned());
    let mut choices: Vec<(Option<&str>, &str)> = Vec::new();
    if optional {
        choices.push((None, texts.no_image));
    }
    // The image shown stays a choice even if it is not in the folder.
    if let Some(current) = current.as_deref()
        && !images.iter().any(|image| image == current)
    {
        choices.push((Some(current), current));
    }
    choices.extend(
        images
            .iter()
            .map(|image| (Some(image.as_str()), image.as_str())),
    );
    let mut selected = current.as_deref();
    let changed = choice_row(ui, texts.image, &mut selected, &choices);
    if changed {
        *texture = selected.and_then(|path| AssetPath::new(path).ok());
    }
    changed
}

fn filter_row(ui: &mut egui::Ui, filter: &mut TextureFilter) -> bool {
    let texts = texts::current();
    let choices = [TextureFilter::Smooth, TextureFilter::Pixelated]
        .map(|choice| (choice, texts.filter_name(choice)));
    choice_row(ui, texts.filter, filter, &choices)
}

/// The kinds of mesh shape, to choose one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MeshChoice {
    Cube,
    Plane,
    Sphere,
}

fn edit_mesh(ui: &mut egui::Ui, mesh: &mut MeshOutline, images: &[String]) -> bool {
    let texts = texts::current();
    let mut changed = edit_mesh_shape(ui, &mut mesh.shape);
    changed |= color_row(ui, texts.color, &mut mesh.color);
    changed |= fraction_row(ui, texts.metallic, &mut mesh.metallic);
    changed |= fraction_row(ui, texts.roughness, &mut mesh.roughness);
    let filter = mesh.texture.as_ref().map(|(_, filter)| *filter);
    let mut texture = mesh.texture.as_ref().map(|(path, _)| path.clone());
    if image_row(ui, &mut texture, images, true) {
        mesh.texture = texture.map(|path| (path, filter.unwrap_or_default()));
        changed = true;
    }
    if let Some((_, filter)) = &mut mesh.texture {
        changed |= filter_row(ui, filter);
    }
    changed
}

fn edit_mesh_shape(ui: &mut egui::Ui, shape: &mut MeshShape) -> bool {
    let texts = texts::current();
    let mut choice = match shape {
        MeshShape::Cube { .. } => MeshChoice::Cube,
        MeshShape::Plane { .. } => MeshChoice::Plane,
        MeshShape::Sphere { .. } => MeshChoice::Sphere,
    };
    let choices = [
        (MeshChoice::Cube, texts.cube),
        (MeshChoice::Plane, texts.plane),
        (MeshChoice::Sphere, texts.sphere),
    ];
    let chosen = choice_row(ui, texts.shape, &mut choice, &choices);
    if chosen {
        *shape = match choice {
            MeshChoice::Cube => MeshShape::Cube { size: Vec3::ONE },
            MeshChoice::Plane => MeshShape::Plane { size: Vec2::ONE },
            MeshChoice::Sphere => MeshShape::Sphere {
                radius: DEFAULT_RADIUS,
                segments: DEFAULT_SPHERE_SEGMENTS,
                rings: DEFAULT_SPHERE_RINGS,
            },
        };
    }
    let resized = match shape {
        MeshShape::Cube { size } => size3_row(ui, texts.size, size),
        MeshShape::Plane { size } => size2_row(ui, texts.size, size),
        MeshShape::Sphere {
            radius,
            segments,
            rings,
        } => {
            number_from_row(ui, texts.radius, radius, MIN_SIZE)
                | whole_row(
                    ui,
                    texts.segments,
                    segments,
                    MIN_SPHERE_SEGMENTS..=MAX_SPHERE_DIVISIONS,
                )
                | whole_row(
                    ui,
                    texts.rings,
                    rings,
                    MIN_SPHERE_RINGS..=MAX_SPHERE_DIVISIONS,
                )
        }
    };
    chosen | resized
}

/// The narrowest and widest a spot can shine, from its axis to its edge,
/// in degrees: scene files take anything between 0 and 90.
const SPOT_ANGLE_DEGREES: std::ops::RangeInclusive<f32> = 1.0..=89.0;

fn edit_light(ui: &mut egui::Ui, light: &mut LightNode) -> bool {
    let texts = texts::current();
    let mut kind = LightKind::of(light);
    let choices = LightKind::ALL.map(|choice| (choice, texts.light_kind_name(choice)));
    let mut changed = choice_row(ui, texts.body_kind, &mut kind, &choices);
    if changed {
        *light = new_light(kind);
    }
    let (LightNode::Directional { color, intensity }
    | LightNode::Point {
        color, intensity, ..
    }
    | LightNode::Spot {
        color, intensity, ..
    }) = light;
    changed |= color_row(ui, texts.color, color);
    changed |= number_from_row(ui, texts.intensity, intensity, 0.0);
    if let LightNode::Point { range, .. } | LightNode::Spot { range, .. } = light {
        changed |= number_from_row(ui, texts.range, range, MIN_SIZE);
    }
    if let LightNode::Spot { angle, blend, .. } = light {
        changed |= angle_slider_row(ui, texts.cone_angle, angle, SPOT_ANGLE_DEGREES);
        changed |= fraction_row(ui, texts.blend, blend);
    }
    changed
}

/// The narrowest and widest fields of view a 3D camera can be given, in
/// degrees: scene files take anything between 0 and 180.
const FIELD_OF_VIEW_DEGREES: std::ops::RangeInclusive<f32> = 1.0..=179.0;

fn edit_camera_2d(ui: &mut egui::Ui, camera: &mut Camera2dNode) -> bool {
    let texts = texts::current();
    number_from_row(ui, texts.view_height, &mut camera.view_height, MIN_SIZE)
        | checkbox_hint_row(
            ui,
            texts.main_camera,
            &mut camera.main,
            texts.main_camera_hint,
        )
}

fn edit_camera_3d(ui: &mut egui::Ui, camera: &mut Camera3dNode) -> bool {
    let texts = texts::current();
    angle_slider_row(
        ui,
        texts.field_of_view,
        &mut camera.fov,
        FIELD_OF_VIEW_DEGREES,
    ) | checkbox_hint_row(
        ui,
        texts.main_camera,
        &mut camera.main,
        texts.main_camera_hint,
    )
}

fn edit_light_2d(ui: &mut egui::Ui, light: &mut Light2dNode) -> bool {
    let texts = texts::current();
    let mut kind = LightKind::of_2d(light);
    let choices = LightKind::ALL_2D.map(|choice| (choice, texts.light_kind_name(choice)));
    let mut changed = choice_row(ui, texts.body_kind, &mut kind, &choices);
    if changed {
        *light = new_light_2d(kind);
    }
    let (Light2dNode::Point {
        color,
        intensity,
        range,
    }
    | Light2dNode::Spot {
        color,
        intensity,
        range,
        ..
    }) = light;
    changed |= color_row(ui, texts.color, color);
    changed |= number_from_row(ui, texts.intensity, intensity, 0.0);
    changed |= number_from_row(ui, texts.range, range, MIN_SIZE);
    if let Light2dNode::Spot { angle, blend, .. } = light {
        changed |= angle_slider_row(ui, texts.cone_angle, angle, SPOT_ANGLE_DEGREES);
        changed |= fraction_row(ui, texts.blend, blend);
    }
    changed
}

/// The kinds of 2D body shape, to choose one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum ShapeChoice {
    Circle,
    Rectangle,
    Capsule,
}

fn edit_body(ui: &mut egui::Ui, body: &mut BodySettings) -> bool {
    edit_shape(ui, &mut body.shape) | edit_body_settings(ui, body)
}

fn edit_shape(ui: &mut egui::Ui, shape: &mut Shape2d) -> bool {
    let texts = texts::current();
    let mut kind = shape.kind();
    let mut choice = match kind {
        ShapeKind::Circle { .. } => ShapeChoice::Circle,
        ShapeKind::Rectangle { .. } => ShapeChoice::Rectangle,
        ShapeKind::Capsule { .. } => ShapeChoice::Capsule,
    };
    let choices = [
        (ShapeChoice::Circle, texts.circle),
        (ShapeChoice::Rectangle, texts.rectangle),
        (ShapeChoice::Capsule, texts.capsule),
    ];
    if choice_row(ui, texts.shape, &mut choice, &choices) {
        kind = match choice {
            ShapeChoice::Circle => ShapeKind::Circle {
                radius: DEFAULT_RADIUS,
            },
            ShapeChoice::Rectangle => ShapeKind::Rectangle { size: Vec2::ONE },
            ShapeChoice::Capsule => ShapeKind::Capsule {
                height: DEFAULT_CAPSULE_HEIGHT,
                radius: DEFAULT_RADIUS,
            },
        };
    }
    match &mut kind {
        ShapeKind::Circle { radius } => {
            number_from_row(ui, texts.radius, radius, MIN_SIZE);
        }
        ShapeKind::Rectangle { size } => {
            size2_row(ui, texts.size, size);
        }
        ShapeKind::Capsule { height, radius } => {
            number_from_row(ui, texts.height, height, MIN_SIZE);
            number_from_row(ui, texts.radius, radius, MIN_SIZE);
        }
    }
    // The physics checks the sizes (a capsule at least as high as it is
    // wide); sizes it refuses leave the shape as it was.
    let rebuilt = match kind {
        ShapeKind::Circle { radius } => Shape2d::circle(radius),
        ShapeKind::Rectangle { size } => Shape2d::rectangle(size),
        ShapeKind::Capsule { height, radius } => Shape2d::capsule(height, radius),
    };
    replace_if_valid(shape, rebuilt.ok())
}

/// The kinds of 3D body shape, to choose one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape3dChoice {
    Sphere,
    Box,
    Capsule,
}

fn edit_body_3d(ui: &mut egui::Ui, body: &mut BodySettings<Shape3d>) -> bool {
    edit_shape_3d(ui, &mut body.shape) | edit_body_settings(ui, body)
}

fn edit_shape_3d(ui: &mut egui::Ui, shape: &mut Shape3d) -> bool {
    let texts = texts::current();
    let mut kind = shape.kind();
    let mut choice = match kind {
        ShapeKind3d::Sphere { .. } => Shape3dChoice::Sphere,
        ShapeKind3d::Box { .. } => Shape3dChoice::Box,
        ShapeKind3d::Capsule { .. } => Shape3dChoice::Capsule,
    };
    let choices = [
        (Shape3dChoice::Sphere, texts.sphere),
        (Shape3dChoice::Box, texts.box_shape),
        (Shape3dChoice::Capsule, texts.capsule),
    ];
    if choice_row(ui, texts.shape, &mut choice, &choices) {
        kind = match choice {
            Shape3dChoice::Sphere => ShapeKind3d::Sphere {
                radius: DEFAULT_RADIUS,
            },
            Shape3dChoice::Box => ShapeKind3d::Box { size: Vec3::ONE },
            Shape3dChoice::Capsule => ShapeKind3d::Capsule {
                height: DEFAULT_CAPSULE_HEIGHT,
                radius: DEFAULT_RADIUS,
            },
        };
    }
    match &mut kind {
        ShapeKind3d::Sphere { radius } => {
            number_from_row(ui, texts.radius, radius, MIN_SIZE);
        }
        ShapeKind3d::Box { size } => {
            size3_row(ui, texts.size, size);
        }
        ShapeKind3d::Capsule { height, radius } => {
            number_from_row(ui, texts.height, height, MIN_SIZE);
            number_from_row(ui, texts.radius, radius, MIN_SIZE);
        }
    }
    let rebuilt = match kind {
        ShapeKind3d::Sphere { radius } => Shape3d::sphere(radius),
        ShapeKind3d::Box { size } => Shape3d::cuboid(size),
        ShapeKind3d::Capsule { height, radius } => Shape3d::capsule(height, radius),
    };
    replace_if_valid(shape, rebuilt.ok())
}

/// Puts `new` in `value` if it is valid and different. Returns whether
/// `value` changed.
fn replace_if_valid<Value: PartialEq>(value: &mut Value, new: Option<Value>) -> bool {
    match new {
        Some(new) if new != *value => {
            *value = new;
            true
        }
        _ => false,
    }
}

/// What 2D and 3D bodies share: everything but their shape.
fn edit_body_settings<Shape>(ui: &mut egui::Ui, body: &mut BodySettings<Shape>) -> bool {
    let texts = texts::current();
    let choices = [BodyKind::Dynamic, BodyKind::Static, BodyKind::Kinematic]
        .map(|kind| (kind, texts.body_kind_name(kind)));
    choice_row(ui, texts.body_kind, &mut body.kind, &choices)
        | number_row(ui, texts.gravity_scale, &mut body.gravity_scale)
        | number_from_row(ui, texts.friction, &mut body.friction, 0.0)
        | number_from_row(ui, texts.bounciness, &mut body.bounciness, 0.0)
        | number_from_row(ui, texts.density, &mut body.density, MIN_SIZE)
        | checkbox_row(ui, texts.sensor, &mut body.is_sensor)
        | checkbox_row(ui, texts.fixed_rotation, &mut body.fixed_rotation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_valid_different_values_replace_the_old_one() {
        let mut value = 1;

        assert!(!replace_if_valid(&mut value, None));
        assert!(!replace_if_valid(&mut value, Some(1)));
        assert!(replace_if_valid(&mut value, Some(2)));
        assert_eq!(value, 2);
    }
}
