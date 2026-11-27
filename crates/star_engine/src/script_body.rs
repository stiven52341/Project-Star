//! The physics bodies of StarScript nodes, as the generated game describes
//! them to [`ScriptNodes`](crate::script_nodes::ScriptNodes).
//!
//! StarScript has no enums yet, so a script's `Body` names how it moves and
//! its shape with words, and keeps the sizes of every shape; only those of
//! its shape are used. The same `Body` is a body of the 2D physics on a 2D
//! node and of the 3D physics on a `Node3D`.

use star_core::{EngineError, EngineResult};
use star_math::{Vec2, Vec3};
use star_physics::{BodyKind, BodySettings, Shape2d, Shape3d, ShapeKind, ShapeKind3d};

/// How StarScript names [`BodyKind::Dynamic`].
pub const DYNAMIC: &str = "dynamic";
/// How StarScript names [`BodyKind::Static`].
pub const STATIC: &str = "static";
/// How StarScript names [`BodyKind::Kinematic`].
pub const KINEMATIC: &str = "kinematic";
/// How StarScript names circles.
pub const CIRCLE: &str = "circle";
/// How StarScript names rectangles.
pub const RECTANGLE: &str = "rectangle";
/// How StarScript names capsules, in 2D and 3D.
pub const CAPSULE: &str = "capsule";
/// How StarScript names spheres.
pub const SPHERE: &str = "sphere";
/// How StarScript names boxes.
pub const BOX: &str = "box";
/// How StarScript names the shape of the triangles of a 3D node's mesh or
/// model.
pub const MESH: &str = "mesh";

/// The radius of a script body whose shape does not give one.
pub const DEFAULT_RADIUS: f32 = 0.5;
/// The width, height and depth of a script body whose shape does not give
/// them.
pub const DEFAULT_SIZE: f32 = 1.0;

/// A StarScript `Body`: what the body is, and what the script asked it to
/// do during its frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptBody<'a> {
    /// [`DYNAMIC`], [`STATIC`] or [`KINEMATIC`].
    pub kind: &'a str,
    /// [`CIRCLE`], [`RECTANGLE`] or [`CAPSULE`] in 2D; [`SPHERE`], [`BOX`],
    /// [`CAPSULE`] or [`MESH`] in 3D.
    pub shape: &'a str,
    /// Of circles, spheres and capsules.
    pub radius: f32,
    /// Of rectangles and boxes.
    pub width: f32,
    /// Of rectangles, boxes and capsules (round ends included).
    pub height: f32,
    /// Of boxes.
    pub depth: f32,
    /// Where the shape's center is from the node (`z` is left out in 2D).
    pub offset: Vec3,
    /// How much gravity pulls it.
    pub gravity_scale: f32,
    /// Whether collisions never turn it.
    pub fixed_rotation: bool,
    /// How hard it is to slide over.
    pub friction: f32,
    /// How much it bounces.
    pub bounciness: f32,
    /// Mass per square (2D) or cubic (3D) world unit.
    pub density: f32,
    /// Whether it only detects what overlaps it.
    pub sensor: bool,
    /// The velocity the script's `Body` has, in world units per second
    /// (`z` is left out in 2D).
    pub velocity: Vec3,
    /// The impulses the script gave it during its frame, added up (`z` is
    /// left out in 2D).
    pub impulse: Vec3,
}

/// What the engine tells a StarScript `Body` before the script's frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScriptBodyState<'a> {
    /// Whether the body is in the 3D physics, so `position.z` and the
    /// pitch and yaw of `angles` count.
    pub is_3d: bool,
    /// Where the simulation left its node, relative to the node's parent.
    pub position: Vec3,
    /// How it left it turned, relative to the node's parent: pitch (`x`),
    /// yaw (`y`) and roll (`z`, the rotation of 2D nodes), in radians.
    pub angles: Vec3,
    /// How fast it moves, in world units per second.
    pub velocity: Vec3,
    /// The script nodes whose bodies touch it (or overlap it, for sensors).
    pub touching: &'a [u64],
    /// The script nodes that started touching it during the last frame.
    pub just_touched: &'a [u64],
    /// The script nodes that stopped touching it during the last frame.
    pub just_separated: &'a [u64],
}

