//! When bodies start and stop touching.

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Mutex, MutexGuard};

use crate::BodyId;

/// Whether a [`Contact`] begins or ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContactPhase {
    /// The bodies started touching (or overlapping, for a sensor).
    Started,
    /// The bodies stopped touching, or one of them was removed.
    Ended,
}

/// Two bodies that started or stopped touching during a step: of a
/// [`Physics2d`](crate::Physics2d) by default, of a
/// [`Physics3d`](crate::Physics3d) as `Contact<BodyId3d>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contact<Body = BodyId> {
    /// One of the bodies.
    pub first: Body,
    /// The other one.
    pub second: Body,
    /// Whether it begins or ends.
    pub phase: ContactPhase,
    /// Whether one of them is a sensor, so they overlap instead of
    /// colliding.
    pub is_sensor: bool,
}

impl<Body: Copy + PartialEq> Contact<Body> {
    /// Whether `body` is one of the two.
    #[must_use]
    pub fn involves(&self, body: Body) -> bool {
        self.first == body || self.second == body
    }

    /// The body touching `body`, if `body` is one of the two.
    #[must_use]
    pub fn other(&self, body: Body) -> Option<Body> {
        if self.first == body {
            Some(self.second)
        } else if self.second == body {
            Some(self.first)
        } else {
            None
        }
    }
}

/// A contact as rapier reports it: by colliders, which [`ContactLog`]
/// turns into bodies (a removed body's collider is no longer in rapier's
/// set when its contacts end).
#[derive(Debug, Clone, Copy)]
struct ColliderContact<Collider> {
    first: Collider,
    second: Collider,
    phase: ContactPhase,
    is_sensor: bool,
}

/// Collects rapier's collision events during a step. Rapier may call it
/// from several threads, hence the lock.
pub(crate) struct ContactCollector<Collider> {
    events: Mutex<Vec<ColliderContact<Collider>>>,
}

impl<Collider> ContactCollector<Collider> {
    fn new() -> Self {
        Self {
            events: Mutex::new(Vec::new()),
        }
    }

    /// Records one event of rapier.
    fn record(&self, first: Collider, second: Collider, started: bool, is_sensor: bool) {
        let phase = if started {
            ContactPhase::Started
        } else {
            ContactPhase::Ended
        };
        self.lock().push(ColliderContact {
            first,
            second,
            phase,
            is_sensor,
        });
    }

    fn lock(&self) -> MutexGuard<'_, Vec<ColliderContact<Collider>>> {
        // A lock poisoned by a panic still holds whole contacts.
        self.events
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// The contacts of a simulation, by body: which body owns each collider
/// (one per body), and the contacts of the steps of the last advance.
pub(crate) struct ContactLog<Collider, Body> {
    collector: ContactCollector<Collider>,
    /// The body of each collider, kept after the body is removed until its
    /// last contacts are reported.
    owners: HashMap<Collider, Body>,
    removed: Vec<Collider>,
    pending: Vec<ColliderContact<Collider>>,
    contacts: Vec<Contact<Body>>,
}

impl<Collider: Copy + Eq + Hash, Body: Copy> ContactLog<Collider, Body> {
    pub(crate) fn new() -> Self {
        Self {
            collector: ContactCollector::new(),
            owners: HashMap::new(),
            removed: Vec::new(),
            pending: Vec::new(),
            contacts: Vec::new(),
        }
    }

    /// What rapier reports its events to during a step.
    pub(crate) const fn collector(&self) -> &ContactCollector<Collider> {
        &self.collector
    }

    /// Records that `collider` belongs to `body`.
    pub(crate) fn add_owner(&mut self, collider: Collider, body: Body) {
        self.owners.insert(collider, body);
    }

    /// Records that the body of `colliders` was removed: they are forgotten
    /// after the next step, which ends their contacts.
    pub(crate) fn remove_owners(&mut self, colliders: &[Collider]) {
        self.removed.extend_from_slice(colliders);
    }

    /// Forgets the contacts of the last advance.
    pub(crate) fn clear(&mut self) {
        self.contacts.clear();
    }

    /// Turns the events of the step that just ran into contacts between
    /// bodies, keeping every buffer's memory for the next steps.
    pub(crate) fn collect_step(&mut self) {
        self.pending.append(&mut self.collector.lock());
        for contact in self.pending.drain(..) {
            let owner = |collider| self.owners.get(&collider).copied();
            if let (Some(first), Some(second)) = (owner(contact.first), owner(contact.second)) {
                self.contacts.push(Contact {
                    first,
                    second,
                    phase: contact.phase,
                    is_sensor: contact.is_sensor,
                });
            }
        }
        // The contacts of removed bodies ended during this step.
        for collider in self.removed.drain(..) {
            self.owners.remove(&collider);
        }
    }

    /// The contacts since the last [`clear`](Self::clear).
    pub(crate) fn contacts(&self) -> &[Contact<Body>] {
        &self.contacts
    }
}

/// Implements rapier's `EventHandler` of one flavor (2D or 3D) for the
/// [`ContactCollector`] of its colliders.
macro_rules! collect_rapier_events {
    ($rapier:ident) => {
        impl $rapier::prelude::EventHandler for ContactCollector<$rapier::prelude::ColliderHandle> {
            fn handle_collision_event(
                &self,
                _bodies: &$rapier::prelude::RigidBodySet,
                _colliders: &$rapier::prelude::ColliderSet,
                event: $rapier::prelude::CollisionEvent,
                _contact_pair: Option<&$rapier::prelude::ContactPair>,
            ) {
                use $rapier::prelude::{CollisionEvent, CollisionEventFlags};
                let (first, second, flags, started) = match event {
                    CollisionEvent::Started(first, second, flags) => (first, second, flags, true),
                    CollisionEvent::Stopped(first, second, flags) => (first, second, flags, false),
                };
                self.record(
                    first,
                    second,
                    started,
                    flags.contains(CollisionEventFlags::SENSOR),
                );
            }

            fn handle_contact_force_event(
                &self,
                _dt: f32,
                _bodies: &$rapier::prelude::RigidBodySet,
                _colliders: &$rapier::prelude::ColliderSet,
                _contact_pair: &$rapier::prelude::ContactPair,
                _total_force_magnitude: f32,
            ) {
            }

            fn handle_soft_body_tear_event(
                &self,
                _soft_bodies: &$rapier::prelude::SoftBodySet,
                _event: &$rapier::prelude::SoftBodyTearEvent,
            ) {
            }
        }
    };
}

collect_rapier_events!(rapier2d);
collect_rapier_events!(rapier3d);
