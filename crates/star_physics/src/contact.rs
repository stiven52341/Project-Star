//! When bodies start and stop touching.

use std::sync::{Mutex, MutexGuard};

use rapier2d::prelude::{
    ColliderHandle, ColliderSet, CollisionEvent, CollisionEventFlags, ContactPair, EventHandler,
    RigidBodySet, SoftBodySet, SoftBodyTearEvent,
};

use crate::BodyId;

/// Whether a [`Contact`] begins or ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactPhase {
    /// The bodies started touching (or overlapping, for a sensor).
    Started,
    /// The bodies stopped touching, or one of them was removed.
    Ended,
}

/// Two bodies that started or stopped touching during a step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contact {
    /// One of the bodies.
    pub first: BodyId,
    /// The other one.
    pub second: BodyId,
    /// Whether it begins or ends.
    pub phase: ContactPhase,
    /// Whether one of them is a sensor, so they overlap instead of
    /// colliding.
    pub is_sensor: bool,
}

impl Contact {
    /// Whether `body` is one of the two.
    #[must_use]
    pub fn involves(&self, body: BodyId) -> bool {
        self.first == body || self.second == body
    }

    /// The body touching `body`, if `body` is one of the two.
    #[must_use]
    pub fn other(&self, body: BodyId) -> Option<BodyId> {
        if self.first == body {
            Some(self.second)
        } else if self.second == body {
            Some(self.first)
        } else {
            None
        }
    }
}

/// A contact as rapier reports it: by colliders, which [`Physics2d`]
/// turns into bodies (a removed body's collider is no longer in rapier's
/// set when its contacts end).
///
/// [`Physics2d`]: crate::Physics2d
#[derive(Debug, Clone, Copy)]
pub(crate) struct ColliderContact {
    pub(crate) first: ColliderHandle,
    pub(crate) second: ColliderHandle,
    pub(crate) phase: ContactPhase,
    pub(crate) is_sensor: bool,
}

/// Collects rapier's collision events during a step. Rapier may call it
/// from several threads, hence the lock.
#[derive(Default)]
pub(crate) struct ContactCollector {
    events: Mutex<Vec<ColliderContact>>,
}

impl ContactCollector {
    /// Moves the contacts collected so far to the end of `contacts`,
    /// keeping both buffers' memory for the next steps.
    pub(crate) fn drain_into(&self, contacts: &mut Vec<ColliderContact>) {
        contacts.append(&mut self.lock());
    }

    fn lock(&self) -> MutexGuard<'_, Vec<ColliderContact>> {
        // A lock poisoned by a panic still holds whole contacts.
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl EventHandler for ContactCollector {
    fn handle_collision_event(
        &self,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        event: CollisionEvent,
        _contact_pair: Option<&ContactPair>,
    ) {
        let (first, second, flags, phase) = match event {
            CollisionEvent::Started(first, second, flags) => {
                (first, second, flags, ContactPhase::Started)
            }
            CollisionEvent::Stopped(first, second, flags) => {
                (first, second, flags, ContactPhase::Ended)
            }
        };
        self.lock().push(ColliderContact {
            first,
            second,
            phase,
            is_sensor: flags.contains(CollisionEventFlags::SENSOR),
        });
    }

    fn handle_contact_force_event(
        &self,
        _dt: f32,
        _bodies: &RigidBodySet,
        _colliders: &ColliderSet,
        _contact_pair: &ContactPair,
        _total_force_magnitude: f32,
    ) {
    }

    fn handle_soft_body_tear_event(&self, _soft_bodies: &SoftBodySet, _event: &SoftBodyTearEvent) {}
}
