//! The inspector of a scene node: every part it has, in its own section,
//! with its values to edit, and a menu to add the parts it lacks.

use std::path::Path;

use eframe::egui;
use star_engine::prelude::{
    BodyKind, BodySettings, LightNode, MeshShape, Quat, Shape2d, Shape3d, ShapeKind, ShapeKind3d,
    TextureFilter, Transform2D, Transform3D, Vec2, Vec3,
};
use star_engine::star_assets::{AssetPath, BuiltinImage};
use star_engine::star_render::{
    DEFAULT_SPHERE_RINGS, DEFAULT_SPHERE_SEGMENTS, MAX_SPHERE_DIVISIONS, MIN_SPHERE_RINGS,
    MIN_SPHERE_SEGMENTS, SpriteSheet,
};
use star_engine::star_scene::{
    Bone2dNode, Camera2dNode, Camera3dNode, Light2dNode, MeshOutline, ModelNode, NodeOutline,
    SoundPlayerNode, SpriteLook, SpriteOutline, shaped_by_sprite,
};

use super::animation_rows::{AssetFiles, animation_rows};
use super::animator_rows::edit_animator;
use super::body_fit::{Shown, fitted};
use super::components::{
    DEFAULT_CAPSULE_HEIGHT, DEFAULT_RADIUS, LightKind, add_component_menu, new_light, new_light_2d,
};
use super::exported_fields::{edit_exported, exported_fields};
use super::fields::{
    MIN_SIZE, angle_row, angle_slider_row, angles_row, checkbox_hint_row, checkbox_row, choice_row,
    color_row, fraction_row, number_from_row, number_row, size2_row, size3_row, tags_row, text_row,
    vec2_row, vec3_row, whole_row,
};
use super::key_buttons::{KeyButtons, keyed};
use star_script::semantic::ClassTable;

use crate::project::{ProjectDimension, read_sound_groups};
use crate::style::PROPERTY_SPACING;
use crate::texts;

/// What the inspector edits a node with: the project's kind and the
/// images of the assets folder, to choose from.
pub(super) struct NodeContext<'context> {
    pub(super) dimension: ProjectDimension,
    pub(super) images: &'context [String],
    /// The `.frames` files of the assets folder, for sprites, and its
    /// `.anim` files, for animators.
    pub(super) animations: AssetFiles<'context>,
    /// The classes of the scripts, once checked, for `export` fields.
    pub(super) classes: Option<&'context ClassTable>,
    /// The key buttons of the node's properties, while the timeline of
    /// an animator that animates it is shown.
    pub(super) keys: Option<&'context KeyButtons>,
}