impl ScriptBody<'static> {
    /// The script body that `settings` describe, at rest: how scenes give
    /// bodies to script nodes.
    #[must_use]
    pub fn from_settings(settings: &BodySettings) -> Self {
        let (shape, radius, size) = match settings.shape.kind() {
            ShapeKind::Circle { radius } => (CIRCLE, radius, Vec2::ONE * DEFAULT_SIZE),
            ShapeKind::Rectangle { size } => (RECTANGLE, DEFAULT_RADIUS, size),
            ShapeKind::Capsule { height, radius } => {
                (CAPSULE, radius, Vec2::new(DEFAULT_SIZE, height))
            }
        };
        let offset = settings.shape.offset();
        Self::at_rest(
            settings,
            shape,
            radius,
            Vec3::new(size.x, size.y, DEFAULT_SIZE),
        )
        .with_offset(Vec3::new(offset.x, offset.y, 0.0))
    }

    /// The script body that `settings` of a 3D body describe, at rest.
    #[must_use]
    pub fn from_settings_3d(settings: &BodySettings<Shape3d>) -> Self {
        let (shape, radius, size) = match settings.shape.kind() {
            ShapeKind3d::Sphere { radius } => (SPHERE, radius, Vec3::ONE * DEFAULT_SIZE),
            ShapeKind3d::Box { size } => (BOX, DEFAULT_RADIUS, size),
            ShapeKind3d::Capsule { height, radius } => (
                CAPSULE,
                radius,
                Vec3::new(DEFAULT_SIZE, height, DEFAULT_SIZE),
            ),
            ShapeKind3d::Mesh => (MESH, DEFAULT_RADIUS, Vec3::ONE * DEFAULT_SIZE),
        };
        Self::at_rest(settings, shape, radius, size).with_offset(settings.shape.offset())
    }

    const fn with_offset(mut self, offset: Vec3) -> Self {
        self.offset = offset;
        self
    }

    fn at_rest<Shape>(
        settings: &BodySettings<Shape>,
        shape: &'static str,
        radius: f32,
        size: Vec3,
    ) -> Self {
        let kind = match settings.kind {
            BodyKind::Dynamic => DYNAMIC,
            BodyKind::Static => STATIC,
            BodyKind::Kinematic => KINEMATIC,
        };
        Self {
            kind,
            shape,
            radius,
            width: size.x,
            height: size.y,
            depth: size.z,
            offset: Vec3::ZERO,
            gravity_scale: settings.gravity_scale,
            fixed_rotation: settings.fixed_rotation,
            friction: settings.friction,
            bounciness: settings.bounciness,
            density: settings.density,
            sensor: settings.is_sensor,
            velocity: Vec3::ZERO,
            impulse: Vec3::ZERO,
        }
    }
}

