//! The parts a node can have, as the inspector adds them: each starts with
//! values that show something right away.

use eframe::egui;
use star_engine::prelude::{
    BodyKind, BodySettings, Color, LightNode, MeshShape, Shape2d, Shape3d, Sprite, TextureFilter,
    TextureRegion, Transform2D, Transform3D, Vec2, Vec3,
};
use star_engine::star_assets::AssetPath;
use star_engine::star_render::{DEFAULT_ROUGHNESS, DEFAULT_SPHERE_RINGS, DEFAULT_SPHERE_SEGMENTS};
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

/// What “Create” in the menus of the scene's tree adds: a node that shows
/// something right away, placed as the project's kind asks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NodeTemplate {
    Empty,
    Sprite,
    Cube,
    Sphere,
    Plane,
    DirectionalLight,
    PointLight,
}

impl NodeTemplate {
    /// The templates of a project of `dimension`, in the menu's order.
    pub(super) const fn of(dimension: ProjectDimension) -> &'static [Self] {
        match dimension {
            ProjectDimension::TwoD => &[Self::Empty, Self::Sprite],
            ProjectDimension::ThreeD => &[
                Self::Empty,
                Self::Cube,
                Self::Sphere,
                Self::Plane,
                Self::DirectionalLight,
                Self::PointLight,
            ],
        }
    }

    /// What the menu calls it.
    pub(super) fn label(self) -> &'static str {
        let texts = texts::current();
        match self {
            Self::Empty => texts.empty_node,
            Self::Sprite => texts.sprite_section,
            Self::Cube => texts.cube,
            Self::Sphere => texts.sphere,
            Self::Plane => texts.plane,
            Self::DirectionalLight => texts.directional_light,
            Self::PointLight => texts.point_light,
        }
    }

    /// The name its nodes start with, before a number makes it unique.
    pub(super) fn node_name(self) -> &'static str {
        match self {
            Self::Empty => texts::current().new_node_name,
            _ => self.label(),
        }
    }

    /// Whether it can be made: sprites need an image to show.
    pub(super) const fn is_available(self, images: &[String]) -> bool {
        !matches!(self, Self::Sprite) || !images.is_empty()
    }

    /// A node of it named `name`, for a project of `dimension`; sprites
    /// show the first of `images`.
    pub(super) fn build(
        self,
        name: String,
        dimension: ProjectDimension,
        images: &[String],
    ) -> NodeOutline {
        let mut node = new_node(name, dimension);
        let shape = match self {
            Self::Cube => Some(MeshShape::Cube { size: Vec3::ONE }),
            Self::Sphere => Some(MeshShape::Sphere {
                radius: DEFAULT_RADIUS,
                segments: DEFAULT_SPHERE_SEGMENTS,
                rings: DEFAULT_SPHERE_RINGS,
            }),
            Self::Plane => Some(MeshShape::Plane { size: Vec2::ONE }),
            _ => None,
        };
        node.mesh = shape.map(|shape| MeshOutline {
            shape,
            ..new_mesh()
        });
        match self {
            Self::Sprite => node.sprite = new_sprite(images),
            Self::DirectionalLight => node.light = Some(new_light(true)),
            Self::PointLight => node.light = Some(new_light(false)),
            _ => {}
        }
        node
    }
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
    fn templates_show_what_they_are_named_after() {
        let images = ["hero.png".to_owned()];
        let build = |template: NodeTemplate, dimension| {
            template.build("Thing".to_owned(), dimension, &images)
        };

        let sprite = build(NodeTemplate::Sprite, ProjectDimension::TwoD);
        let sphere = build(NodeTemplate::Sphere, ProjectDimension::ThreeD);
        let sun = build(NodeTemplate::DirectionalLight, ProjectDimension::ThreeD);

        assert!(sprite.sprite.is_some() && sprite.transform.is_some());
        assert!(matches!(
            sphere.mesh.map(|mesh| mesh.shape),
            Some(MeshShape::Sphere { .. })
        ));
        assert!(matches!(sun.light, Some(LightNode::Directional { .. })));
        assert!(!NodeTemplate::Sprite.is_available(&[]));
    }

    #[test]
    fn new_parts_are_valid() {
        assert!(new_body().is_some());
        assert!(new_body_3d().is_some());
    }
}
