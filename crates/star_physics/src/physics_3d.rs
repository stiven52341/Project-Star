//! The 3D simulation: bodies moving, falling and colliding in space.

use rapier3d::prelude::{
    ActiveCollisionTypes, ActiveEvents, ColliderBuilder, ColliderHandle, PhysicsWorld, Pose,
    RigidBodyBuilder, RigidBodyHandle, Rotation, SharedShape, Vector,
};
use star_core::{EngineError, EngineResult};
use star_math::{Quat, Vec3};

use crate::body::check_finite;
use crate::contact::ContactLog;
use crate::shape::scale_factor;
use crate::step_clock::StepClock;
use crate::{BodyKind, BodySettings, Contact, FIXED_STEP, Shape3d, ShapeKind3d, TriangleMesh};

/// Gravity of a new [`Physics3d`]: Earth's, downwards (-y), taking a world
/// unit as a meter.
pub const DEFAULT_GRAVITY_3D: Vec3 = Vec3::new(0.0, -9.81, 0.0);

/// A body in a [`Physics3d`], by its number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyId3d(RigidBodyHandle);

/// A 3D world of bodies that move under gravity and collide: the same as
/// a [`Physics2d`](crate::Physics2d), in space.
///
/// Time goes forward with [`advance`](Self::advance), in fixed steps; the
/// bodies that started or stopped touching meanwhile are its
/// [`contacts`](Self::contacts). Bodies are found by their [`BodyId3d`];
/// an id of a removed body is simply ignored.
pub struct Physics3d {
    world: PhysicsWorld,
    contacts: ContactLog<ColliderHandle, BodyId3d>,
    clock: StepClock,
}

impl Default for Physics3d {
    fn default() -> Self {
        Self::new(DEFAULT_GRAVITY_3D)
    }
}

impl Physics3d {
    /// An empty world with `gravity`, in world units per second squared.
    #[must_use]
    pub fn new(gravity: Vec3) -> Self {
        let mut world = PhysicsWorld::new();
        world.gravity = to_vector(gravity);
        world.integration_parameters.dt = FIXED_STEP;
        Self {
            world,
            contacts: ContactLog::new(),
            clock: StepClock::default(),
        }
    }

    /// The pull on every body, in world units per second squared.
    #[must_use]
    pub fn gravity(&self) -> Vec3 {
        from_vector(self.world.gravity)
    }

    /// Changes the pull on every body.
    pub fn set_gravity(&mut self, gravity: Vec3) {
        self.world.gravity = to_vector(gravity);
        self.world.wake_up_all(true);
    }

    /// Adds a body at rest with its center at `position`, turned by
    /// `rotation`, and gives its id.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`](star_core::EngineError::Physics) if the
    /// position, the rotation or the gravity scale is not finite, the
    /// friction or bounciness is negative, or the density is not positive.
    pub fn add_body(
        &mut self,
        settings: &BodySettings<Shape3d>,
        position: Vec3,
        rotation: Quat,
    ) -> EngineResult<BodyId3d> {
        let [x, y, z, w] = rotation.to_xyzw();
        check_finite(&[
            ("position", position.x),
            ("position", position.y),
            ("position", position.z),
            ("rotation", x),
            ("rotation", y),
            ("rotation", z),
            ("rotation", w),
        ])?;
        settings.check()?;
        let shape = shared_shape(&settings.shape).ok_or_else(|| {
            EngineError::Physics("a mesh body needs the triangles of its mesh".to_owned())
        })?;
        Ok(self.insert(settings, shape, position, rotation))
    }

    /// Adds a body with a [`mesh`](Shape3d::mesh) shape, made of the
    /// triangles of `mesh` stretched by `scale`, as
    /// [`add_body`](Self::add_body) adds the others: static and kinematic
    /// bodies collide with every triangle, and dynamic ones with the
    /// convex hull around them, which has a volume to fall and roll with.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`] for the reasons of `add_body`, if a factor
    /// of `scale` is not finite, or the triangles cannot make a shape (a
    /// dynamic body's are flat, for example).
    pub fn add_mesh_body(
        &mut self,
        settings: &BodySettings<Shape3d>,
        mesh: &TriangleMesh,
        scale: Vec3,
        position: Vec3,
        rotation: Quat,
    ) -> EngineResult<BodyId3d> {
        let [x, y, z, w] = rotation.to_xyzw();
        check_finite(&[
            ("position", position.x),
            ("position", position.y),
            ("position", position.z),
            ("rotation", x),
            ("rotation", y),
            ("rotation", z),
            ("rotation", w),
        ])?;
        settings.check()?;
        let shape = mesh_shape(mesh, scale, settings.kind == BodyKind::Dynamic)?;
        Ok(self.insert(settings, shape, position, rotation))
    }

