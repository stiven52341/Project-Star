//! The parts a node can have, as the inspector adds them: each starts with
//! values that show something right away.

use eframe::egui;
use star_engine::prelude::{
    BodyKind, BodySettings, Color, LightNode, MeshShape, Shape2d, Shape3d, Sprite, TextureFilter,
    TextureRegion, Transform2D, Transform3D, Vec2, Vec3,
};
use star_engine::star_assets::AssetPath;
use star_engine::star_render::DEFAULT_ROUGHNESS;
use star_engine::star_scene::{MeshOutline, NodeOutline, SpriteOutline};

use crate::project::ProjectDimension;
use crate::texts;

/// Radius of new round shapes, so they are one unit wide.
pub(super) const DEFAULT_RADIUS: f32 = 0.5;
/// Height of new capsules: twice their width.
pub(super) const DEFAULT_CAPSULE_HEIGHT: f32 = 2.0;
/// How bright new suns are.
pub(super) const DEFAULT_SUN_INTENSITY: f32 = 1.0;
/// How bright new lamps are one unit away.
pub(super) const DEFAULT_LAMP_INTENSITY: f32 = 5.0;
/// How far new lamps reach.
pub(super) const DEFAULT_LAMP_RANGE: f32 = 10.0;

/// A menu of the parts `node` does not have yet, for the kind of project
/// it belongs to. Returns whether one was added.
pub(super) fn add_component_menu(
    ui: &mut egui::Ui,
    node: &mut NodeOutline,
    dimension: ProjectDimension,
    images: &[String],
) -> bool {
    let texts = texts::current();
    let mut added = false;
    ui.menu_button(texts.add_component, |ui| {
        let mut offer = |ui: &mut egui::Ui, missing: bool, name: &str, add: &mut dyn FnMut()| {
            if missing && ui.button(name).clicked() {
                add();
                added = true;
                ui.close();
            }
        };
        match dimension {
            ProjectDimension::TwoD => {
                offer(
                    ui,
                    node.transform.is_none(),
                    texts.transform_section,
                    &mut || {
                        node.transform = Some(Transform2D::IDENTITY);
                    },
                );
                if node.sprite.is_none() {
                    match new_sprite(images) {
                        Some(sprite) => offer(ui, true, texts.sprite_section, &mut || {
                            node.sprite = Some(sprite.clone());
                        }),
                        None => {
                            ui.add_enabled(false, egui::Button::new(texts.sprite_section))
                                .on_disabled_hover_text(texts.no_images_hint);
                        }
                    }
                }
                offer(ui, node.body.is_none(), texts.body_section, &mut || {
                    node.body = new_body();
                });
            }
            ProjectDimension::ThreeD => {
                offer(
                    ui,
                    node.transform_3d.is_none(),
                    texts.transform_3d_section,
                    &mut || node.transform_3d = Some(Transform3D::IDENTITY),
                );
                offer(ui, node.mesh.is_none(), texts.mesh_section, &mut || {
                    node.mesh = Some(new_mesh());
                });
                offer(ui, node.light.is_none(), texts.light_section, &mut || {
                    node.light = Some(new_light(false));
                });
                offer(
                    ui,
                    node.body_3d.is_none(),
                    texts.body_3d_section,
                    &mut || {
                        node.body_3d = new_body_3d();
                    },
                );
            }
        }
    });
    added
}

/// A node named `name` with the place in space of the project's kind.
pub(super) fn new_node(name: String, dimension: ProjectDimension) -> NodeOutline {
    let (transform, transform_3d) = match dimension {
        ProjectDimension::TwoD => (Some(Transform2D::IDENTITY), None),
        ProjectDimension::ThreeD => (None, Some(Transform3D::IDENTITY)),
    };
    NodeOutline {
        name,
        class: None,
        parent: None,
        transform,
        sprite: None,
        body: None,
        transform_3d,
        mesh: None,
        light: None,
        body_3d: None,
    }
}

/// A sprite one unit wide showing the first image, if there is one.
fn new_sprite(images: &[String]) -> Option<SpriteOutline> {
    let texture = images.first().and_then(|path| AssetPath::new(path).ok())?;
    Some(sprite_of(texture, Vec2::ONE))
}

/// A sprite of `size` showing `texture`, whole and as it is.
pub(super) const fn sprite_of(texture: AssetPath, size: Vec2) -> SpriteOutline {
    SpriteOutline {
        texture,
        filter: TextureFilter::Smooth,
        size,
        region: TextureRegion::FULL,
        anchor: Sprite::CENTER,
        color: Color::WHITE,
        layer: 0,
    }
}

fn new_body() -> Option<BodySettings> {
    Shape2d::rectangle(Vec2::ONE)
        .ok()
        .map(|shape| BodySettings::new(BodyKind::Dynamic, shape))
}

fn new_body_3d() -> Option<BodySettings<Shape3d>> {
    Shape3d::cuboid(Vec3::ONE)
        .ok()
        .map(|shape| BodySettings::new(BodyKind::Dynamic, shape))
}

pub(super) fn new_mesh() -> MeshOutline {
    MeshOutline {
        shape: MeshShape::Cube { size: Vec3::ONE },
        color: Color::WHITE,
        metallic: 0.0,
        roughness: DEFAULT_ROUGHNESS,
        texture: None,
    }
}

/// A white light: a sun if `directional`, a lamp if not.
pub(super) const fn new_light(directional: bool) -> LightNode {
    if directional {
        LightNode::Directional {
            color: Color::WHITE,
            intensity: DEFAULT_SUN_INTENSITY,
        }
    } else {
        LightNode::Point {
            color: Color::WHITE,
            intensity: DEFAULT_LAMP_INTENSITY,
            range: DEFAULT_LAMP_RANGE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_nodes_are_placed_in_the_projects_space() {
        let flat = new_node("A".to_owned(), ProjectDimension::TwoD);
        let deep = new_node("B".to_owned(), ProjectDimension::ThreeD);

        assert!(flat.transform.is_some() && flat.transform_3d.is_none());
        assert!(deep.transform_3d.is_some() && deep.transform.is_none());
    }

    #[test]
    fn new_sprites_show_the_first_image() {
        let images = ["sprites/hero.png".to_owned()];

        let sprite = new_sprite(&images).unwrap();

        assert_eq!(sprite.texture.as_str(), "sprites/hero.png");
        assert!(new_sprite(&[]).is_none());
    }

    #[test]
    fn new_parts_are_valid() {
        assert!(new_body().is_some());
        assert!(new_body_3d().is_some());
    }
}
