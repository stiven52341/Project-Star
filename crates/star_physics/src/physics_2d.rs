//! The 2D simulation: bodies moving, falling and colliding.

use rapier2d::prelude::{
    ActiveCollisionTypes, ActiveEvents, ColliderBuilder, ColliderHandle, PhysicsWorld,
    RigidBodyBuilder, RigidBodyHandle, Rotation, SharedShape, Vector,
};
use star_core::EngineResult;
use star_math::Vec2;

use crate::body::check_finite;
use crate::contact::ContactLog;
use crate::shape::ShapeKind;
use crate::step_clock::StepClock;
use crate::{BodyKind, BodySettings, Contact, FIXED_STEP, Shape2d};

/// Gravity of a new [`Physics2d`]: Earth's, downwards, taking a world unit
/// as a meter.
pub const DEFAULT_GRAVITY: Vec2 = Vec2::new(0.0, -9.81);

/// A body in a [`Physics2d`], by its number.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BodyId(pub(crate) RigidBodyHandle);

/// A world of bodies that move under gravity and collide.
///
/// Time goes forward with [`advance`](Self::advance), in fixed steps; the
/// bodies that started or stopped touching meanwhile are its
/// [`contacts`](Self::contacts). Bodies are found by their [`BodyId`]; an
/// id of a removed body is simply ignored.
pub struct Physics2d {
    world: PhysicsWorld,
    contacts: ContactLog<ColliderHandle, BodyId>,
    clock: StepClock,
}

impl Default for Physics2d {
    fn default() -> Self {
        Self::new(DEFAULT_GRAVITY)
    }
}

