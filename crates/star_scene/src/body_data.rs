//! The `[node.body]` and `[node.body_3d]` tables of `.scene` files: a
//! node's body in the 2D or 3D physics.

use std::fmt::Debug;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use star_core::{EngineError, EngineResult};
use star_math::{Vec2, Vec3};
use star_physics::{BodyKind, BodySettings, Shape2d, Shape3d, ShapeKind, ShapeKind3d};

/// A `[node.body]` table: a body of the 2D physics.
pub(crate) type BodyData = BodyTable<Shape2d>;

/// A `[node.body_3d]` table: a body of the 3D physics.
pub(crate) type Body3dData = BodyTable<Shape3d>;

/// A body table. Only the sizes of its shape are written; the surface
/// values appear only when they are not the defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, bound = "")]
pub(crate) struct BodyTable<Shape: TableShape> {
    #[serde(default)]
    pub(crate) kind: KindData,
    pub(crate) shape: Shape::Name,
    /// Of circles, spheres and capsules.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) radius: Option<f32>,
    /// Of rectangles (width and height) and boxes (and depth).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) size: Option<Shape::Size>,
    /// Of capsules, round ends included.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) height: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) gravity_scale: Option<f32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) fixed_rotation: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) friction: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) bounciness: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) density: Option<f32>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub(crate) sensor: bool,
}

/// How a body moves, as written in files.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum KindData {
    #[default]
    Dynamic,
    Static,
    Kinematic,
}

/// The sizes a shape is written with in a body table.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ShapeSizes<Size> {
    radius: Option<f32>,
    size: Option<Size>,
    height: Option<f32>,
}

/// A shape of bodies, as written in body tables: its name, and its sizes
/// as fields of the table.
pub(crate) trait TableShape: Copy + Debug + PartialEq {
    /// The names of the shapes, as written in files.
    type Name: Copy + Debug + PartialEq + Serialize + DeserializeOwned;
    /// The `size` field.
    type Size: Copy + Debug + PartialEq + Serialize + DeserializeOwned;

    /// The name of `name` in errors.
    fn label(name: Self::Name) -> &'static str;

    /// Whether `name` takes a radius, a size and a height.
    fn takes(name: Self::Name) -> [bool; 3];

    /// The shape of `sizes`, which are those `takes` asks for.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`] if a size is not valid.
    fn build(sizes: ShapeSizes<Self::Size>) -> EngineResult<Self>;

    /// The name and sizes the shape is written with.
    fn describe(self) -> (Self::Name, ShapeSizes<Self::Size>);
}

/// The shape of a 2D body, as written in files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ShapeData {
    Circle,
    Rectangle,
    Capsule,
}

/// The shape of a 3D body, as written in files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Shape3dData {
    Sphere,
    Box,
    Capsule,
    /// The triangles of the node's mesh or model, which take no sizes.
    Mesh,
}

/// Which of radius, size and height a shape takes.
const TAKES_RADIUS: [bool; 3] = [true, false, false];
const TAKES_SIZE: [bool; 3] = [false, true, false];
const TAKES_RADIUS_AND_HEIGHT: [bool; 3] = [true, false, true];
const TAKES_NOTHING: [bool; 3] = [false, false, false];

impl TableShape for Shape2d {
    type Name = ShapeData;
    type Size = [f32; 2];

    fn label(name: ShapeData) -> &'static str {
        match name {
            ShapeData::Circle => "circle",
            ShapeData::Rectangle => "rectangle",
            ShapeData::Capsule => "capsule",
        }
    }

    fn takes(name: ShapeData) -> [bool; 3] {
        match name {
            ShapeData::Circle => TAKES_RADIUS,
            ShapeData::Rectangle => TAKES_SIZE,
            ShapeData::Capsule => TAKES_RADIUS_AND_HEIGHT,
        }
    }

    fn build(sizes: ShapeSizes<[f32; 2]>) -> EngineResult<Self> {
        match (sizes.radius, sizes.size, sizes.height) {
            (Some(radius), None, None) => Self::circle(radius),
            (None, Some([width, height]), None) => Self::rectangle(Vec2::new(width, height)),
            (Some(radius), None, Some(height)) => Self::capsule(height, radius),
            _ => Err(EngineError::Physics("a body has no size".to_owned())),
        }
    }

    fn describe(self) -> (ShapeData, ShapeSizes<[f32; 2]>) {
        let (name, radius, size, height) = match self.kind() {
            ShapeKind::Circle { radius } => (ShapeData::Circle, Some(radius), None, None),
            ShapeKind::Rectangle { size } => {
                (ShapeData::Rectangle, None, Some([size.x, size.y]), None)
            }
            ShapeKind::Capsule { height, radius } => {
                (ShapeData::Capsule, Some(radius), None, Some(height))
            }
        };
        (
            name,
            ShapeSizes {
                radius,
                size,
                height,
            },
        )
    }
}

impl TableShape for Shape3d {
    type Name = Shape3dData;
    type Size = [f32; 3];