/// Shows the parts of `node` to edit. Returns whether it changed.
pub(super) fn edit_node(
    ui: &mut egui::Ui,
    node: &mut NodeOutline,
    context: &NodeContext<'_>,
) -> bool {
    let texts = texts::current();
    let keys = context.keys;
    let mut changed = section(ui, texts.node_section, None, |ui| {
        edit_identity(ui, node, keys)
    });
    changed |= exported_section(ui, node, context);
    changed |= optional_section(
        ui,
        texts.transform_section,
        &mut node.transform,
        |ui, transform| edit_transform(ui, transform, keys),
    );
    changed |= optional_section(
        ui,
        texts.transform_3d_section,
        &mut node.transform_3d,
        |ui, transform| edit_transform_3d(ui, transform, keys),
    );
    changed |= optional_section(ui, texts.sprite_section, &mut node.sprite, |ui, sprite| {
        edit_sprite(ui, sprite, context.images, keys)
            | animation_rows(ui, sprite, context.animations)
    });
    changed |= optional_section(ui, texts.mesh_section, &mut node.mesh, |ui, mesh| {
        edit_mesh(ui, mesh, context.images, keys)
    });
    changed |= optional_section(ui, texts.light_section, &mut node.light, |ui, light| {
        edit_light(ui, light, keys)
    });
    changed |= optional_section(
        ui,
        texts.light_2d_section,
        &mut node.light_2d,
        |ui, light| edit_light_2d(ui, light, keys),
    );
    let look = node
        .sprite
        .as_ref()
        .map(|sprite| (sprite.texture.clone(), sprite.size, sprite.anchor));
    let follows_sprite = look
        .as_ref()
        .is_some_and(|(texture, _, _)| texture.builtin_image().is_some());
    changed |= optional_section(ui, texts.body_section, &mut node.body, |ui, body| {
        edit_body(ui, body, follows_sprite)
    });
    // A body on a square or circle of the engine is that shape, and the
    // file says so.
    if let (Some(body), Some((texture, size, anchor))) = (&mut node.body, &look) {
        let shaped = shaped_by_sprite(
            *body,
            SpriteLook {
                texture: texture.as_str(),
                size: *size,
                anchor: *anchor,
            },
        );
        changed |= replace_if_valid(body, Some(shaped));
    }
    let shown = Shown::of(node);
    let fit = shown
        .as_ref()
        .map(|shown| (shown, context.animations.folder));
    changed |= optional_section(ui, texts.body_3d_section, &mut node.body_3d, |ui, body| {
        edit_body_3d(ui, body, fit)
    });
    changed |= optional_section(ui, texts.camera_2d, &mut node.camera, |ui, camera| {
        edit_camera_2d(ui, camera, keys)
    });
    changed |= optional_section(ui, texts.camera_3d, &mut node.camera_3d, |ui, camera| {
        edit_camera_3d(ui, camera, keys)
    });
    changed |= optional_section(
        ui,
        texts.sound_player_section(),
        &mut node.sound_player,
        |ui, player| {
            edit_sound_player(
                ui,
                player,
                context.animations.sounds,
                context.animations.folder,
            )
        },
    );
    changed |= optional_section(ui, texts.model_section(), &mut node.model, |ui, model| {
        edit_model(ui, model, context.animations.models)
    });
    changed |= optional_section(ui, texts.bone_2d(), &mut node.bone, |ui, bone| {
        edit_bone(ui, bone, keys)
    });
    changed |= optional_section(
        ui,
        texts.animator_section(),
        &mut node.animator,
        |ui, animator| edit_animator(ui, animator, context.animations),
    );
    ui.separator();
    changed | add_component_menu(ui, node, context)
}