    fn insert(
        &mut self,
        settings: &BodySettings<Shape3d>,
        shape: SharedShape,
        position: Vec3,
        rotation: Quat,
    ) -> BodyId3d {
        let body = body_builder(settings)
            .pose(Pose::from_parts(to_vector(position), to_rotation(rotation)));
        let (body, collider) = self.world.insert(body, collider_builder(settings, shape));
        let id = BodyId3d(body);
        self.contacts.add_owner(collider, id);
        id
    }

    /// Removes a body. Its contacts that were going on end with the next
    /// step.
    pub fn remove_body(&mut self, body: BodyId3d) {
        if let Some(removed) = self.world.remove_body(body.0) {
            self.contacts.remove_owners(removed.colliders());
        }
    }

    /// Whether `body` is in this world.
    #[must_use]
    pub fn contains(&self, body: BodyId3d) -> bool {
        self.world.bodies.contains(body.0)
    }

    /// How many bodies there are.
    #[must_use]
    pub fn body_count(&self) -> usize {
        self.world.bodies.len()
    }

    /// Where the center of `body` is, in world units.
    #[must_use]
    pub fn position(&self, body: BodyId3d) -> Option<Vec3> {
        let body = self.world.bodies.get(body.0)?;
        Some(from_vector(body.translation()))
    }

    /// How `body` is turned.
    #[must_use]
    pub fn rotation(&self, body: BodyId3d) -> Option<Quat> {
        let rotation = self.world.bodies.get(body.0)?.rotation();
        Quat::from_xyzw(rotation.to_array())
    }

    /// How fast `body` moves, in world units per second.
    #[must_use]
    pub fn velocity(&self, body: BodyId3d) -> Option<Vec3> {
        Some(from_vector(self.world.bodies.get(body.0)?.linvel()))
    }

    /// Moves `body` to `position` at once, through whatever is in between.
    pub fn set_position(&mut self, body: BodyId3d, position: Vec3) {
        if let Some(body) = self.world.bodies.get_mut(body.0) {
            body.set_translation(to_vector(position), true);
        }
    }

    /// Turns `body` to `rotation` at once.
    pub fn set_rotation(&mut self, body: BodyId3d, rotation: Quat) {
        if let Some(body) = self.world.bodies.get_mut(body.0) {
            body.set_rotation(to_rotation(rotation), true);
        }
    }

    /// Changes how fast `body` moves. Kinematic bodies move only this way.
    pub fn set_velocity(&mut self, body: BodyId3d, velocity: Vec3) {
        if let Some(body) = self.world.bodies.get_mut(body.0) {
            body.set_linvel(to_vector(velocity), true);
        }
    }

    /// Pushes `body` at once by `impulse` (mass times velocity), like a
    /// jump or a hit. Only dynamic bodies are pushed.
    pub fn apply_impulse(&mut self, body: BodyId3d, impulse: Vec3) {
        if let Some(body) = self.world.bodies.get_mut(body.0) {
            body.apply_impulse(to_vector(impulse), true);
        }
    }

    /// Gives `body` another shape, keeping where it is and how it moves:
    /// the shape of a node whose scale changed. Its mass follows the new
    /// shape, and its offset. A [`mesh`](Shape3d::mesh) shape only moves
    /// to its offset: its triangles change with
    /// [`set_mesh`](Self::set_mesh).
    pub fn set_shape(&mut self, body: BodyId3d, shape: &Shape3d) {
        if let Some(triangles) = shared_shape(shape) {
            self.replace_shape(body, &triangles);
        }
        let Some(rigid_body) = self.world.bodies.get(body.0) else {
            return;
        };
        for &collider in rigid_body.colliders() {
            if let Some(collider) = self.world.colliders.get_mut(collider) {
                collider.set_translation_wrt_parent(to_vector(shape.offset()));
            }
        }
    }

