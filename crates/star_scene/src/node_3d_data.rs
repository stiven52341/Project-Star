//! The tables of 3D nodes in `.scene` files: `[node.transform_3d]`,
//! `[node.mesh]` and `[node.light]`.

use serde::{Deserialize, Serialize};
use star_assets::AssetPath;
use star_core::{EngineError, EngineResult};
use star_math::{Quat, Vec2, Vec3};
use star_render::{
    Color, DEFAULT_ROUGHNESS, DEFAULT_SPHERE_RINGS, DEFAULT_SPHERE_SEGMENTS, DEFAULT_SPOT_ANGLE,
    DEFAULT_SPOT_BLEND, MAX_SPOT_ANGLE, MeshShape, TextureFilter,
};

use crate::scene_file::FilterData;
use crate::{LightNode, Transform3D};

/// A `[node.transform_3d]` table; what it leaves out takes its default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Transform3dData {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) position: Option<[f32; 3]>,
    /// Pitch, yaw and roll in radians, as [`Quat::from_euler`] takes them.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) rotation: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) scale: Option<[f32; 3]>,
}

impl Transform3dData {
    pub(crate) fn to_transform(&self) -> Transform3D {
        Transform3D {
            position: self.position.map_or(Vec3::ZERO, vec3),
            rotation: self
                .rotation
                .map_or(Quat::IDENTITY, |angles| Quat::from_euler(vec3(angles))),
            scale: self.scale.map_or(Vec3::ONE, vec3),
        }
    }

    pub(crate) fn from_transform(transform: &Transform3D) -> Self {
        let rotation = transform.rotation.to_euler();
        Self {
            position: Some(array3(transform.position)),
            rotation: (transform.rotation != Quat::IDENTITY).then(|| array3(rotation)),
            scale: (transform.scale != Vec3::ONE).then(|| array3(transform.scale)),
        }
    }
}

/// A `[node.mesh]` table: the shape of a mesh and its material.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MeshTable {
    pub(crate) shape: ShapeData,
    /// Of cubes (width, height, depth) and planes (width, depth).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) size: Option<Vec<f32>>,
    /// Of spheres.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) radius: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) segments: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) rings: Option<u32>,
    /// Red, green, blue and alpha, from 0 to 1, in sRGB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) color: Option<[f32; 4]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) metallic: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) roughness: Option<f32>,
    /// Asset path of the image multiplied by the color.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) texture: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) filter: Option<FilterData>,
}

/// The shape of a mesh, as written in files; its sizes are fields of the
/// table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum ShapeData {
    Cube,
    Plane,
    Sphere,
}

/// A mesh of a scene, with its material, and its image named by path.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshOutline {
    /// The shape of the mesh.
    pub shape: MeshShape,
    /// The color of the surface, in sRGB.
    pub color: Color,
    /// How metallic the surface is, from 0 to 1.
    pub metallic: f32,
    /// How rough the surface is, from 0 to 1.
    pub roughness: f32,
    /// The image multiplied by the color, and how it is sampled.
    pub texture: Option<(AssetPath, TextureFilter)>,
}

impl MeshTable {
    pub(crate) fn to_outline(&self) -> EngineResult<MeshOutline> {
        let shape = self.shape()?;
        shape.to_mesh_data()?;
        let texture = self
            .texture
            .as_deref()
            .map(|path| {
                let filter = self.filter.unwrap_or_default().into();
                AssetPath::new(path).map(|path| (path, filter))
            })
            .transpose()?;
        Ok(MeshOutline {
            shape,
            color: self
                .color
                .map_or(Color::WHITE, |[r, g, b, a]| Color::rgba(r, g, b, a)),
            metallic: self.metallic.unwrap_or_default(),
            roughness: self.roughness.unwrap_or(DEFAULT_ROUGHNESS),
            texture,
        })
    }

