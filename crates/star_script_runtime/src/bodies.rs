//! StarScript's `Body`: what a node's physics body is, the impulses given
//! to it during a frame and what it touched in the last one.

use std::cell::{Cell, RefCell};

use crate::nodes::Host;
use crate::{Field, List};

/// The friction of a new body's surface.
const FRICTION: f32 = 0.5;
/// The half size of the shapes that do not use it, so they are 1 across.
const UNUSED_RADIUS: f32 = 0.5;

/// The fields of a body: the public ones, the impulses given during the
/// frame, and what it touched in the last one.
pub struct BodyFields<H: Host> {
    /// `kind`: `"dynamic"`, `"static"` or `"kinematic"`.
    pub kind: Field<String>,
    /// `shape`: `"circle"`, `"rectangle"` or `"capsule"` in 2D, `"sphere"`,
    /// `"box"` or `"capsule"` in 3D.
    pub shape: Field<String>,
    /// `radius`
    pub radius: Field<f32>,
    /// `width`
    pub width: Field<f32>,
    /// `height`
    pub height: Field<f32>,
    /// `depth`
    pub depth: Field<f32>,
    /// `gravity_scale`
    pub gravity_scale: Field<f32>,
    /// `friction`
    pub friction: Field<f32>,
    /// `bounciness`
    pub bounciness: Field<f32>,
    /// `density`
    pub density: Field<f32>,
    /// `velocity_x`
    pub velocity_x: Field<f32>,
    /// `velocity_y`
    pub velocity_y: Field<f32>,
    /// `velocity_z`
    pub velocity_z: Field<f32>,
    /// `fixed_rotation`
    pub fixed_rotation: Field<bool>,
    /// `sensor`: it tells what it touches, but nothing bumps into it.
    pub sensor: Field<bool>,
    impulse: Cell<[f32; 3]>,
    touching: Contacts<H>,
    just_touched: Contacts<H>,
    just_separated: Contacts<H>,
}

/// Nodes a body touches, without keeping them alive.
type Contacts<H> = RefCell<Vec<<H as Host>::WeakNode>>;

/// Which of the nodes a body touches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Contact {
    /// Those it touches.
    Touching,
    /// Those it began to touch in the last frame.
    JustTouched,
    /// Those it stopped touching in the last frame.
    JustSeparated,
}

impl<H: Host> BodyFields<H> {
    /// `Body.circle`
    #[must_use]
    pub fn circle(kind: String, radius: f32) -> Self {
        Self::new(kind, "circle", radius, [1.0, 1.0, 1.0])
    }

    /// `Body.rectangle`
    #[must_use]
    pub fn rectangle(kind: String, width: f32, height: f32) -> Self {
        Self::new(kind, "rectangle", UNUSED_RADIUS, [width, height, 1.0])
    }

    /// `Body.capsule`
    #[must_use]
    pub fn capsule(kind: String, height: f32, radius: f32) -> Self {
        Self::new(kind, "capsule", radius, [1.0, height, 1.0])
    }

    /// `Body.sphere`
    #[must_use]
    pub fn sphere(kind: String, radius: f32) -> Self {
        Self::new(kind, "sphere", radius, [1.0, 1.0, 1.0])
    }

    /// `Body.box`
    #[must_use]
    pub fn cuboid(kind: String, width: f32, height: f32, depth: f32) -> Self {
        Self::new(kind, "box", UNUSED_RADIUS, [width, height, depth])
    }

    /// A body of `kind` and `shape` at rest, with the engine's surface;
    /// `size` is its width, height and depth.
    #[must_use]
    pub fn new(kind: String, shape: &str, radius: f32, [width, height, depth]: [f32; 3]) -> Self {
        Self {
            kind: Field::new(kind),
            shape: Field::new(shape.to_owned()),
            radius: Field::new(radius),
            width: Field::new(width),
            height: Field::new(height),
            depth: Field::new(depth),
            gravity_scale: Field::new(1.0),
            friction: Field::new(FRICTION),
            bounciness: Field::new(0.0),
            density: Field::new(1.0),
            velocity_x: Field::new(0.0),
            velocity_y: Field::new(0.0),
            velocity_z: Field::new(0.0),
            fixed_rotation: Field::new(false),
            sensor: Field::new(false),
            impulse: Cell::new([0.0; 3]),
            touching: RefCell::new(Vec::new()),
            just_touched: RefCell::new(Vec::new()),
            just_separated: RefCell::new(Vec::new()),
        }
    }