    /// Gives `body` the triangles of `mesh` stretched by `scale`, as
    /// [`add_mesh_body`](Self::add_mesh_body) makes them for its kind.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`] if a factor of `scale` is not finite, or
    /// the triangles cannot make a shape.
    pub fn set_mesh(
        &mut self,
        body: BodyId3d,
        mesh: &TriangleMesh,
        scale: Vec3,
    ) -> EngineResult<()> {
        let Some(rigid_body) = self.world.bodies.get(body.0) else {
            return Ok(());
        };
        let shape = mesh_shape(mesh, scale, rigid_body.is_dynamic())?;
        self.replace_shape(body, &shape);
        Ok(())
    }

    fn replace_shape(&mut self, body: BodyId3d, shape: &SharedShape) {
        let Some(rigid_body) = self.world.bodies.get(body.0) else {
            return;
        };
        for &collider in rigid_body.colliders() {
            if let Some(collider) = self.world.colliders.get_mut(collider) {
                collider.set_shape(shape.clone());
            }
        }
    }

    /// Simulates `elapsed` seconds more, in [`FIXED_STEP`]s, and gives how
    /// many steps ran. Time shorter than a step waits for the next call;
    /// at most [`MAX_STEPS_PER_ADVANCE`](crate::MAX_STEPS_PER_ADVANCE)
    /// steps run, and the rest is dropped. Contacts are those of the steps
    /// of this call.
    pub fn advance(&mut self, elapsed: f32) -> u32 {
        self.contacts.clear();
        let Self {
            world,
            contacts,
            clock,
        } = self;
        clock.advance(elapsed, || {
            world.step_with_events(&(), contacts.collector());
            contacts.collect_step();
        })
    }

    /// The bodies that started or stopped touching during the last
    /// [`advance`](Self::advance), in the order it happened.
    #[must_use]
    pub fn contacts(&self) -> &[Contact<BodyId3d>] {
        self.contacts.contacts()
    }
}

fn body_builder(settings: &BodySettings<Shape3d>) -> RigidBodyBuilder {
    let builder = match settings.kind {
        BodyKind::Dynamic => RigidBodyBuilder::dynamic(),
        BodyKind::Static => RigidBodyBuilder::fixed(),
        BodyKind::Kinematic => RigidBodyBuilder::kinematic_velocity_based(),
    };
    let builder = builder.gravity_scale(settings.gravity_scale);
    if settings.fixed_rotation {
        builder.lock_rotations()
    } else {
        builder
    }
}

fn collider_builder(settings: &BodySettings<Shape3d>, shape: SharedShape) -> ColliderBuilder {
    ColliderBuilder::new(shape)
        .translation(to_vector(settings.shape.offset()))
        .friction(settings.friction)
        .restitution(settings.bounciness)
        .density(settings.density)
        .sensor(settings.is_sensor)
        .active_events(ActiveEvents::COLLISION_EVENTS)
        // Every pair is reported, so a kinematic character also notices
        // static sensors (coins, doors); only dynamic bodies are pushed.
        .active_collision_types(ActiveCollisionTypes::all())
}

/// The collider shape of `shape`; `None` for a mesh, whose triangles it
/// does not have.
fn shared_shape(shape: &Shape3d) -> Option<SharedShape> {
    Some(match shape.kind() {
        ShapeKind3d::Sphere { radius } => SharedShape::ball(radius),
        ShapeKind3d::Box { size } => SharedShape::cuboid(size.x / 2.0, size.y / 2.0, size.z / 2.0),
        ShapeKind3d::Capsule { height, radius } => {
            SharedShape::capsule_y(height / 2.0 - radius, radius)
        }
        ShapeKind3d::Mesh => return None,
    })
}

/// The collider shape of the triangles of `mesh` stretched by `scale`
/// (mirrored by a negative factor): the convex hull around them for a
/// `dynamic` body, the triangles themselves otherwise.
fn mesh_shape(mesh: &TriangleMesh, scale: Vec3, dynamic: bool) -> EngineResult<SharedShape> {
    let signed = |factor: f32| scale_factor(factor).map(|size| size.copysign(factor));
    let (x, y, z) = (signed(scale.x)?, signed(scale.y)?, signed(scale.z)?);
    let corners: Vec<Vector> = mesh
        .corners()
        .iter()
        .map(|corner| Vector::new(corner.x * x, corner.y * y, corner.z * z))
        .collect();
    if dynamic {
        SharedShape::convex_hull(&corners)
            .filter(|hull| hull.mass_properties(1.0).mass() > f32::EPSILON)
            .ok_or_else(|| {
                EngineError::Physics(
                    "a dynamic mesh body needs a mesh with a volume, not a flat one".to_owned(),
                )
            })
    } else {
        SharedShape::trimesh(corners, mesh.triangles().to_vec())
            .map_err(|error| EngineError::Physics(format!("a mesh body cannot be made: {error}")))
    }
}

