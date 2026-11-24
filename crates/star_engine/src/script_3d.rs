//! What StarScript's 3D nodes show, as the generated game describes them to
//! [`ScriptNodes`](crate::script_nodes::ScriptNodes): the meshes of `Mesh`,
//! the light of `Light` and the view of `Camera3D`.
//!
//! As with bodies, shapes and kinds of light are words, and a `Mesh` keeps
//! the sizes of every shape; only those of its shape are used.

use star_core::{EngineError, EngineResult};
use star_math::{Quat, Vec2, Vec3};
use star_render::{
    Color, DEFAULT_SPHERE_RINGS, DEFAULT_SPHERE_SEGMENTS, DEFAULT_SPOT_ANGLE, DEFAULT_SPOT_BLEND,
    MAX_SPOT_ANGLE, MeshShape, TextureFilter,
};
use star_scene::{LightNode, MeshOutline};

use crate::script_body::{DEFAULT_RADIUS, DEFAULT_SIZE, SPHERE};

/// How StarScript names cubes (boxes, of any size).
pub const CUBE: &str = "cube";
/// How StarScript names planes: flat, facing up.
pub const PLANE: &str = "plane";
/// How StarScript names directional lights.
pub const DIRECTIONAL: &str = "directional";
/// How StarScript names point lights.
pub const POINT: &str = "point";
/// How StarScript names spot lights.
pub const SPOT: &str = "spot";

/// How far a script light reaches when its kind gives no range.
pub const DEFAULT_RANGE: f32 = 10.0;

/// The narrowest view a script camera shows, in radians: a zero, negative
/// or not-a-number field of view shows nothing.
const MIN_FIELD_OF_VIEW: f32 = 0.01;
/// The widest view a script camera shows, in radians: just under half a
/// turn, where the view turns inside out.
const MAX_FIELD_OF_VIEW: f32 = 3.1;

/// What a StarScript `Mesh` shows.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptMesh<'a> {
    /// [`CUBE`], [`SPHERE`] or [`PLANE`].
    pub shape: &'a str,
    /// Of cubes and planes, along x.
    pub width: f32,
    /// Of cubes, along y.
    pub height: f32,
    /// Of cubes and planes, along z.
    pub depth: f32,
    /// Of spheres.
    pub radius: f32,
    /// The color of the surface.
    pub color: Color,
    /// How metallic the surface is, from 0 to 1.
    pub metallic: f32,
    /// How rough the surface is, from 0 to 1.
    pub roughness: f32,
    /// Asset path of the image on it, as the script wrote it; empty for
    /// none.
    pub texture: &'a str,
    /// Keeps the image's pixels sharp.
    pub pixelated: bool,
}

impl<'a> ScriptMesh<'a> {
    /// The script mesh that a scene's `mesh` describes: how scenes give
    /// meshes to script nodes. The divisions of spheres are not kept.
    #[must_use]
    pub fn from_outline(mesh: &'a MeshOutline) -> Self {
        let (shape, size, radius) = match mesh.shape {
            MeshShape::Cube { size } => (CUBE, size, DEFAULT_RADIUS),
            MeshShape::Plane { size } => (
                PLANE,
                Vec3::new(size.x, DEFAULT_SIZE, size.y),
                DEFAULT_RADIUS,
            ),
            MeshShape::Sphere { radius, .. } => (SPHERE, Vec3::ONE * DEFAULT_SIZE, radius),
        };
        let (texture, filter) = mesh
            .texture
            .as_ref()
            .map_or(("", TextureFilter::Smooth), |(path, filter)| {
                (path.as_str(), *filter)
            });
        Self {
            shape,
            width: size.x,
            height: size.y,
            depth: size.z,
            radius,
            color: mesh.color,
            metallic: mesh.metallic,
            roughness: mesh.roughness,
            texture,
            pixelated: filter == TextureFilter::Pixelated,
        }
    }