    fn label(name: Shape3dData) -> &'static str {
        match name {
            Shape3dData::Sphere => "sphere",
            Shape3dData::Box => "box",
            Shape3dData::Capsule => "capsule",
            Shape3dData::Mesh => "mesh",
        }
    }

    fn takes(name: Shape3dData) -> [bool; 3] {
        match name {
            Shape3dData::Sphere => TAKES_RADIUS,
            Shape3dData::Box => TAKES_SIZE,
            Shape3dData::Capsule => TAKES_RADIUS_AND_HEIGHT,
            Shape3dData::Mesh => TAKES_NOTHING,
        }
    }

    fn build(sizes: ShapeSizes<[f32; 3]>) -> EngineResult<Self> {
        match (sizes.radius, sizes.size, sizes.height) {
            (Some(radius), None, None) => Self::sphere(radius),
            (None, Some([width, height, depth]), None) => {
                Self::cuboid(Vec3::new(width, height, depth))
            }
            (Some(radius), None, Some(height)) => Self::capsule(height, radius),
            // Only a mesh takes no sizes.
            (None, None, None) => Ok(Self::mesh()),
            _ => Err(EngineError::Physics("a body has no size".to_owned())),
        }
    }

    fn describe(self) -> (Shape3dData, ShapeSizes<[f32; 3]>) {
        let (name, radius, size, height) = match self.kind() {
            ShapeKind3d::Sphere { radius } => (Shape3dData::Sphere, Some(radius), None, None),
            ShapeKind3d::Box { size } => {
                (Shape3dData::Box, None, Some([size.x, size.y, size.z]), None)
            }
            ShapeKind3d::Capsule { height, radius } => {
                (Shape3dData::Capsule, Some(radius), None, Some(height))
            }
            ShapeKind3d::Mesh => (Shape3dData::Mesh, None, None, None),
        };
        (
            name,
            ShapeSizes {
                radius,
                size,
                height,
            },
        )
    }
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde passes the field by reference"
)]
fn is_false(value: &bool) -> bool {
    !*value
}

impl<Shape: TableShape> BodyTable<Shape> {
    /// The body the table describes.
    ///
    /// # Errors
    ///
    /// [`EngineError::Scene`] if the shape lacks a size, has a size of
    /// another shape, or a size is not valid.
    pub(crate) fn to_settings(&self) -> EngineResult<BodySettings<Shape>> {
        let defaults = BodySettings::new(self.kind.into(), self.shape()?);
        Ok(BodySettings {
            gravity_scale: self.gravity_scale.unwrap_or(defaults.gravity_scale),
            fixed_rotation: self.fixed_rotation,
            friction: self.friction.unwrap_or(defaults.friction),
            bounciness: self.bounciness.unwrap_or(defaults.bounciness),
            density: self.density.unwrap_or(defaults.density),
            is_sensor: self.sensor,
            ..defaults
        })
    }

    /// The table that describes `settings`.
    pub(crate) fn from_settings(settings: &BodySettings<Shape>) -> Self {
        let defaults = BodySettings::new(settings.kind, settings.shape);
        #[expect(
            clippy::float_cmp,
            reason = "only values exactly equal to the default are left out"
        )]
        let differs = |value: f32, default: f32| (value != default).then_some(value);
        let (shape, sizes) = settings.shape.describe();
        Self {
            kind: settings.kind.into(),
            shape,
            radius: sizes.radius,
            size: sizes.size,
            height: sizes.height,
            gravity_scale: differs(settings.gravity_scale, defaults.gravity_scale),
            fixed_rotation: settings.fixed_rotation,
            friction: differs(settings.friction, defaults.friction),
            bounciness: differs(settings.bounciness, defaults.bounciness),
            density: differs(settings.density, defaults.density),
            sensor: settings.is_sensor,
        }
    }

    fn shape(&self) -> EngineResult<Shape> {
        let name = Shape::label(self.shape);
        let [radius, size, height] = Shape::takes(self.shape);
        let wants = |field: &str, wanted: bool, given: bool| match (wanted, given) {
            (true, false) => Err(EngineError::Scene(format!("a {name} body needs a {field}"))),
            (false, true) => Err(EngineError::Scene(format!("a {name} body has no {field}"))),
            _ => Ok(()),
        };
        wants("radius", radius, self.radius.is_some())?;
        wants("size", size, self.size.is_some())?;
        wants("height", height, self.height.is_some())?;
        Shape::build(ShapeSizes {
            radius: self.radius,
            size: self.size,
            height: self.height,
        })
        .map_err(|error| EngineError::Scene(error.to_string()))
    }
}

impl From<KindData> for BodyKind {
    fn from(kind: KindData) -> Self {
        match kind {
            KindData::Dynamic => Self::Dynamic,
            KindData::Static => Self::Static,
            KindData::Kinematic => Self::Kinematic,
        }
    }
}