impl ScriptBody<'_> {
    /// The body to simulate on a 2D node.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`] if the kind or shape is not a known word
    /// (of 2D), or the physics cannot simulate the body (see
    /// [`BodySettings::check`]).
    pub fn to_settings(&self) -> EngineResult<BodySettings> {
        let kind = self.body_kind()?;
        let shape = match self.shape {
            CIRCLE => Shape2d::circle(self.radius),
            RECTANGLE => Shape2d::rectangle(Vec2::new(self.width, self.height)),
            CAPSULE => Shape2d::capsule(self.height, self.radius),
            other => {
                return Err(EngineError::Physics(format!(
                    "a body of a 2D node is a \"{CIRCLE}\", a \"{RECTANGLE}\" or a \"{CAPSULE}\", not a \"{other}\""
                )));
            }
        }?
        .with_offset(Vec2::new(self.offset.x, self.offset.y))?;
        self.with_surface(kind, shape)
    }

    /// The body to simulate on a `Node3D`.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`] if the kind or shape is not a known word
    /// (of 3D), or the physics cannot simulate the body (see
    /// [`BodySettings::check`]).
    pub fn to_settings_3d(&self) -> EngineResult<BodySettings<Shape3d>> {
        let kind = self.body_kind()?;
        let shape = match self.shape {
            SPHERE => Shape3d::sphere(self.radius),
            BOX => Shape3d::cuboid(Vec3::new(self.width, self.height, self.depth)),
            CAPSULE => Shape3d::capsule(self.height, self.radius),
            MESH => Ok(Shape3d::mesh()),
            other => {
                return Err(EngineError::Physics(format!(
                    "a body of a 3D node is a \"{SPHERE}\", a \"{BOX}\", a \"{CAPSULE}\" or a \"{MESH}\", not a \"{other}\""
                )));
            }
        }?
        .with_offset(self.offset)?;
        self.with_surface(kind, shape)
    }

    fn body_kind(&self) -> EngineResult<BodyKind> {
        match self.kind {
            DYNAMIC => Ok(BodyKind::Dynamic),
            STATIC => Ok(BodyKind::Static),
            KINEMATIC => Ok(BodyKind::Kinematic),
            other => Err(EngineError::Physics(format!(
                "a body is \"{DYNAMIC}\", \"{STATIC}\" or \"{KINEMATIC}\", not \"{other}\""
            ))),
        }
    }

    /// The settings of a body of `kind` and `shape` with the script's
    /// surface, checked.
    fn with_surface<Shape>(
        &self,
        kind: BodyKind,
        shape: Shape,
    ) -> EngineResult<BodySettings<Shape>> {
        let settings = BodySettings {
            gravity_scale: self.gravity_scale,
            fixed_rotation: self.fixed_rotation,
            friction: self.friction,
            bounciness: self.bounciness,
            density: self.density,
            is_sensor: self.sensor,
            ..BodySettings::new(kind, shape)
        };
        settings.check()?;
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_become_script_bodies_and_back() {
        let shapes = [
            Shape2d::circle(0.3).unwrap(),
            Shape2d::rectangle(Vec2::new(4.0, 1.0))
                .unwrap()
                .with_offset(Vec2::new(1.0, -2.0))
                .unwrap(),
            Shape2d::capsule(2.0, 0.5).unwrap(),
        ];
        for (kind, shape) in [BodyKind::Dynamic, BodyKind::Static, BodyKind::Kinematic]
            .into_iter()
            .zip(shapes)
        {
            let mut settings = BodySettings::new(kind, shape);
            settings.bounciness = 0.7;
            settings.is_sensor = true;

            let body = ScriptBody::from_settings(&settings);

            assert_eq!(body.to_settings().unwrap(), settings);
        }
    }

    #[test]
    fn settings_become_3d_script_bodies_and_back() {
        let shapes = [
            Shape3d::sphere(0.3).unwrap(),
            Shape3d::cuboid(Vec3::new(4.0, 1.0, 2.0)).unwrap(),
            Shape3d::capsule(2.0, 0.5)
                .unwrap()
                .with_offset(Vec3::new(0.0, 1.0, -0.5))
                .unwrap(),
            Shape3d::mesh(),
        ];
        for shape in shapes {
            let mut settings = BodySettings::new(BodyKind::Kinematic, shape);
            settings.friction = 0.1;

            let body = ScriptBody::from_settings_3d(&settings);

            assert_eq!(body.to_settings_3d().unwrap(), settings);
        }
    }

    #[test]
    fn shapes_belong_to_their_dimension() {
        let sphere = ScriptBody::from_settings_3d(&BodySettings::new(
            BodyKind::Dynamic,
            Shape3d::sphere(1.0).unwrap(),
        ));
        let circle = ScriptBody::from_settings(&BodySettings::new(
            BodyKind::Dynamic,
            Shape2d::circle(1.0).unwrap(),
        ));
        let capsule = ScriptBody {
            shape: CAPSULE,
            height: 2.0,
            ..circle
        };

        assert!(matches!(sphere.to_settings(), Err(EngineError::Physics(_))));
        assert!(matches!(
            circle.to_settings_3d(),
            Err(EngineError::Physics(_))
        ));
        assert!(capsule.to_settings().is_ok() && capsule.to_settings_3d().is_ok());
    }

    #[test]
    fn unknown_words_and_impossible_bodies_are_physics_errors() {
        let body = ScriptBody::from_settings(&BodySettings::new(
            BodyKind::Dynamic,
            Shape2d::circle(1.0).unwrap(),
        ));
        let broken = [
            ScriptBody {
                kind: "flying",
                ..body
            },
            ScriptBody {
                shape: "triangle",
                ..body
            },
            ScriptBody {
                radius: 0.0,
                ..body
            },
            ScriptBody {
                density: -1.0,
                ..body
            },
        ];
        for body in broken {
            assert!(
                matches!(body.to_settings(), Err(EngineError::Physics(_))),
                "{body:?}"
            );
        }
    }
}