    /// The shape the mesh shows.
    ///
    /// # Errors
    ///
    /// [`EngineError::Rendering`] if the shape is not a known word.
    pub fn to_shape(&self) -> EngineResult<MeshShape> {
        match self.shape {
            CUBE => Ok(MeshShape::Cube {
                size: Vec3::new(self.width, self.height, self.depth),
            }),
            PLANE => Ok(MeshShape::Plane {
                size: Vec2::new(self.width, self.depth),
            }),
            SPHERE => Ok(MeshShape::Sphere {
                radius: self.radius,
                segments: DEFAULT_SPHERE_SEGMENTS,
                rings: DEFAULT_SPHERE_RINGS,
            }),
            other => Err(EngineError::Rendering(format!(
                "a mesh is a \"{CUBE}\", a \"{SPHERE}\" or a \"{PLANE}\", not a \"{other}\""
            ))),
        }
    }
}

/// What a StarScript `Light` shines.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptLight<'a> {
    /// [`DIRECTIONAL`] or [`POINT`].
    pub kind: &'a str,
    /// Its color.
    pub color: Color,
    /// How bright it is.
    pub intensity: f32,
    /// How far a point light or a spot reaches.
    pub range: f32,
    /// From a spot's axis to the edge of its cone, in radians.
    pub angle: f32,
    /// How much of a spot's cone fades, from 0 to 1.
    pub blend: f32,
}

impl ScriptLight<'static> {
    /// The script light that a scene's `light` describes.
    #[must_use]
    pub const fn from_light(light: &LightNode) -> Self {
        match *light {
            LightNode::Directional { color, intensity } => Self {
                kind: DIRECTIONAL,
                color,
                intensity,
                range: DEFAULT_RANGE,
                angle: DEFAULT_SPOT_ANGLE,
                blend: DEFAULT_SPOT_BLEND,
            },
            LightNode::Point {
                color,
                intensity,
                range,
            } => Self {
                kind: POINT,
                color,
                intensity,
                range,
                angle: DEFAULT_SPOT_ANGLE,
                blend: DEFAULT_SPOT_BLEND,
            },
            LightNode::Spot {
                color,
                intensity,
                range,
                angle,
                blend,
            } => Self {
                kind: SPOT,
                color,
                intensity,
                range,
                angle,
                blend,
            },
        }
    }
}

impl ScriptLight<'_> {
    /// The light of the node.
    ///
    /// # Errors
    ///
    /// [`EngineError::Rendering`] if the kind is not a known word, or a
    /// spot's angle is not more than 0 and less than a right angle. A
    /// spot's blend is kept from 0 to 1.
    pub fn to_light(&self) -> EngineResult<LightNode> {
        match self.kind {
            DIRECTIONAL => Ok(LightNode::Directional {
                color: self.color,
                intensity: self.intensity,
            }),
            POINT => Ok(LightNode::Point {
                color: self.color,
                intensity: self.intensity,
                range: self.range,
            }),
            SPOT if self.angle > 0.0 && self.angle < MAX_SPOT_ANGLE => Ok(LightNode::Spot {
                color: self.color,
                intensity: self.intensity,
                range: self.range,
                angle: self.angle,
                blend: self.blend.clamp(0.0, 1.0),
            }),
            SPOT => Err(EngineError::Rendering(format!(
                "a spot's angle is more than 0 and less than {MAX_SPOT_ANGLE} radians, not {}",
                self.angle
            ))),
            other => Err(EngineError::Rendering(format!(
                "a light is \"{DIRECTIONAL}\", \"{POINT}\" or \"{SPOT}\", not \"{other}\""
            ))),
        }
    }
}

/// A StarScript `Camera3D`, as the generated game describes it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptCamera3D {
    /// The script object, as in
    /// [`ScriptNode::id`](crate::script_nodes::ScriptNode::id).
    pub id: u64,
    /// Where it is, relative to its parent.
    pub position: Vec3,
    /// How it is turned, relative to its parent: pitch, yaw and roll.
    pub angles: Vec3,
    /// Vertical field of view, in radians.
    pub field_of_view: f32,
}