    /// The table that describes `outline`; the material's values appear
    /// only when they are not the defaults.
    pub(crate) fn from_outline(outline: &MeshOutline) -> Self {
        let (shape, size, radius, segments, rings) = match outline.shape {
            MeshShape::Cube { size } => (
                ShapeData::Cube,
                Some(vec![size.x, size.y, size.z]),
                None,
                None,
                None,
            ),
            MeshShape::Plane { size } => (
                ShapeData::Plane,
                Some(vec![size.x, size.y]),
                None,
                None,
                None,
            ),
            MeshShape::Sphere {
                radius,
                segments,
                rings,
            } => (
                ShapeData::Sphere,
                None,
                Some(radius),
                (segments != DEFAULT_SPHERE_SEGMENTS).then_some(segments),
                (rings != DEFAULT_SPHERE_RINGS).then_some(rings),
            ),
        };
        let Color { r, g, b, a } = outline.color;
        #[expect(
            clippy::float_cmp,
            reason = "only values exactly equal to the default are left out"
        )]
        let differs = |value: f32, default: f32| (value != default).then_some(value);
        Self {
            shape,
            size,
            radius,
            segments,
            rings,
            color: (outline.color != Color::WHITE).then_some([r, g, b, a]),
            metallic: differs(outline.metallic, 0.0),
            roughness: differs(outline.roughness, DEFAULT_ROUGHNESS),
            texture: outline
                .texture
                .as_ref()
                .map(|(path, _)| path.as_str().to_owned()),
            filter: outline
                .texture
                .as_ref()
                .map(|(_, filter)| (*filter).into())
                .filter(|filter| *filter != FilterData::default()),
        }
    }

    fn shape(&self) -> EngineResult<MeshShape> {
        let shape_error = |reason: &str| {
            Err(EngineError::Scene(
                format!("a {:?} mesh {reason}", self.shape).to_lowercase(),
            ))
        };
        let has_sphere_fields =
            self.radius.is_some() || self.segments.is_some() || self.rings.is_some();
        match (self.shape, self.size.as_deref()) {
            (ShapeData::Cube, Some(&[x, y, z])) if !has_sphere_fields => Ok(MeshShape::Cube {
                size: Vec3::new(x, y, z),
            }),
            (ShapeData::Plane, Some(&[x, z])) if !has_sphere_fields => Ok(MeshShape::Plane {
                size: Vec2::new(x, z),
            }),
            (ShapeData::Sphere, None) => match self.radius {
                Some(radius) => Ok(MeshShape::Sphere {
                    radius,
                    segments: self.segments.unwrap_or(DEFAULT_SPHERE_SEGMENTS),
                    rings: self.rings.unwrap_or(DEFAULT_SPHERE_RINGS),
                }),
                None => shape_error("needs a radius"),
            },
            (ShapeData::Cube, _) => {
                shape_error("needs a size of width, height and depth, and nothing else")
            }
            (ShapeData::Plane, _) => {
                shape_error("needs a size of width and depth, and nothing else")
            }
            (ShapeData::Sphere, Some(_)) => shape_error("has a radius, not a size"),
        }
    }
}

/// A `[node.light]` table.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LightData {
    pub(crate) kind: LightKindData,
    /// Red, green and blue, from 0 to 1, in sRGB.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) color: Option<[f32; 3]>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) intensity: Option<f32>,
    /// Of point lights and spots.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) range: Option<f32>,
    /// Of spots: from the cone's axis to its edge, in radians.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) angle: Option<f32>,
    /// Of spots: how much of the cone fades, from 0 to 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) blend: Option<f32>,
}

/// What a light is, as written in files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LightKindData {
    Directional,
    Point,
    Spot,
}

/// The intensity of a light that gives none.
const DEFAULT_INTENSITY: f32 = 1.0;

