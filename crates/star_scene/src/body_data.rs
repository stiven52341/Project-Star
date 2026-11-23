//! The `[node.body]` table of `.scene` files: a node's physics body.

use serde::{Deserialize, Serialize};
use star_core::{EngineError, EngineResult};
use star_math::Vec2;
use star_physics::{BodyKind, BodySettings, Shape2d, ShapeKind};

/// A `[node.body]` table. Only the sizes of its shape are written; the
/// surface values appear only when they are not the defaults.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct BodyData {
    #[serde(default)]
    pub(crate) kind: KindData,
    pub(crate) shape: ShapeData,
    /// Of circles and capsules.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) radius: Option<f32>,
    /// Of rectangles: width and height.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) size: Option<[f32; 2]>,
    /// Of capsules, half circles included.
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

/// The shape of a body, as written in files; its sizes are fields of the
/// table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ShapeData {
    Circle,
    Rectangle,
    Capsule,
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "serde passes the field by reference"
)]
fn is_false(value: &bool) -> bool {
    !*value
}

impl BodyData {
    /// The body the table describes.
    ///
    /// # Errors
    ///
    /// [`EngineError::Scene`] if the shape lacks a size, has a size of
    /// another shape, or a size is not valid.
    pub(crate) fn to_settings(&self) -> EngineResult<BodySettings> {
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
    pub(crate) fn from_settings(settings: &BodySettings) -> Self {
        let defaults = BodySettings::new(settings.kind, settings.shape);
        #[expect(
            clippy::float_cmp,
            reason = "only values exactly equal to the default are left out"
        )]
        let differs = |value: f32, default: f32| (value != default).then_some(value);
        let (shape, radius, size, height) = match settings.shape.kind() {
            ShapeKind::Circle { radius } => (ShapeData::Circle, Some(radius), None, None),
            ShapeKind::Rectangle { size } => {
                (ShapeData::Rectangle, None, Some([size.x, size.y]), None)
            }
            ShapeKind::Capsule { height, radius } => {
                (ShapeData::Capsule, Some(radius), None, Some(height))
            }
        };
        Self {
            kind: settings.kind.into(),
            shape,
            radius,
            size,
            height,
            gravity_scale: differs(settings.gravity_scale, defaults.gravity_scale),
            fixed_rotation: settings.fixed_rotation,
            friction: differs(settings.friction, defaults.friction),
            bounciness: differs(settings.bounciness, defaults.bounciness),
            density: differs(settings.density, defaults.density),
            sensor: settings.is_sensor,
        }
    }

    fn shape(&self) -> EngineResult<Shape2d> {
        let name = match self.shape {
            ShapeData::Circle => "circle",
            ShapeData::Rectangle => "rectangle",
            ShapeData::Capsule => "capsule",
        };
        let wants = |field: &str, wanted: bool, given: bool| match (wanted, given) {
            (true, false) => Err(EngineError::Scene(format!("a {name} body needs a {field}"))),
            (false, true) => Err(EngineError::Scene(format!("a {name} body has no {field}"))),
            _ => Ok(()),
        };
        let is_circle = self.shape == ShapeData::Circle;
        let is_rectangle = self.shape == ShapeData::Rectangle;
        let is_capsule = self.shape == ShapeData::Capsule;
        wants("radius", is_circle || is_capsule, self.radius.is_some())?;
        wants("size", is_rectangle, self.size.is_some())?;
        wants("height", is_capsule, self.height.is_some())?;
        let shape = match (self.radius, self.size, self.height) {
            (Some(radius), None, None) => Shape2d::circle(radius),
            (None, Some([width, height]), None) => Shape2d::rectangle(Vec2::new(width, height)),
            (Some(radius), None, Some(height)) => Shape2d::capsule(height, radius),
            _ => Err(EngineError::Scene(format!("a {name} body has no size"))),
        };
        shape.map_err(|error| EngineError::Scene(error.to_string()))
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

    fn parse(text: &str) -> EngineResult<BodySettings> {
        let data: BodyData =
            toml::from_str(text).map_err(|error| EngineError::Scene(error.to_string()))?;
        data.to_settings()
    }

    #[test]
    fn tables_give_bodies_with_defaults() {
        let settings = parse("shape = \"circle\"\nradius = 0.5").unwrap();

        assert_eq!(settings.kind, BodyKind::Dynamic);
        assert_eq!(
            settings,
            BodySettings::new(BodyKind::Dynamic, Shape2d::circle(0.5).unwrap())
        );
    }

    #[test]
    fn every_field_is_read() {
        let settings = parse(
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
        ] {
            assert!(parse(text).is_err(), "{text}");
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
}