impl ScriptCamera3D {
    /// The field of view, kept between what can be shown.
    #[must_use]
    pub fn shown_field_of_view(&self) -> f32 {
        if self.field_of_view.is_nan() {
            MIN_FIELD_OF_VIEW
        } else {
            self.field_of_view
                .clamp(MIN_FIELD_OF_VIEW, MAX_FIELD_OF_VIEW)
        }
    }

    /// How the camera is turned, from its angles.
    #[must_use]
    pub fn rotation(&self) -> Quat {
        Quat::from_euler(self.angles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mesh(shape: &str) -> ScriptMesh<'_> {
        ScriptMesh {
            shape,
            width: 1.0,
            height: 2.0,
            depth: 3.0,
            radius: 0.5,
            color: Color::WHITE,
            metallic: 0.0,
            roughness: 0.5,
            texture: "",
            pixelated: false,
        }
    }

    #[test]
    fn meshes_take_the_sizes_of_their_shape() {
        assert_eq!(
            mesh(CUBE).to_shape().unwrap(),
            MeshShape::Cube {
                size: Vec3::new(1.0, 2.0, 3.0)
            }
        );
        assert_eq!(
            mesh(PLANE).to_shape().unwrap(),
            MeshShape::Plane {
                size: Vec2::new(1.0, 3.0)
            }
        );
        assert!(matches!(
            mesh(SPHERE).to_shape().unwrap(),
            MeshShape::Sphere { radius: 0.5, .. }
        ));
        assert!(matches!(
            mesh("teapot").to_shape(),
            Err(EngineError::Rendering(_))
        ));
    }

    #[test]
    fn scene_meshes_and_lights_read_back_the_same() {
        let outline = MeshOutline {
            shape: MeshShape::Plane {
                size: Vec2::new(4.0, 5.0),
            },
            color: Color::rgb(0.1, 0.2, 0.3),
            metallic: 1.0,
            roughness: 0.3,
            texture: Some((
                star_assets::AssetPath::new("floor.png").unwrap(),
                TextureFilter::Pixelated,
            )),
        };
        let mesh = ScriptMesh::from_outline(&outline);

        assert_eq!(mesh.to_shape().unwrap(), outline.shape);
        assert_eq!((mesh.texture, mesh.pixelated), ("floor.png", true));
        let lamp = LightNode::Point {
            color: Color::WHITE,
            intensity: 2.0,
            range: 3.0,
        };
        assert_eq!(ScriptLight::from_light(&lamp).to_light().unwrap(), lamp);
    }

    #[test]
    fn lights_are_directional_or_point() {
        let light = ScriptLight {
            kind: POINT,
            color: Color::WHITE,
            intensity: 2.0,
            range: 5.0,
            angle: 0.5,
            blend: 3.0,
        };

        assert!(matches!(
            light.to_light().unwrap(),
            LightNode::Point { range: 5.0, .. }
        ));
        assert!(matches!(
            ScriptLight {
                kind: DIRECTIONAL,
                ..light
            }
            .to_light()
            .unwrap(),
            LightNode::Directional { .. }
        ));
        assert!(
            ScriptLight {
                kind: "laser",
                ..light
            }
            .to_light()
            .is_err()
        );
        let spot = ScriptLight {
            kind: SPOT,
            ..light
        };
        assert!(matches!(
            spot.to_light().unwrap(),
            LightNode::Spot {
                angle: 0.5,
                blend: 1.0,
                ..
            }
        ));
        assert!(ScriptLight { angle: 2.0, ..spot }.to_light().is_err());
    }

    #[test]
    fn fields_of_view_stay_showable() {
        let camera = |field_of_view| ScriptCamera3D {
            id: 1,
            position: Vec3::ZERO,
            angles: Vec3::ZERO,
            field_of_view,
        };

        assert!((camera(1.0).shown_field_of_view() - 1.0).abs() < f32::EPSILON);
        for broken in [0.0, -1.0, f32::NAN] {
            assert!(
                (camera(broken).shown_field_of_view() - MIN_FIELD_OF_VIEW).abs() < f32::EPSILON
            );
        }
        assert!((camera(10.0).shown_field_of_view() - MAX_FIELD_OF_VIEW).abs() < f32::EPSILON);
    }
}
