//! The parts a node can have, as the inspector adds them: each starts with
//! values that show something right away.

use eframe::egui;
use star_engine::prelude::{
    BodyKind, BodySettings, Color, LightNode, MeshShape, Shape2d, Shape3d, Sprite, TextureFilter,
    TextureRegion, Transform2D, Transform3D, Vec2, Vec3,
};
use star_engine::star_assets::AssetPath;
use star_engine::star_render::{
    DEFAULT_ROUGHNESS, DEFAULT_SPHERE_RINGS, DEFAULT_SPHERE_SEGMENTS, DEFAULT_SPOT_ANGLE,
    DEFAULT_SPOT_BLEND, SpriteSheet,
};
use star_engine::star_scene::{
    Camera2dNode, Camera3dNode, Light2dNode, MeshOutline, NodeOutline, SpriteOutline,
};

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
/// How far new 2D lights reach: a little more than the default view, two
/// units high.
pub(super) const DEFAULT_LAMP_RANGE_2D: f32 = 3.0;

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
                offer(
                    ui,
                    node.light_2d.is_none(),
                    texts.light_2d_section,
                    &mut || {
                        node.light_2d = Some(new_light_2d(LightKind::Point));
                    },
                );
                offer(ui, node.camera.is_none(), texts.camera_2d, &mut || {
                    node.camera = Some(Camera2dNode::default());
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
                    node.light = Some(new_light(LightKind::Point));
                });
                offer(
                    ui,
                    node.body_3d.is_none(),
                    texts.body_3d_section,
                    &mut || {
                        node.body_3d = new_body_3d();
                    },
                );
                offer(ui, node.camera_3d.is_none(), texts.camera_3d, &mut || {
                    node.camera_3d = Some(Camera3dNode::default());
                });
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
    SpotLight,
    PointLight2D,
    SpotLight2D,
    Camera2D,
    Camera3D,
    Body(BodyTemplate),
}

/// The bodies “Create” offers under “Physics”: each one a node with a body
/// of one unit, in the project's space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BodyTemplate {
    /// Never moves: floors, walls.
    Static,
    /// Moved by the simulation: falls, bounces, is pushed.
    Rigid,
    /// Moved only by the game: platforms, doors, characters.
    Kinematic,
    /// Detects what enters it, without stopping anything: a sensor.
    Area,
}

impl BodyTemplate {
    /// Every one, in the menu's order.
    pub(super) const ALL: [Self; 4] = [Self::Static, Self::Rigid, Self::Kinematic, Self::Area];

    fn label(self) -> &'static str {
        let texts = texts::current();
        match self {
            Self::Static => texts.static_body,
            Self::Rigid => texts.rigid_body,
            Self::Kinematic => texts.kinematic_body,
            Self::Area => texts.area,
        }
    }

    /// The settings of its body, around `shape`. Areas do not move: a
    /// static body detects all the others, and follows its node when the
    /// game moves it.
    fn settings<Shape>(self, shape: Shape) -> BodySettings<Shape> {
        let kind = match self {
            Self::Static | Self::Area => BodyKind::Static,
            Self::Rigid => BodyKind::Dynamic,
            Self::Kinematic => BodyKind::Kinematic,
        };
        let mut settings = BodySettings::new(kind, shape);
        settings.is_sensor = self == Self::Area;
        settings
    }
}