impl From<BodyKind> for KindData {
    fn from(kind: BodyKind) -> Self {
        match kind {
            BodyKind::Dynamic => Self::Dynamic,
            BodyKind::Static => Self::Static,
            BodyKind::Kinematic => Self::Kinematic,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse<Shape: TableShape>(text: &str) -> EngineResult<BodySettings<Shape>> {
        let data: BodyTable<Shape> =
            toml::from_str(text).map_err(|error| EngineError::Scene(error.to_string()))?;
        data.to_settings()
    }

    #[test]
    fn tables_give_bodies_with_defaults() {
        let settings = parse::<Shape2d>("shape = \"circle\"\nradius = 0.5").unwrap();

        assert_eq!(settings.kind, BodyKind::Dynamic);
        assert_eq!(
            settings,
            BodySettings::new(BodyKind::Dynamic, Shape2d::circle(0.5).unwrap())
        );
    }

    #[test]
    fn every_field_is_read() {
        let settings = parse::<Shape2d>(
            "kind = \"kinematic\"\nshape = \"capsule\"\nradius = 0.5\nheight = 2.0\n\
             gravity_scale = 0.0\nfixed_rotation = true\nfriction = 0.1\n\
             bounciness = 0.8\ndensity = 3.0\nsensor = true",
        )
        .unwrap();

        assert_eq!(settings.kind, BodyKind::Kinematic);
        assert_eq!(settings.shape, Shape2d::capsule(2.0, 0.5).unwrap());
        assert!(settings.fixed_rotation && settings.is_sensor);
        assert_eq!(
            (
                settings.gravity_scale,
                settings.friction,
                settings.bounciness,
                settings.density
            ),
            (0.0, 0.1, 0.8, 3.0)
        );
    }

    #[test]
    fn shapes_need_their_own_sizes_only() {
        for text in [
            "shape = \"circle\"",
            "shape = \"circle\"\nradius = 1.0\nsize = [1.0, 1.0]",
            "shape = \"rectangle\"\nradius = 1.0",
            "shape = \"capsule\"\nradius = 1.0",
            "shape = \"rectangle\"\nsize = [0.0, 1.0]",
            "shape = \"triangle\"\nradius = 1.0",
            "shape = \"sphere\"\nradius = 1.0",
        ] {
            assert!(parse::<Shape2d>(text).is_err(), "{text}");
        }
    }

    #[test]
    fn written_tables_read_back_the_same() {
        let mut settings = BodySettings::new(
            BodyKind::Static,
            Shape2d::rectangle(Vec2::new(4.0, 1.0)).unwrap(),
        );
        settings.bounciness = 0.5;
        let data = BodyData::from_settings(&settings);

        let text = toml::to_string(&data).unwrap();

        assert_eq!(parse(&text).unwrap(), settings);
        assert!(!text.contains("friction"), "{text}");
    }

    #[test]
    fn tables_give_3d_bodies() {
        let ball = parse::<Shape3d>("shape = \"sphere\"\nradius = 0.5\nbounciness = 0.7").unwrap();
        let crate_box =
            parse::<Shape3d>("kind = \"static\"\nshape = \"box\"\nsize = [1.0, 2.0, 3.0]").unwrap();
        let capsule = parse::<Shape3d>("shape = \"capsule\"\nradius = 0.5\nheight = 2.0").unwrap();

        assert_eq!(ball.shape, Shape3d::sphere(0.5).unwrap());
        assert!((ball.bounciness - 0.7).abs() < f32::EPSILON);
        assert_eq!(crate_box.kind, BodyKind::Static);
        assert_eq!(
            crate_box.shape,
            Shape3d::cuboid(Vec3::new(1.0, 2.0, 3.0)).unwrap()
        );
        assert_eq!(capsule.shape, Shape3d::capsule(2.0, 0.5).unwrap());
        let ground = parse::<Shape3d>("kind = \"static\"\nshape = \"mesh\"").unwrap();
        assert_eq!(ground.shape, Shape3d::mesh());
        let written = toml::to_string(&Body3dData::from_settings(&ground)).unwrap();
        assert_eq!(written, "kind = \"static\"\nshape = \"mesh\"\n");
    }

    #[test]
    fn shapes_3d_need_their_own_sizes_only() {
        for text in [
            "shape = \"sphere\"",
            "shape = \"box\"\nsize = [1.0, 1.0]",
            "shape = \"box\"\nsize = [1.0, 1.0, -1.0]",
            "shape = \"circle\"\nradius = 1.0",
            "shape = \"capsule\"\nheight = 1.0",
            "shape = \"mesh\"\nradius = 1.0",
        ] {
            assert!(parse::<Shape3d>(text).is_err(), "{text}");
        }
    }

    #[test]
    fn written_3d_tables_read_back_the_same() {
        let mut settings = BodySettings::new(
            BodyKind::Kinematic,
            Shape3d::cuboid(Vec3::new(4.0, 1.0, 2.0)).unwrap(),
        );
        settings.friction = 0.1;
        let data = Body3dData::from_settings(&settings);

        let text = toml::to_string(&data).unwrap();

        assert_eq!(parse(&text).unwrap(), settings);
        assert!(text.contains("shape = \"box\""), "{text}");
    }
}