fn to_vector(vector: Vec3) -> Vector {
    Vector::new(vector.x, vector.y, vector.z)
}

fn from_vector(vector: Vector) -> Vec3 {
    Vec3::new(vector.x, vector.y, vector.z)
}

fn to_rotation(rotation: Quat) -> Rotation {
    let [x, y, z, w] = rotation.to_xyzw();
    Rotation::from_xyzw(x, y, z, w)
}

#[cfg(test)]
mod tests {
    use std::f32::consts::FRAC_PI_2;

    use super::*;
    use crate::{ContactPhase, MAX_STEPS_PER_ADVANCE};

    /// Seconds of a test frame: a 60 Hz screen.
    const FRAME: f32 = 1.0 / 60.0;
    const TOLERANCE: f32 = 1e-3;

    fn ball(kind: BodyKind) -> BodySettings<Shape3d> {
        BodySettings::new(kind, Shape3d::sphere(0.5).unwrap())
    }

    fn add(physics: &mut Physics3d, settings: &BodySettings<Shape3d>, position: Vec3) -> BodyId3d {
        physics
            .add_body(settings, position, Quat::IDENTITY)
            .unwrap()
    }

    fn floor() -> BodySettings<Shape3d> {
        BodySettings::new(
            BodyKind::Static,
            Shape3d::cuboid(Vec3::new(20.0, 1.0, 20.0)).unwrap(),
        )
    }

    fn run(physics: &mut Physics3d, seconds: f32) -> Vec<Contact<BodyId3d>> {
        let mut contacts = Vec::new();
        let mut elapsed = 0.0;
        while elapsed < seconds {
            physics.advance(FRAME);
            contacts.extend_from_slice(physics.contacts());
            elapsed += FRAME;
        }
        contacts
    }

    /// Two triangles making a square of 2 x 2 on the XZ plane, facing up.
    fn square() -> TriangleMesh {
        let corners = vec![
            Vec3::new(-1.0, 0.0, -1.0),
            Vec3::new(1.0, 0.0, -1.0),
            Vec3::new(1.0, 0.0, 1.0),
            Vec3::new(-1.0, 0.0, 1.0),
        ];
        TriangleMesh::new(corners, vec![[0, 2, 1], [0, 3, 2]]).unwrap()
    }

    /// The eight corners of a cube 1 across, and its twelve triangles.
    fn cube() -> TriangleMesh {
        let corners = (0..8)
            .map(|corner| {
                let side = |bit: u32| if corner & bit == 0 { -0.5 } else { 0.5 };
                Vec3::new(side(1), side(2), side(4))
            })
            .collect();
        let faces = [
            [0, 1, 3, 2],
            [4, 6, 7, 5],
            [0, 4, 5, 1],
            [2, 3, 7, 6],
            [0, 2, 6, 4],
            [1, 5, 7, 3],
        ];
        let triangles = faces
            .iter()
            .flat_map(|[a, b, c, d]| [[*a, *b, *c], [*a, *c, *d]])
            .collect();
        TriangleMesh::new(corners, triangles).unwrap()
    }

    fn mesh(kind: BodyKind) -> BodySettings<Shape3d> {
        BodySettings::new(kind, Shape3d::mesh())
    }

    #[test]
    fn bodies_land_on_the_triangles_of_static_meshes() {
        let mut physics = Physics3d::default();
        let scale = Vec3::new(5.0, 1.0, 5.0);
        physics
            .add_mesh_body(
                &mesh(BodyKind::Static),
                &square(),
                scale,
                Vec3::ZERO,
                Quat::IDENTITY,
            )
            .unwrap();
        // Beyond the unscaled square, within the stretched one.
        let falling = add(
            &mut physics,
            &ball(BodyKind::Dynamic),
            Vec3::new(3.0, 2.0, 3.0),
        );

        run(&mut physics, 2.0);

        let landed = physics.position(falling).unwrap();
        assert!((landed.y - 0.5).abs() < 0.05, "{landed:?}");
    }