impl LightData {
    pub(crate) fn to_light(&self) -> EngineResult<LightNode> {
        let color = self
            .color
            .map_or(Color::WHITE, |[r, g, b]| Color::rgb(r, g, b));
        let intensity = self.intensity.unwrap_or(DEFAULT_INTENSITY);
        let is_spot = self.kind == LightKindData::Spot;
        if !is_spot && (self.angle.is_some() || self.blend.is_some()) {
            return Err(scene_error("only spot lights have an angle and a blend"));
        }
        if self.kind == LightKindData::Directional {
            return match self.range {
                None => Ok(LightNode::Directional { color, intensity }),
                Some(_) => Err(scene_error(
                    "a directional light reaches everything: it has no range",
                )),
            };
        }
        let range = self
            .range
            .filter(|range| range.is_finite() && *range > 0.0)
            .ok_or_else(|| scene_error("a point or spot light needs a positive range"))?;
        if !is_spot {
            return Ok(LightNode::Point {
                color,
                intensity,
                range,
            });
        }
        let angle = self.angle.unwrap_or(DEFAULT_SPOT_ANGLE);
        let blend = self.blend.unwrap_or(DEFAULT_SPOT_BLEND);
        if !(angle > 0.0 && angle < MAX_SPOT_ANGLE) {
            return Err(EngineError::Scene(format!(
                "a spot light's angle is more than 0 and less than {MAX_SPOT_ANGLE} radians, \
                 not {angle}"
            )));
        }
        if !(0.0..=1.0).contains(&blend) {
            return Err(EngineError::Scene(format!(
                "a spot light's blend goes from 0 to 1, not {blend}"
            )));
        }
        Ok(LightNode::Spot {
            color,
            intensity,
            range,
            angle,
            blend,
        })
    }

    pub(crate) fn from_light(light: &LightNode) -> Self {
        let (kind, color, intensity, range, cone) = match *light {
            LightNode::Directional { color, intensity } => {
                (LightKindData::Directional, color, intensity, None, None)
            }
            LightNode::Point {
                color,
                intensity,
                range,
            } => (LightKindData::Point, color, intensity, Some(range), None),
            LightNode::Spot {
                color,
                intensity,
                range,
                angle,
                blend,
            } => (
                LightKindData::Spot,
                color,
                intensity,
                Some(range),
                Some((angle, blend)),
            ),
        };
        let intensity = (intensity != DEFAULT_INTENSITY).then_some(intensity);
        Self {
            kind,
            color: (color != Color::WHITE).then_some([color.r, color.g, color.b]),
            intensity,
            range,
            angle: cone.map(|(angle, _)| angle),
            blend: cone.map(|(_, blend)| blend),
        }
    }
}

fn scene_error(reason: &str) -> EngineError {
    EngineError::Scene(reason.to_owned())
}

const fn vec3([x, y, z]: [f32; 3]) -> Vec3 {
    Vec3::new(x, y, z)
}

