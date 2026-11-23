//! The inspector of a scene node: every part the file gives it, shown in
//! its own section.

use eframe::egui;
use star_engine::prelude::{BodySettings, Color, LightNode, Transform2D, Transform3D};
use star_engine::star_scene::{MeshOutline, NodeOutline, SpriteOutline};

use crate::style::{PROPERTY_SPACING, property_row};
use crate::texts::{self, number_text, size_text};

/// Shows the parts of `node`.
pub(super) fn show_node_properties(ui: &mut egui::Ui, node: &NodeOutline) {
    let texts = texts::current();
    section(ui, texts.node_section, |ui| {
        property_row(ui, texts.name, &node.name);
        property_row(
            ui,
            texts.class,
            node.class.as_deref().unwrap_or(texts.no_class),
        );
    });
    if let Some(transform) = &node.transform {
        section(ui, texts.transform_section, |ui| {
            show_transform(ui, transform);
        });
    }
    if let Some(transform) = &node.transform_3d {
        section(ui, texts.transform_3d_section, |ui| {
            show_transform_3d(ui, transform);
        });
    }
    if let Some(sprite) = &node.sprite {
        section(ui, texts.sprite_section, |ui| show_sprite(ui, sprite));
    }
    if let Some(mesh) = &node.mesh {
        section(ui, texts.mesh_section, |ui| show_mesh(ui, mesh));
    }
    if let Some(light) = &node.light {
        section(ui, texts.light_section, |ui| show_light(ui, light));
    }
    if let Some(body) = &node.body {
        section(ui, texts.body_section, |ui| {
            property_row(ui, texts.shape, &texts.shape_description(body.shape.kind()));
            show_body(ui, body);
        });
    }
    if let Some(body) = &node.body_3d {
        section(ui, texts.body_3d_section, |ui| {
            property_row(
                ui,
                texts.shape,
                &texts.shape_3d_description(body.shape.kind()),
            );
            show_body(ui, body);
        });
    }
}

/// A section that starts open, with its properties in a grid.
fn section(ui: &mut egui::Ui, title: &str, add_rows: impl FnOnce(&mut egui::Ui)) {
    egui::CollapsingHeader::new(title)
        .default_open(true)
        .show(ui, |ui| {
            egui::Grid::new(title)
                .num_columns(2)
                .spacing(PROPERTY_SPACING)
                .show(ui, add_rows);
        });
}

fn show_transform(ui: &mut egui::Ui, transform: &Transform2D) {
    let texts = texts::current();
    property_row(
        ui,
        texts.position,
        &coordinates(&[transform.position.x, transform.position.y]),
    );
    property_row(ui, texts.rotation, &degrees(transform.rotation));
    property_row(
        ui,
        texts.scale,
        &coordinates(&[transform.scale.x, transform.scale.y]),
    );
}

fn show_transform_3d(ui: &mut egui::Ui, transform: &Transform3D) {
    let texts = texts::current();
    let position = transform.position;
    let angles = transform.rotation.to_euler();
    let scale = transform.scale;
    property_row(
        ui,
        texts.position,
        &coordinates(&[position.x, position.y, position.z]),
    );
    property_row(
        ui,
        texts.rotation,
        &[angles.x, angles.y, angles.z].map(degrees).join(", "),
    );
    property_row(ui, texts.scale, &coordinates(&[scale.x, scale.y, scale.z]));
}

fn show_sprite(ui: &mut egui::Ui, sprite: &SpriteOutline) {
    let texts = texts::current();
    property_row(ui, texts.image, sprite.texture.as_str());
    property_row(ui, texts.filter, texts.filter_name(sprite.filter));
    property_row(ui, texts.size, &size_text(&[sprite.size.x, sprite.size.y]));
    property_row(
        ui,
        texts.anchor,
        &coordinates(&[sprite.anchor.x, sprite.anchor.y]),
    );
    let region = sprite.region;
    property_row(
        ui,
        texts.region,
        &coordinates(&[region.min.x, region.min.y, region.max.x, region.max.y]),
    );
    property_row(ui, texts.color, &color_text(sprite.color));
    property_row(ui, texts.layer, &sprite.layer.to_string());
}

fn show_mesh(ui: &mut egui::Ui, mesh: &MeshOutline) {
    let texts = texts::current();
    property_row(ui, texts.shape, &texts.mesh_description(mesh.shape));
    property_row(ui, texts.color, &color_text(mesh.color));
    property_row(ui, texts.metallic, &number_text(mesh.metallic));
    property_row(ui, texts.roughness, &number_text(mesh.roughness));
    if let Some((image, filter)) = &mesh.texture {
        property_row(ui, texts.image, image.as_str());
        property_row(ui, texts.filter, texts.filter_name(*filter));
    }
}

fn show_light(ui: &mut egui::Ui, light: &LightNode) {
    let texts = texts::current();
    property_row(ui, texts.body_kind, texts.light_kind_name(light));
    let (LightNode::Directional { color, intensity }
    | LightNode::Point {
        color, intensity, ..
    }) = *light;
    property_row(ui, texts.color, &color_text(color));
    property_row(ui, texts.intensity, &number_text(intensity));
    if let LightNode::Point { range, .. } = *light {
        property_row(ui, texts.range, &number_text(range));
    }
}

/// What 2D and 3D bodies share: everything but their shape.
fn show_body<Shape>(ui: &mut egui::Ui, body: &BodySettings<Shape>) {
    let texts = texts::current();
    property_row(ui, texts.body_kind, texts.body_kind_name(body.kind));
    property_row(ui, texts.gravity_scale, &number_text(body.gravity_scale));
    property_row(ui, texts.friction, &number_text(body.friction));
    property_row(ui, texts.bounciness, &number_text(body.bounciness));
    property_row(ui, texts.density, &number_text(body.density));
    property_row(ui, texts.sensor, yes_or_no(body.is_sensor));
    property_row(ui, texts.fixed_rotation, yes_or_no(body.fixed_rotation));
}

fn coordinates(values: &[f32]) -> String {
    values
        .iter()
        .map(|&value| number_text(value))
        .collect::<Vec<_>>()
        .join(", ")
}

/// An angle in radians, shown in degrees, which people read more easily.
fn degrees(radians: f32) -> String {
    format!("{}°", number_text(radians.to_degrees()))
}

fn color_text(color: Color) -> String {
    coordinates(&[color.r, color.g, color.b, color.a])
}

fn yes_or_no(value: bool) -> &'static str {
    if value {
        texts::current().yes
    } else {
        texts::current().no
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn angles_are_shown_in_degrees() {
        assert_eq!(degrees(std::f32::consts::FRAC_PI_2), "90°");
    }

    #[test]
    fn coordinates_are_separated_by_commas() {
        assert_eq!(coordinates(&[1.0, -0.5]), "1, -0.5");
    }
}