    #[test]
    fn dynamic_meshes_rest_on_their_convex_hull() {
        let mut physics = Physics3d::default();
        add(&mut physics, &floor(), Vec3::new(0.0, -0.5, 0.0));
        let block = physics
            .add_mesh_body(
                &mesh(BodyKind::Dynamic),
                &cube(),
                Vec3::new(1.0, 2.0, 1.0),
                Vec3::new(0.0, 3.0, 0.0),
                Quat::IDENTITY,
            )
            .unwrap();

        run(&mut physics, 3.0);

        let rested = physics.position(block).unwrap();
        assert!((rested.y - 1.0).abs() < 0.05, "{rested:?}");
    }

    #[test]
    fn changed_meshes_keep_their_body() {
        let mut physics = Physics3d::default();
        let ground = physics
            .add_mesh_body(
                &mesh(BodyKind::Static),
                &square(),
                Vec3::ONE,
                Vec3::ZERO,
                Quat::IDENTITY,
            )
            .unwrap();
        let falling = add(
            &mut physics,
            &ball(BodyKind::Dynamic),
            Vec3::new(3.0, 2.0, 0.0),
        );

        physics
            .set_mesh(ground, &square(), Vec3::new(5.0, 1.0, 5.0))
            .unwrap();
        run(&mut physics, 2.0);

        assert!((physics.position(falling).unwrap().y - 0.5).abs() < 0.05);
    }

    #[test]
    fn mesh_bodies_need_their_triangles_and_a_volume_to_fall() {
        let mut physics = Physics3d::default();
        let flat = physics.add_mesh_body(
            &mesh(BodyKind::Dynamic),
            &square(),
            Vec3::ONE,
            Vec3::ZERO,
            Quat::IDENTITY,
        );

        assert!(
            physics
                .add_body(&mesh(BodyKind::Static), Vec3::ZERO, Quat::IDENTITY)
                .is_err()
        );
        assert!(matches!(flat, Err(EngineError::Physics(_))));
        assert_eq!(physics.body_count(), 0);
    }

    #[test]
    fn dynamic_bodies_fall_and_static_ones_stay() {
        let mut physics = Physics3d::default();
        let falling = add(
            &mut physics,
            &ball(BodyKind::Dynamic),
            Vec3::new(0.0, 10.0, 0.0),
        );
        let fixed = add(
            &mut physics,
            &ball(BodyKind::Static),
            Vec3::new(5.0, 10.0, 2.0),
        );

        run(&mut physics, 1.0);

        let fallen = physics.position(falling).unwrap();
        // About g / 2 after one second (the fixed steps round it a little).
        assert!((10.0 - fallen.y - 9.81 / 2.0).abs() < 0.2, "{fallen:?}");
        assert_eq!(physics.position(fixed), Some(Vec3::new(5.0, 10.0, 2.0)));
    }

    #[test]
    fn a_body_lands_on_the_floor_and_the_contact_is_reported() {
        let mut physics = Physics3d::default();
        let floor = add(&mut physics, &floor(), Vec3::ZERO);
        let falling = add(
            &mut physics,
            &ball(BodyKind::Dynamic),
            Vec3::new(0.0, 3.0, 0.0),
        );

        let contacts = run(&mut physics, 2.0);

        // The floor's top is at 0.5 and the ball's radius is 0.5.
        let rest = physics.position(falling).unwrap();
        assert!((rest.y - 1.0).abs() < 0.05, "{rest:?}");
        let started = contacts
            .iter()
            .find(|contact| contact.phase == ContactPhase::Started)
            .unwrap();
        assert_eq!(started.other(falling), Some(floor));
        assert!(!started.is_sensor);
    }

    #[test]
    fn sensors_detect_without_stopping_anything() {
        let mut physics = Physics3d::default();
        let mut area = BodySettings::new(
            BodyKind::Static,
            Shape3d::cuboid(Vec3::new(4.0, 1.0, 4.0)).unwrap(),
        );
        area.is_sensor = true;
        let area = add(&mut physics, &area, Vec3::ZERO);
        let falling = add(
            &mut physics,
            &ball(BodyKind::Dynamic),
            Vec3::new(0.0, 2.0, 0.0),
        );

        let contacts = run(&mut physics, 2.0);

        assert!(physics.position(falling).unwrap().y < -5.0);
        let phases: Vec<_> = contacts
            .iter()
            .filter(|contact| contact.involves(area))
            .map(|contact| (contact.phase, contact.is_sensor))
            .collect();
        assert_eq!(
            phases,
            [(ContactPhase::Started, true), (ContactPhase::Ended, true)]
        );
    }