const fn array3(vector: Vec3) -> [f32; 3] {
    [vector.x, vector.y, vector.z]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh(text: &str) -> EngineResult<MeshOutline> {
        let table: MeshTable =
            toml::from_str(text).map_err(|error| EngineError::Scene(error.to_string()))?;
        table.to_outline()
    }

    #[test]
    fn mesh_tables_give_shapes_and_materials() {
        let crate_mesh = mesh(
            "shape = \"cube\"\nsize = [1.0, 2.0, 3.0]\ncolor = [1.0, 0.0, 0.0, 1.0]\n\
             metallic = 1.0\ntexture = \"crate.png\"\nfilter = \"pixelated\"",
        )
        .unwrap();
        let ball = mesh("shape = \"sphere\"\nradius = 0.5").unwrap();

        assert_eq!(
            crate_mesh.shape,
            MeshShape::Cube {
                size: Vec3::new(1.0, 2.0, 3.0)
            }
        );
        assert_eq!(crate_mesh.metallic, 1.0);
        assert_eq!(crate_mesh.roughness, DEFAULT_ROUGHNESS);
        let (path, filter) = crate_mesh.texture.unwrap();
        assert_eq!(
            (path.as_str(), filter),
            ("crate.png", TextureFilter::Pixelated)
        );
        assert_eq!(
            ball.shape,
            MeshShape::Sphere {
                radius: 0.5,
                segments: DEFAULT_SPHERE_SEGMENTS,
                rings: DEFAULT_SPHERE_RINGS
            }
        );
    }

    #[test]
    fn shapes_need_their_own_sizes_only() {
        for text in [
            "shape = \"cube\"",
            "shape = \"cube\"\nsize = [1.0, 1.0]",
            "shape = \"plane\"\nsize = [1.0, 1.0]\nradius = 1.0",
            "shape = \"sphere\"",
            "shape = \"sphere\"\nradius = 1.0\nsize = [1.0, 1.0]",
            "shape = \"sphere\"\nradius = -1.0",
            "shape = \"sphere\"\nradius = 1.0\nsegments = 2",
            "shape = \"cube\"\nsize = [1.0, 1.0, 1.0]\ntexture = \"../outside.png\"",
        ] {
            assert!(mesh(text).is_err(), "{text}");
        }
    }

    #[test]
    fn written_tables_read_back_the_same() {
        let outline = MeshOutline {
            shape: MeshShape::Plane {
                size: Vec2::new(4.0, 2.0),
            },
            color: Color::rgb(0.5, 0.5, 0.5),
            metallic: 0.0,
            roughness: 0.9,
            texture: Some((AssetPath::new("ground.png").unwrap(), TextureFilter::Smooth)),
        };

        let text = toml::to_string(&MeshTable::from_outline(&outline)).unwrap();

        assert_eq!(mesh(&text).unwrap(), outline);
        assert!(
            !text.contains("metallic") && !text.contains("filter"),
            "{text}"
        );
    }

    #[test]
    fn light_tables_give_lights() {
        let read = |text: &str| -> EngineResult<LightNode> {
            let data: LightData =
                toml::from_str(text).map_err(|error| EngineError::Scene(error.to_string()))?;
            data.to_light()
        };

        let lamp = read("kind = \"point\"\nrange = 5.0\ncolor = [1.0, 0.5, 0.0]").unwrap();

        assert_eq!(
            lamp,
            LightNode::Point {
                color: Color::rgb(1.0, 0.5, 0.0),
                intensity: DEFAULT_INTENSITY,
                range: 5.0
            }
        );
        let text = toml::to_string(&LightData::from_light(&lamp)).unwrap();
        assert_eq!(read(&text).unwrap(), lamp);
        assert!(read("kind = \"point\"").is_err());
        assert!(read("kind = \"directional\"\nrange = 1.0").is_err());
        assert!(read("kind = \"point\"\nrange = 1.0\nangle = 0.5").is_err());
    }

    #[test]
    fn spot_tables_give_spots_with_their_cone() {
        let read = |text: &str| -> EngineResult<LightNode> {
            let data: LightData =
                toml::from_str(text).map_err(|error| EngineError::Scene(error.to_string()))?;
            data.to_light()
        };

        let spot = read("kind = \"spot\"\nrange = 8.0\nangle = 0.4").unwrap();

        assert_eq!(
            spot,
            LightNode::Spot {
                color: Color::WHITE,
                intensity: DEFAULT_INTENSITY,
                range: 8.0,
                angle: 0.4,
                blend: DEFAULT_SPOT_BLEND
            }
        );
        let text = toml::to_string(&LightData::from_light(&spot)).unwrap();
        assert_eq!(read(&text).unwrap(), spot);
        for broken in [
            "kind = \"spot\"",
            "kind = \"spot\"\nrange = 1.0\nangle = 2.0",
            "kind = \"spot\"\nrange = 1.0\nblend = 1.5",
        ] {
            assert!(read(broken).is_err(), "{broken}");
        }
    }

    #[test]
    fn transforms_read_back_the_same() {
        let transform = Transform3D::from_position(Vec3::new(1.0, 2.0, 3.0))
            .with_rotation(Quat::from_euler(Vec3::new(0.2, -0.4, 0.1)));

        let read = Transform3dData::from_transform(&transform).to_transform();

        assert_eq!(read.position, transform.position);
        assert_eq!(read.scale, Vec3::ONE);
        let point = Vec3::new(1.0, 1.0, 1.0);
        let error = (read.rotation.rotate(point) - transform.rotation.rotate(point)).length();
        assert!(error < 1e-5, "{error}");
    }
}