impl Physics2d {
    /// An empty world with `gravity`, in world units per second squared.
    #[must_use]
    pub fn new(gravity: Vec2) -> Self {
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
    pub fn gravity(&self) -> Vec2 {
        from_vector(self.world.gravity)
    }

    /// Changes the pull on every body.
    pub fn set_gravity(&mut self, gravity: Vec2) {
        self.world.gravity = to_vector(gravity);
        self.world.wake_up_all(true);
    }

    /// Adds a body at rest with its center at `position`, turned `rotation`
    /// radians counterclockwise, and gives its id.
    ///
    /// # Errors
    ///
    /// [`EngineError::Physics`](star_core::EngineError::Physics) if the position, the rotation or the gravity
    /// scale is not finite, the friction or bounciness is negative, or the
    /// density is not positive.
    pub fn add_body(
        &mut self,
        settings: &BodySettings,
        position: Vec2,
        rotation: f32,
    ) -> EngineResult<BodyId> {
        check_settings(settings, position, rotation)?;
        let body = body_builder(settings)
            .translation(to_vector(position))
            .rotation(rotation);
        let (body, collider) = self.world.insert(body, collider_builder(settings));
        let id = BodyId(body);
        self.contacts.add_owner(collider, id);
        Ok(id)
    }

    /// Removes a body. Its contacts that were going on end with the next
    /// step.
    pub fn remove_body(&mut self, body: BodyId) {
        if let Some(removed) = self.world.remove_body(body.0) {
            self.contacts.remove_owners(removed.colliders());
        }
    }

    /// Whether `body` is in this world.
    #[must_use]
    pub fn contains(&self, body: BodyId) -> bool {
        self.world.bodies.contains(body.0)
    }

    /// How many bodies there are.
    #[must_use]
    pub fn body_count(&self) -> usize {
        self.world.bodies.len()
    }

    /// Where the center of `body` is, in world units.
    #[must_use]
    pub fn position(&self, body: BodyId) -> Option<Vec2> {
        let body = self.world.bodies.get(body.0)?;
        Some(from_vector(body.translation()))
    }

    /// The angle of `body`, in radians, counterclockwise.
    #[must_use]
    pub fn rotation(&self, body: BodyId) -> Option<f32> {
        Some(self.world.bodies.get(body.0)?.rotation().angle())
    }

    /// How fast `body` moves, in world units per second.
    #[must_use]
    pub fn velocity(&self, body: BodyId) -> Option<Vec2> {
        Some(from_vector(self.world.bodies.get(body.0)?.linvel()))
    }

    /// Moves `body` to `position` at once, through whatever is in between.
    pub fn set_position(&mut self, body: BodyId, position: Vec2) {
        if let Some(body) = self.world.bodies.get_mut(body.0) {
            body.set_translation(to_vector(position), true);
        }
    }

    /// Turns `body` to `rotation` (radians, counterclockwise) at once.
    pub fn set_rotation(&mut self, body: BodyId, rotation: f32) {
        if let Some(body) = self.world.bodies.get_mut(body.0) {
            body.set_rotation(Rotation::new(rotation), true);
        }
    }

    /// Changes how fast `body` moves. Kinematic bodies move only this way.
    pub fn set_velocity(&mut self, body: BodyId, velocity: Vec2) {
        if let Some(body) = self.world.bodies.get_mut(body.0) {
            body.set_linvel(to_vector(velocity), true);
        }
    }

    /// Pushes `body` at once by `impulse` (mass times velocity), like a
    /// jump or a hit. Only dynamic bodies are pushed.
    pub fn apply_impulse(&mut self, body: BodyId, impulse: Vec2) {
        if let Some(body) = self.world.bodies.get_mut(body.0) {
            body.apply_impulse(to_vector(impulse), true);
        }
    }

    /// Gives `body` another shape, keeping where it is and how it moves:
    /// the shape of a node whose scale changed. Its mass follows the new
    /// shape.
    pub fn set_shape(&mut self, body: BodyId, shape: &Shape2d) {
        let Some(rigid_body) = self.world.bodies.get(body.0) else {
            return;
        };
        for &collider in rigid_body.colliders() {
            if let Some(collider) = self.world.colliders.get_mut(collider) {
                collider.set_shape(shared_shape(shape));
                collider.set_translation_wrt_parent(to_vector(shape.offset()));
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
    pub fn contacts(&self) -> &[Contact] {
        self.contacts.contacts()
    }
}

fn body_builder(settings: &BodySettings) -> RigidBodyBuilder {
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

fn collider_builder(settings: &BodySettings) -> ColliderBuilder {
    ColliderBuilder::new(shared_shape(&settings.shape))
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

/// The collider shape of `shape`.
fn shared_shape(shape: &Shape2d) -> SharedShape {
    match shape.kind() {
        ShapeKind::Circle { radius } => SharedShape::ball(radius),
        ShapeKind::Rectangle { size } => SharedShape::cuboid(size.x / 2.0, size.y / 2.0),
        ShapeKind::Capsule { height, radius } => {
            SharedShape::capsule_y(height / 2.0 - radius, radius)
        }
    }
}

fn check_settings(settings: &BodySettings, position: Vec2, rotation: f32) -> EngineResult<()> {
    check_finite(&[
        ("position", position.x),
        ("position", position.y),
        ("rotation", rotation),
    ])?;
    settings.check()
}

fn to_vector(vector: Vec2) -> Vector {
    Vector::new(vector.x, vector.y)
}

fn from_vector(vector: Vector) -> Vec2 {
    Vec2::new(vector.x, vector.y)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContactPhase, MAX_STEPS_PER_ADVANCE, Shape2d};

    /// Seconds of a test frame: a 60 Hz screen.
    const FRAME: f32 = 1.0 / 60.0;
    const TOLERANCE: f32 = 1e-3;

    fn ball(kind: BodyKind) -> BodySettings {
        BodySettings::new(kind, Shape2d::circle(0.5).unwrap())
    }

    fn add(physics: &mut Physics2d, settings: &BodySettings, position: Vec2) -> BodyId {
        physics.add_body(settings, position, 0.0).unwrap()
    }

    fn floor() -> BodySettings {
        BodySettings::new(
            BodyKind::Static,
            Shape2d::rectangle(Vec2::new(20.0, 1.0)).unwrap(),
        )
    }

    fn run(physics: &mut Physics2d, seconds: f32) -> Vec<Contact> {
        let mut contacts = Vec::new();
        let mut elapsed = 0.0;
        while elapsed < seconds {
            physics.advance(FRAME);
            contacts.extend_from_slice(physics.contacts());
            elapsed += FRAME;
        }
        contacts
    }

    #[test]
    fn dynamic_bodies_fall_and_static_ones_stay() {
        let mut physics = Physics2d::default();
        let falling = add(&mut physics, &ball(BodyKind::Dynamic), Vec2::new(0.0, 10.0));
        let fixed = add(&mut physics, &ball(BodyKind::Static), Vec2::new(5.0, 10.0));

        run(&mut physics, 1.0);

        let fallen = physics.position(falling).unwrap();
        // About g / 2 after one second (the fixed steps round it a little).
        assert!((10.0 - fallen.y - 9.81 / 2.0).abs() < 0.2, "{fallen:?}");
        assert_eq!(physics.position(fixed), Some(Vec2::new(5.0, 10.0)));
    }

    #[test]
    fn a_body_lands_on_the_floor_and_the_contact_is_reported() {
        let mut physics = Physics2d::default();
        let floor = add(&mut physics, &floor(), Vec2::ZERO);
        let falling = add(&mut physics, &ball(BodyKind::Dynamic), Vec2::new(0.0, 3.0));

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
        let mut physics = Physics2d::default();
        let mut area = BodySettings::new(
            BodyKind::Static,
            Shape2d::rectangle(Vec2::new(4.0, 1.0)).unwrap(),
        );
        area.is_sensor = true;
        let area = add(&mut physics, &area, Vec2::ZERO);
        let falling = add(&mut physics, &ball(BodyKind::Dynamic), Vec2::new(0.0, 2.0));

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
        let mut physics = Physics2d::default();
        let platform = add(&mut physics, &ball(BodyKind::Kinematic), Vec2::ZERO);
        physics.set_velocity(platform, Vec2::new(2.0, 0.0));

        run(&mut physics, 1.0);

        let moved = physics.position(platform).unwrap();
        assert!((moved.x - 2.0).abs() < 0.05, "{moved:?}");
        assert!(moved.y.abs() < TOLERANCE);
    }

    #[test]
    fn kinematic_bodies_notice_static_sensors() {
        let mut physics = Physics2d::new(Vec2::ZERO);
        let mut coin = ball(BodyKind::Static);
        coin.is_sensor = true;
        let coin = add(&mut physics, &coin, Vec2::new(1.0, 0.0));
        let player = add(
            &mut physics,
            &ball(BodyKind::Kinematic),
            Vec2::new(-1.0, 0.0),
        );
        physics.set_velocity(player, Vec2::new(2.0, 0.0));

        let contacts = run(&mut physics, 1.0);

        assert!(
            contacts
                .iter()
                .any(|contact| contact.other(coin) == Some(player))
        );
    }

    #[test]
    fn impulses_push_dynamic_bodies() {
        let mut physics = Physics2d::new(Vec2::ZERO);
        let body = add(&mut physics, &ball(BodyKind::Dynamic), Vec2::ZERO);

        physics.apply_impulse(body, Vec2::new(0.0, 1.0));
        physics.advance(FRAME);

        assert!(physics.velocity(body).unwrap().y > 0.0);
    }

    #[test]
    fn teleporting_moves_and_turns_a_body() {
        let mut physics = Physics2d::new(Vec2::ZERO);
        let body = add(&mut physics, &ball(BodyKind::Dynamic), Vec2::ZERO);

        physics.set_position(body, Vec2::new(3.0, 4.0));
        physics.set_rotation(body, 1.0);

        assert_eq!(physics.position(body), Some(Vec2::new(3.0, 4.0)));
        assert!((physics.rotation(body).unwrap() - 1.0).abs() < TOLERANCE);
    }

    #[test]
    fn time_goes_in_fixed_steps() {
        let mut physics = Physics2d::default();

        assert_eq!(physics.advance(FIXED_STEP / 2.0), 0);
        assert_eq!(physics.advance(FIXED_STEP / 2.0 + 1e-4), 1);
        assert_eq!(physics.advance(FIXED_STEP * 3.0), 3);
        assert_eq!(physics.advance(f32::NAN), 0);
        assert_eq!(physics.advance(-1.0), 0);
    }

    #[test]
    fn a_long_frame_runs_a_bounded_number_of_steps() {
        let mut physics = Physics2d::default();

        assert_eq!(physics.advance(10.0), MAX_STEPS_PER_ADVANCE);
        assert!(physics.advance(0.0) <= 1);
    }

    #[test]
    fn removed_bodies_end_their_contacts_and_ids() {
        let mut physics = Physics2d::default();
        let floor = add(&mut physics, &floor(), Vec2::ZERO);
        let falling = add(&mut physics, &ball(BodyKind::Dynamic), Vec2::new(0.0, 1.0));
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
        let mut physics = Physics2d::default();
        let mut settings = ball(BodyKind::Dynamic);
        assert!(
            physics
                .add_body(&settings, Vec2::new(f32::NAN, 0.0), 0.0)
                .is_err()
        );
        assert!(
            physics
                .add_body(&settings, Vec2::ZERO, f32::INFINITY)
                .is_err()
        );

        settings.friction = -1.0;
        assert!(physics.add_body(&settings, Vec2::ZERO, 0.0).is_err());

        settings = ball(BodyKind::Dynamic);
        settings.density = 0.0;
        assert!(physics.add_body(&settings, Vec2::ZERO, 0.0).is_err());
        assert_eq!(physics.body_count(), 0);
    }

    #[test]
    fn gravity_can_change() {
        let mut physics = Physics2d::default();
        let body = add(&mut physics, &ball(BodyKind::Dynamic), Vec2::ZERO);

        physics.set_gravity(Vec2::new(0.0, 5.0));
        run(&mut physics, 0.5);

        assert_eq!(physics.gravity(), Vec2::new(0.0, 5.0));
        assert!(physics.position(body).unwrap().y > 0.0);
    }
}