    /// `func apply_impulse_3d` (and `apply_impulse`, with `z` 0): pushes
    /// the body once the frame ends.
    pub fn apply_impulse(&self, x: f32, y: f32, z: f32) {
        let [old_x, old_y, old_z] = self.impulse.get();
        self.impulse.set([old_x + x, old_y + y, old_z + z]);
    }

    /// The impulses given since the last call, added up; they are
    /// forgotten, as the physics receives them once.
    pub fn take_impulse(&self) -> [f32; 3] {
        self.impulse.replace([0.0; 3])
    }

    /// `velocity` as an array.
    #[must_use]
    pub fn velocity(&self) -> [f32; 3] {
        [
            self.velocity_x.get(),
            self.velocity_y.get(),
            self.velocity_z.get(),
        ]
    }

    /// Sets how fast the body moves, as the physics computed it.
    pub fn set_velocity(&self, [x, y, z]: [f32; 3]) {
        self.velocity_x.set(x);
        self.velocity_y.set(y);
        self.velocity_z.set(z);
    }

    /// `func touching`, `just_touched` or `just_separated`: a new list
    /// with the nodes of `which` that still exist.
    #[must_use]
    pub fn contacts(&self, which: Contact) -> List<H::Node> {
        let nodes = self
            .list(which)
            .borrow()
            .iter()
            .filter_map(H::upgrade)
            .collect();
        List::from_vec(nodes)
    }

    /// Replaces the nodes of `which`, as the physics found them.
    pub fn set_contacts<'node>(
        &self,
        which: Contact,
        nodes: impl IntoIterator<Item = &'node H::Node>,
    ) {
        let mut list = self.list(which).borrow_mut();
        list.clear();
        list.extend(nodes.into_iter().map(H::downgrade));
    }

    const fn list(&self, which: Contact) -> &Contacts<H> {
        match which {
            Contact::Touching => &self.touching,
            Contact::JustTouched => &self.just_touched,
            Contact::JustSeparated => &self.just_separated,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::rc::Rc;

    use super::*;
    use crate::nodes::NodeFields;
    use crate::nodes::tests::{TestHost, TestNode};

    fn node(name: &str) -> Rc<TestNode> {
        Rc::new(TestNode::flat(NodeFields::new(name.to_owned())))
    }

    #[test]
    fn shapes_take_their_sizes() {
        let rectangle = BodyFields::<TestHost>::rectangle("static".to_owned(), 4.0, 2.0);
        let cuboid = BodyFields::<TestHost>::cuboid("dynamic".to_owned(), 1.0, 2.0, 3.0);

        assert_eq!(rectangle.shape.get(), "rectangle");
        assert!((rectangle.width.get() - 4.0).abs() < f32::EPSILON);
        assert_eq!(cuboid.shape.get(), "box");
        assert!((cuboid.depth.get() - 3.0).abs() < f32::EPSILON);
        assert!((cuboid.friction.get() - FRICTION).abs() < f32::EPSILON);
    }

    #[test]
    fn impulses_add_up_until_taken() {
        let body = BodyFields::<TestHost>::circle("dynamic".to_owned(), 0.5);

        body.apply_impulse(1.0, 0.0, 0.0);
        body.apply_impulse(1.0, 2.0, 0.0);

        assert_eq!(body.take_impulse(), [2.0, 2.0, 0.0]);
        assert_eq!(body.take_impulse(), [0.0; 3]);
    }

    #[test]
    fn contacts_only_list_nodes_still_alive() {
        let body = BodyFields::<TestHost>::circle("dynamic".to_owned(), 0.5);
        let (wall, coin) = (node("wall"), node("coin"));

        body.set_contacts(Contact::Touching, [&wall, &coin]);
        drop(coin);

        let touching = body.contacts(Contact::Touching).to_vec();
        assert_eq!(touching.len(), 1);
        assert!(Rc::ptr_eq(&touching[0], &wall));
        assert!(body.contacts(Contact::JustTouched).is_empty());
    }

    #[test]
    fn the_velocity_is_set_at_once() {
        let body = BodyFields::<TestHost>::sphere("dynamic".to_owned(), 1.0);

        body.set_velocity([1.0, 2.0, 3.0]);

        assert_eq!(body.velocity(), [1.0, 2.0, 3.0]);
    }
}