/// The `export` fields of the node's class, if the scripts are checked and
/// it has any, or values the scene gives fields.
fn exported_section(ui: &mut egui::Ui, node: &mut NodeOutline, context: &NodeContext<'_>) -> bool {
    let (Some(class), Some(classes)) = (node.class.clone(), context.classes) else {
        return false;
    };
    let fields = exported_fields(classes, &class).unwrap_or_default();
    if fields.is_empty() && node.fields.is_empty() {
        return false;
    }
    let title = texts::current().exported_section(&class);
    section(ui, &title, None, |ui| {
        edit_exported(ui, &mut node.fields, &fields, context.keys)
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

fn edit_identity(ui: &mut egui::Ui, node: &mut NodeOutline, keys: Option<&KeyButtons>) -> bool {
    let texts = texts::current();
    let mut changed = text_row(ui, texts.name, &mut node.name, "");
    let mut class = node.class.clone().unwrap_or_default();
    if text_row(ui, texts.class, &mut class, texts.no_class) {
        // A class of blanks is no class: scene files reject it.
        node.class = Some(class).filter(|class| !class.trim().is_empty());
        changed = true;
    }
    changed |= tags_row(ui, texts.tags, &mut node.tags, texts.new_tag_hint);
    changed
        | checkbox_hint_row(
            ui,
            keyed(keys, texts.visible, &["visible"]),
            &mut node.visible,
            texts.visible_hint,
        )
}

fn edit_transform(
    ui: &mut egui::Ui,
    transform: &mut Transform2D,
    keys: Option<&KeyButtons>,
) -> bool {
    let texts = texts::current();
    vec2_row(
        ui,
        keyed(keys, texts.position, &["x", "y"]),
        &mut transform.position,
    ) | angle_row(
        ui,
        keyed(keys, texts.rotation, &["rotation"]),
        &mut transform.rotation,
    ) | vec2_row(
        ui,
        keyed(keys, texts.scale, &["scale_x", "scale_y"]),
        &mut transform.scale,
    )
}

/// A 3D node's rotation is its `pitch`, its `yaw` and its `rotation` (the
/// roll), as StarScript's `Node3D` names them.
fn edit_transform_3d(
    ui: &mut egui::Ui,
    transform: &mut Transform3D,
    keys: Option<&KeyButtons>,
) -> bool {
    let texts = texts::current();
    let moved = vec3_row(
        ui,
        keyed(keys, texts.position, &["x", "y", "z"]),
        &mut transform.position,
    );
    let mut angles = transform.rotation.to_euler();
    let turned = angles_row(
        ui,
        keyed(keys, texts.rotation, &["pitch", "yaw", "rotation"]),
        &mut angles,
    );
    if turned {
        transform.rotation = Quat::from_euler(angles);
    }
    moved
        | turned
        | vec3_row(
            ui,
            keyed(keys, texts.scale, &["scale_x", "scale_y", "scale_z"]),
            &mut transform.scale,
        )
}

fn edit_sprite(
    ui: &mut egui::Ui,
    sprite: &mut SpriteOutline,
    images: &[String],
    keys: Option<&KeyButtons>,
) -> bool {
    let texts = texts::current();
    let mut texture = Some(sprite.texture.clone());
    let mut changed = image_row(ui, &mut texture, images, false, keys);
    if let Some(texture) = texture {
        sprite.texture = texture;
    }
    changed |= filter_row(ui, &mut sprite.filter, keys);
    changed |= size2_row(
        ui,
        keyed(keys, texts.size, &["width", "height"]),
        &mut sprite.size,
    );
    changed |= vec2_row(ui, texts.anchor, &mut sprite.anchor);
    changed |= vec2_row(ui, texts.region, &mut sprite.region.min);
    changed |= vec2_row(ui, "", &mut sprite.region.max);
    changed |= sheet_rows(ui, &mut sprite.sheet, keys);
    changed |= color_row(ui, keyed(keys, texts.color, &["color"]), &mut sprite.color);
    changed
        | whole_row(
            ui,
            keyed(keys, texts.layer, &["layer"]),
            &mut sprite.layer,
            i32::MIN..=i32::MAX,
        )
}

/// How a sprite's region is cut into cells, and the one it shows.
fn sheet_rows(ui: &mut egui::Ui, sheet: &mut SpriteSheet, keys: Option<&KeyButtons>) -> bool {
    let texts = texts::current();
    let (mut columns, mut rows, mut frame) = (sheet.columns(), sheet.rows(), sheet.frame());
    let mut changed = whole_row(
        ui,
        keyed(keys, texts.sheet_columns, &["columns"]),
        &mut columns,
        1..=u16::MAX,
    );
    changed |= whole_row(
        ui,
        keyed(keys, texts.sheet_rows, &["rows"]),
        &mut rows,
        1..=u16::MAX,
    );
    let last = u32::from(columns) * u32::from(rows) - 1;
    changed |= whole_row(
        ui,
        keyed(keys, texts.sheet_frame, &["frame"]),
        &mut frame,
        0..=last,
    );
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
    keys: Option<&KeyButtons>,
) -> bool {
    let texts = texts::current();
    let current = texture.as_ref().map(|path| path.as_str().to_owned());
    let mut choices: Vec<(Option<&str>, &str)> = Vec::new();
    if optional {
        choices.push((None, texts.no_image));
    }
    choices.extend(BuiltinImage::ALL.map(|image| (Some(image.path()), texts.builtin_image(image))));
    // The image shown stays a choice even if it is not in the folder.
    if let Some(current) = current.as_deref()
        && !images.iter().any(|image| image == current)
        && BuiltinImage::of(current).is_none()
    {
        choices.push((Some(current), current));
    }
    choices.extend(
        images
            .iter()
            .map(|image| (Some(image.as_str()), image.as_str())),
    );
    let mut selected = current.as_deref();
    // Only a sprite's image can be animated: a mesh's may be none.
    let keys = keys.filter(|_| !optional);
    let changed = choice_row(
        ui,
        keyed(keys, texts.image, &["texture"]),
        &mut selected,
        &choices,
    );
    if changed {
        *texture = selected.and_then(|path| AssetPath::new(path).ok());
    }
    changed
}

fn filter_row(ui: &mut egui::Ui, filter: &mut TextureFilter, keys: Option<&KeyButtons>) -> bool {
    let texts = texts::current();
    let choices = [TextureFilter::Smooth, TextureFilter::Pixelated]
        .map(|choice| (choice, texts.filter_name(choice)));
    choice_row(
        ui,
        keyed(keys, texts.filter, &["pixelated"]),
        filter,
        &choices,
    )
}

/// The kinds of mesh shape, to choose one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum MeshChoice {
    Cube,
    Plane,
    Sphere,
}

fn edit_mesh(
    ui: &mut egui::Ui,
    mesh: &mut MeshOutline,
    images: &[String],
    keys: Option<&KeyButtons>,
) -> bool {
    let texts = texts::current();
    let mut changed = edit_mesh_shape(ui, &mut mesh.shape, keys);
    changed |= color_row(ui, keyed(keys, texts.color, &["color"]), &mut mesh.color);
    changed |= fraction_row(
        ui,
        keyed(keys, texts.metallic, &["metallic"]),
        &mut mesh.metallic,
    );
    changed |= fraction_row(
        ui,
        keyed(keys, texts.roughness, &["roughness"]),
        &mut mesh.roughness,
    );
    let filter = mesh.texture.as_ref().map(|(_, filter)| *filter);
    let mut texture = mesh.texture.as_ref().map(|(path, _)| path.clone());
    if image_row(ui, &mut texture, images, true, keys) {
        mesh.texture = texture.map(|path| (path, filter.unwrap_or_default()));
        changed = true;
    }
    if let Some((_, filter)) = &mut mesh.texture {
        changed |= filter_row(ui, filter, keys);
    }
    changed
}

/// A mesh's sizes as StarScript's `Mesh` names them: a plane's are its
/// `width` and `depth`, a sphere's its `radius`.
fn edit_mesh_shape(ui: &mut egui::Ui, shape: &mut MeshShape, keys: Option<&KeyButtons>) -> bool {
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
        MeshShape::Cube { size } => size3_row(
            ui,
            keyed(keys, texts.size, &["width", "height", "depth"]),
            size,
        ),
        MeshShape::Plane { size } => {
            size2_row(ui, keyed(keys, texts.size, &["width", "depth"]), size)
        }
        MeshShape::Sphere {
            radius,
            segments,
            rings,
        } => {
            number_from_row(ui, keyed(keys, texts.radius, &["radius"]), radius, MIN_SIZE)
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

fn edit_light(ui: &mut egui::Ui, light: &mut LightNode, keys: Option<&KeyButtons>) -> bool {
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
    changed |= light_rows(ui, color, intensity, keys);
    if let LightNode::Point { range, .. } | LightNode::Spot { range, .. } = light {
        changed |= range_row(ui, range, keys);
    }
    if let LightNode::Spot { angle, blend, .. } = light {
        changed |= spot_rows(ui, angle, blend, keys);
    }
    changed
}

/// What every light has: its color and how bright it is.
fn light_rows(
    ui: &mut egui::Ui,
    color: &mut star_engine::prelude::Color,
    intensity: &mut f32,
    keys: Option<&KeyButtons>,
) -> bool {
    let texts = texts::current();
    color_row(ui, keyed(keys, texts.color, &["color"]), color)
        | number_from_row(
            ui,
            keyed(keys, texts.intensity, &["intensity"]),
            intensity,
            0.0,
        )
}

fn range_row(ui: &mut egui::Ui, range: &mut f32, keys: Option<&KeyButtons>) -> bool {
    let texts = texts::current();
    number_from_row(ui, keyed(keys, texts.range, &["range"]), range, MIN_SIZE)
}

/// How wide a spot shines and how soft its edge is.
fn spot_rows(
    ui: &mut egui::Ui,
    angle: &mut f32,
    blend: &mut f32,
    keys: Option<&KeyButtons>,
) -> bool {
    let texts = texts::current();
    angle_slider_row(
        ui,
        keyed(keys, texts.cone_angle, &["angle"]),
        angle,
        SPOT_ANGLE_DEGREES,
    ) | fraction_row(ui, keyed(keys, texts.blend, &["blend"]), blend)
}

/// The narrowest and widest fields of view a 3D camera can be given, in
/// degrees: scene files take anything between 0 and 180.
const FIELD_OF_VIEW_DEGREES: std::ops::RangeInclusive<f32> = 1.0..=179.0;

fn edit_camera_2d(ui: &mut egui::Ui, camera: &mut Camera2dNode, keys: Option<&KeyButtons>) -> bool {
    let texts = texts::current();
    number_from_row(
        ui,
        keyed(keys, texts.view_height, &["view_height"]),
        &mut camera.view_height,
        MIN_SIZE,
    ) | checkbox_hint_row(
        ui,
        texts.main_camera,
        &mut camera.main,
        texts.main_camera_hint,
    )
}

/// The rows of a sound player: its sound, among the `sounds` of the
/// `assets` folder, its volume, whether it loops and plays on start, and
/// its group, among those of the project.
fn edit_sound_player(
    ui: &mut egui::Ui,
    player: &mut SoundPlayerNode,
    sounds: &[String],
    assets: &Path,
) -> bool {
    let texts = texts::current();
    let current = player.sound.as_str().to_owned();
    let mut chosen = Some(current.as_str());
    let choices: Vec<(Option<&str>, &str)> = std::iter::once(current.as_str())
        .filter(|current| !sounds.iter().any(|sound| sound == current))
        .chain(sounds.iter().map(String::as_str))
        .map(|sound| (Some(sound), sound))
        .collect();
    let mut changed = choice_row(ui, texts.sound_file(), &mut chosen, &choices);
    if changed && let Some(sound) = chosen.and_then(|path| AssetPath::new(path).ok()) {
        player.sound = sound;
    }
    changed |= number_from_row(ui, texts.volume(), &mut player.volume, 0.0);
    changed |= checkbox_row(ui, texts.looping(), &mut player.looping);
    changed |= checkbox_row(ui, texts.play_on_start(), &mut player.play_on_start);
    changed |= group_row(ui, &mut player.group, assets);
    changed |= checkbox_row(ui, texts.spatial_sound(), &mut player.spatial);
    if player.spatial {
        changed |= number_from_row(ui, texts.near_distance(), &mut player.near, 0.0);
        let near = player.near;
        changed |= number_from_row(ui, texts.far_distance(), &mut player.far, near);
    }
    changed
}

/// The group a player plays in, chosen among the groups of the project
/// (read from its `project.toml` only while the list is open), or none.
fn group_row(ui: &mut egui::Ui, group: &mut Option<String>, assets: &Path) -> bool {
    let texts = texts::current();
    ui.label(texts.sound_group());
    let mut chosen = None;
    egui::ComboBox::from_id_salt("sound_group")
        .selected_text(group.as_deref().unwrap_or(texts.no_group()))
        .show_ui(ui, |ui| {
            if ui
                .selectable_label(group.is_none(), texts.no_group())
                .clicked()
            {
                chosen = Some(None);
            }
            let project = assets.parent().unwrap_or(assets);
            for (name, _) in read_sound_groups(project) {
                let current = group.as_deref() == Some(name.as_str());
                if ui.selectable_label(current, &name).clicked() {
                    chosen = Some(Some(name));
                }
            }
        });
    ui.end_row();
    match chosen {
        Some(new) if new != *group => {
            *group = new;
            true
        }
        _ => false,
    }
}

/// The model's file, among the `models` of the assets folder.
fn edit_model(ui: &mut egui::Ui, model: &mut ModelNode, models: &[String]) -> bool {
    let texts = texts::current();
    ui.weak(texts.model_file());
    let mut chosen = None;
    egui::ComboBox::from_id_salt("model_file")
        .selected_text(model.file.as_str())
        .show_ui(ui, |ui| {
            for path in models {
                let current = model.file.as_str() == path;
                if ui.selectable_label(current, path).clicked() && !current {
                    chosen = AssetPath::new(path).ok();
                }
            }
        });
    ui.end_row();
    let Some(file) = chosen else {
        return false;
    };
    model.file = file;
    true
}

fn edit_bone(ui: &mut egui::Ui, bone: &mut Bone2dNode, keys: Option<&KeyButtons>) -> bool {
    let texts = texts::current();
    number_from_row(
        ui,
        keyed(keys, texts.bone_length(), &["length"]),
        &mut bone.length,
        MIN_SIZE,
    )
}

fn edit_camera_3d(ui: &mut egui::Ui, camera: &mut Camera3dNode, keys: Option<&KeyButtons>) -> bool {
    let texts = texts::current();
    angle_slider_row(
        ui,
        keyed(keys, texts.field_of_view, &["fov"]),
        &mut camera.fov,
        FIELD_OF_VIEW_DEGREES,
    ) | checkbox_hint_row(
        ui,
        texts.main_camera,
        &mut camera.main,
        texts.main_camera_hint,
    )
}

fn edit_light_2d(ui: &mut egui::Ui, light: &mut Light2dNode, keys: Option<&KeyButtons>) -> bool {
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
    changed |= light_rows(ui, color, intensity, keys);
    changed |= range_row(ui, range, keys);
    if let Light2dNode::Spot { angle, blend, .. } = light {
        changed |= spot_rows(ui, angle, blend, keys);
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

/// The rows of a 2D body; its shape is only shown when it follows the
/// sprite of its node.
fn edit_body(ui: &mut egui::Ui, body: &mut BodySettings, follows_sprite: bool) -> bool {
    let shaped = if follows_sprite {
        let texts = texts::current();
        ui.label(texts.shape);
        ui.weak(texts.shape_of_sprite());
        ui.end_row();
        false
    } else {
        edit_shape(ui, &mut body.shape)
    };
    shaped | edit_body_settings(ui, body)
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
    let mut offset = shape.offset();
    vec2_row(ui, texts.shape_offset, &mut offset);
    // The physics checks the sizes (a capsule at least as high as it is
    // wide); sizes it refuses leave the shape as it was.
    let rebuilt = match kind {
        ShapeKind::Circle { radius } => Shape2d::circle(radius),
        ShapeKind::Rectangle { size } => Shape2d::rectangle(size),
        ShapeKind::Capsule { height, radius } => Shape2d::capsule(height, radius),
    };
    replace_if_valid(
        shape,
        rebuilt.and_then(|shape| shape.with_offset(offset)).ok(),
    )
}

/// The kinds of 3D body shape, to choose one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Shape3dChoice {
    Sphere,
    Box,
    Capsule,
    Mesh,
}

/// The rows of a 3D body; with `fit`, what its node shows and the assets
/// folder, a button that fits its shape around it.
fn edit_body_3d(
    ui: &mut egui::Ui,
    body: &mut BodySettings<Shape3d>,
    fit: Option<(&Shown, &Path)>,
) -> bool {
    edit_shape_3d(ui, &mut body.shape)
        | fit_row(ui, &mut body.shape, fit)
        | edit_body_settings(ui, body)
}

/// A button that fits `shape` around what its node shows, when it is not
/// a mesh and `fit` says what that is.
fn fit_row(ui: &mut egui::Ui, shape: &mut Shape3d, fit: Option<(&Shown, &Path)>) -> bool {
    let Some((shown, folder)) = fit.filter(|_| shape.kind() != ShapeKind3d::Mesh) else {
        return false;
    };
    let texts = texts::current();
    ui.label("");
    let clicked = ui
        .button(texts.fit_shape)
        .on_hover_text(texts.fit_shape_hint)
        .clicked();
    ui.end_row();
    let Some(bounds) = clicked.then(|| shown.bounds(folder)).flatten() else {
        return false;
    };
    replace_if_valid(shape, Some(fitted(*shape, bounds)))
}

fn edit_shape_3d(ui: &mut egui::Ui, shape: &mut Shape3d) -> bool {
    let texts = texts::current();
    let mut kind = shape.kind();
    let mut choice = match kind {
        ShapeKind3d::Sphere { .. } => Shape3dChoice::Sphere,
        ShapeKind3d::Box { .. } => Shape3dChoice::Box,
        ShapeKind3d::Capsule { .. } => Shape3dChoice::Capsule,
        ShapeKind3d::Mesh => Shape3dChoice::Mesh,
    };
    let choices = [
        (Shape3dChoice::Sphere, texts.sphere),
        (Shape3dChoice::Box, texts.box_shape),
        (Shape3dChoice::Capsule, texts.capsule),
        (Shape3dChoice::Mesh, texts.mesh_shape),
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
            Shape3dChoice::Mesh => ShapeKind3d::Mesh,
        };
    }
    match &mut kind {
        // Its triangles are those of the node's mesh or model.
        ShapeKind3d::Mesh => {}
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
    let mut offset = shape.offset();
    vec3_row(ui, texts.shape_offset, &mut offset);
    let rebuilt = match kind {
        ShapeKind3d::Sphere { radius } => Shape3d::sphere(radius),
        ShapeKind3d::Box { size } => Shape3d::cuboid(size),
        ShapeKind3d::Capsule { height, radius } => Shape3d::capsule(height, radius),
        ShapeKind3d::Mesh => Ok(Shape3d::mesh()),
    };
    replace_if_valid(
        shape,
        rebuilt.and_then(|shape| shape.with_offset(offset)).ok(),
    )
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