    #[test]
    fn kinematic_bodies_follow_their_velocity_only() {
        let mut physics = Physics3d::default();
        let platform = add(&mut physics, &ball(BodyKind::Kinematic), Vec3::ZERO);
        physics.set_velocity(platform, Vec3::new(0.0, 0.0, -2.0));

        run(&mut physics, 1.0);

        let moved = physics.position(platform).unwrap();
        assert!((moved.z + 2.0).abs() < 0.05, "{moved:?}");
        assert!(moved.y.abs() < TOLERANCE);
    }

    #[test]
    fn impulses_push_dynamic_bodies() {
        let mut physics = Physics3d::new(Vec3::ZERO);
        let body = add(&mut physics, &ball(BodyKind::Dynamic), Vec3::ZERO);

        physics.apply_impulse(body, Vec3::new(1.0, 0.0, 0.0));
        physics.advance(FRAME);

        assert!(physics.velocity(body).unwrap().x > 0.0);
    }

    #[test]
    fn teleporting_moves_and_turns_a_body() {
        let mut physics = Physics3d::new(Vec3::ZERO);
        let body = add(&mut physics, &ball(BodyKind::Dynamic), Vec3::ZERO);
        let turned = Quat::from_rotation_y(FRAC_PI_2);

        physics.set_position(body, Vec3::new(3.0, 4.0, 5.0));
        physics.set_rotation(body, turned);

        assert_eq!(physics.position(body), Some(Vec3::new(3.0, 4.0, 5.0)));
        let point = Vec3::new(1.0, 0.0, 0.0);
        let rotation = physics.rotation(body).unwrap();
        assert!((rotation.rotate(point) - turned.rotate(point)).length() < TOLERANCE);
    }

    #[test]
    fn bodies_start_where_they_are_placed() {
        let mut physics = Physics3d::new(Vec3::ZERO);
        let turned = Quat::from_rotation_x(1.0);

        let body = physics
            .add_body(&ball(BodyKind::Dynamic), Vec3::new(1.0, 2.0, 3.0), turned)
            .unwrap();

        assert_eq!(physics.position(body), Some(Vec3::new(1.0, 2.0, 3.0)));
        let point = Vec3::UP;
        let rotation = physics.rotation(body).unwrap();
        assert!((rotation.rotate(point) - turned.rotate(point)).length() < TOLERANCE);
    }

    #[test]
    fn a_long_frame_runs_a_bounded_number_of_steps() {
        let mut physics = Physics3d::default();

        assert_eq!(physics.advance(FIXED_STEP / 2.0), 0);
        assert_eq!(physics.advance(10.0), MAX_STEPS_PER_ADVANCE);
        assert!(physics.advance(0.0) <= 1);
    }

    #[test]
    fn removed_bodies_end_their_contacts_and_ids() {
        let mut physics = Physics3d::default();
        let floor = add(&mut physics, &floor(), Vec3::ZERO);
        let falling = add(
            &mut physics,
            &ball(BodyKind::Dynamic),
            Vec3::new(0.0, 1.0, 0.0),
        );
        run(&mut physics, 0.5);

        physics.remove_body(falling);
        let contacts = run(&mut physics, 0.1);

        assert!(!physics.contains(falling));
        assert_eq!(physics.body_count(), 1);
        assert_eq!(physics.position(falling), None);
        assert!(contacts.iter().any(|contact| {
            contact.phase == ContactPhase::Ended && contact.other(floor) == Some(falling)
        }));
    }

    #[test]
    fn invalid_settings_are_rejected() {
        let mut physics = Physics3d::default();
        let mut settings = ball(BodyKind::Dynamic);
        assert!(
            physics
                .add_body(&settings, Vec3::new(0.0, 0.0, f32::NAN), Quat::IDENTITY)
                .is_err()
        );

        settings.bounciness = -1.0;
        assert!(
            physics
                .add_body(&settings, Vec3::ZERO, Quat::IDENTITY)
                .is_err()
        );
        assert_eq!(physics.body_count(), 0);
    }

    #[test]
    fn gravity_can_change() {
        let mut physics = Physics3d::default();
        let body = add(&mut physics, &ball(BodyKind::Dynamic), Vec3::ZERO);

        physics.set_gravity(Vec3::new(0.0, 5.0, 0.0));
        run(&mut physics, 0.5);

        assert_eq!(physics.gravity(), Vec3::new(0.0, 5.0, 0.0));
        assert!(physics.position(body).unwrap().y > 0.0);
    }
}