impl NodeTemplate {
    /// The templates of a project of `dimension`, in the menu's order.
    pub(super) const fn of(dimension: ProjectDimension) -> &'static [Self] {
        match dimension {
            ProjectDimension::TwoD => &[
                Self::Empty,
                Self::Sprite,
                Self::PointLight2D,
                Self::SpotLight2D,
                Self::Camera2D,
            ],
            ProjectDimension::ThreeD => &[
                Self::Empty,
                Self::Cube,
                Self::Sphere,
                Self::Plane,
                Self::DirectionalLight,
                Self::PointLight,
                Self::SpotLight,
                Self::Camera3D,
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
            Self::SpotLight => texts.spot_light,
            Self::PointLight2D => texts.point_light_2d,
            Self::SpotLight2D => texts.spot_light_2d,
            Self::Camera2D => texts.camera_2d,
            Self::Camera3D => texts.camera_3d,
            Self::Body(body) => body.label(),
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
    /// show the first of `images`. Cameras are `current`: the scene's
    /// tree makes them so only if no other camera is.
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
            Self::DirectionalLight => node.light = Some(new_light(LightKind::Directional)),
            Self::PointLight => node.light = Some(new_light(LightKind::Point)),
            Self::SpotLight => node.light = Some(new_light(LightKind::Spot)),
            Self::PointLight2D => node.light_2d = Some(new_light_2d(LightKind::Point)),
            Self::SpotLight2D => node.light_2d = Some(new_light_2d(LightKind::Spot)),
            Self::Camera2D => {
                node.camera = Some(Camera2dNode {
                    main: true,
                    ..Camera2dNode::default()
                });
            }
            Self::Camera3D => {
                node.camera_3d = Some(Camera3dNode {
                    main: true,
                    ..Camera3dNode::default()
                });
            }
            Self::Body(body) => match dimension {
                ProjectDimension::TwoD => {
                    node.body = Shape2d::rectangle(Vec2::ONE)
                        .ok()
                        .map(|shape| body.settings(shape));
                }
                ProjectDimension::ThreeD => {
                    node.body_3d = Shape3d::cuboid(Vec3::ONE)
                        .ok()
                        .map(|shape| body.settings(shape));
                }
            },
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
        tags: Vec::new(),
        visible: true,
        parent: None,
        transform,
        sprite: None,
        body: None,
        transform_3d,
        mesh: None,
        light: None,
        body_3d: None,
        camera: None,
        light_2d: None,
        camera_3d: None,
        animator: None,
        fields: std::collections::BTreeMap::new(),
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
        sheet: SpriteSheet::SINGLE,
        frames: None,
        animation: None,
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

/// The kinds of light a node can shine, to choose one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LightKind {
    Directional,
    Point,
    Spot,
}

impl LightKind {
    /// Every one, in the menu's order.
    pub(super) const ALL: [Self; 3] = [Self::Directional, Self::Point, Self::Spot];

    /// The kinds a 2D light can be, in the menu's order.
    pub(super) const ALL_2D: [Self; 2] = [Self::Point, Self::Spot];

    /// The kind of the 2D `light`.
    pub(super) const fn of_2d(light: &Light2dNode) -> Self {
        match light {
            Light2dNode::Point { .. } => Self::Point,
            Light2dNode::Spot { .. } => Self::Spot,
        }
    }

    /// The kind of `light`.
    pub(super) const fn of(light: &LightNode) -> Self {
        match light {
            LightNode::Directional { .. } => Self::Directional,
            LightNode::Point { .. } => Self::Point,
            LightNode::Spot { .. } => Self::Spot,
        }
    }
}

/// A white 2D light of `kind`: a lamp, or a spot if it is not one (2D
/// lights have no sun).
pub(super) const fn new_light_2d(kind: LightKind) -> Light2dNode {
    match kind {
        LightKind::Point | LightKind::Directional => Light2dNode::Point {
            color: Color::WHITE,
            intensity: 1.0,
            range: DEFAULT_LAMP_RANGE_2D,
        },
        LightKind::Spot => Light2dNode::Spot {
            color: Color::WHITE,
            intensity: 1.0,
            range: DEFAULT_LAMP_RANGE_2D,
            angle: DEFAULT_SPOT_ANGLE,
            blend: DEFAULT_SPOT_BLEND,
        },
    }
}

/// A white light of `kind`: a sun, a lamp or a spot that reaches as far as
/// a lamp.
pub(super) const fn new_light(kind: LightKind) -> LightNode {
    match kind {
        LightKind::Directional => LightNode::Directional {
            color: Color::WHITE,
            intensity: DEFAULT_SUN_INTENSITY,
        },
        LightKind::Point => LightNode::Point {
            color: Color::WHITE,
            intensity: DEFAULT_LAMP_INTENSITY,
            range: DEFAULT_LAMP_RANGE,
        },
        LightKind::Spot => LightNode::Spot {
            color: Color::WHITE,
            intensity: DEFAULT_LAMP_INTENSITY,
            range: DEFAULT_LAMP_RANGE,
            angle: DEFAULT_SPOT_ANGLE,
            blend: DEFAULT_SPOT_BLEND,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physics_templates_give_bodies_of_the_projects_space() {
        let build =
            |body, dimension| NodeTemplate::Body(body).build("Body".to_owned(), dimension, &[]);

        let floor = build(BodyTemplate::Static, ProjectDimension::TwoD);
        let crate_box = build(BodyTemplate::Rigid, ProjectDimension::ThreeD);
        let coin = build(BodyTemplate::Area, ProjectDimension::TwoD);
        let door = build(BodyTemplate::Kinematic, ProjectDimension::ThreeD);

        assert_eq!(floor.body.map(|body| body.kind), Some(BodyKind::Static));
        assert!(floor.body_3d.is_none());
        assert_eq!(
            crate_box.body_3d.map(|body| body.kind),
            Some(BodyKind::Dynamic)
        );
        assert!(crate_box.body.is_none());
        assert!(coin.body.is_some_and(|body| body.is_sensor));
        assert_eq!(
            door.body_3d.map(|body| body.kind),
            Some(BodyKind::Kinematic)
        );
        assert!(!door.body_3d.is_some_and(|body| body.is_sensor));
    }

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
