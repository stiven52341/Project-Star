//! State machines of animators: the `.states` files that choose which
//! animations of an animator's `.anim` file play (decision 81).
//!
//! An `Animator` whose `states` names a file runs its machine instead of
//! playing one animation: every frame, after its `update`, [`advance`]
//! takes the first transition whose conditions hold, fades from the
//! state's animations to the next state's, and gives the properties the
//! mixed values. Scripts set the values the conditions read
//! ([`set_value`], [`set_flag`], [`trigger`]); `play` jumps to a state.
//! The files are read through the engine ([`set_loader`]), once each.

mod running;
#[cfg(test)]
mod tests;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::nodes::Host;
use crate::tracks::warn;

pub use running::{MachineRun, state};
pub(crate) use running::{advance, jump};

/// The state machine of a `.states` file.
#[derive(Debug, Clone, PartialEq)]
pub struct StateMachine {
    /// The values scripts set, each with a different name.
    pub values: Vec<MachineValue>,
    /// The state the machine starts in, one of `states`.
    pub start: String,
    /// The states, each with a different name.
    pub states: Vec<MachineState>,
    /// The ways from a state to another, tried in this order.
    pub transitions: Vec<Transition>,
}

/// A value of a [`StateMachine`], set by scripts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineValue {
    /// What scripts and conditions name it by.
    pub name: String,
    /// What it holds.
    pub kind: ValueKind,
}

/// What a [`MachineValue`] holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueKind {
    /// A decimal number, zero at the start.
    Number,
    /// `true` or `false`, `false` at the start.
    Flag,
    /// Set by a script for the frame it is set in.
    Trigger,
}

/// A state of a [`StateMachine`].
#[derive(Debug, Clone, PartialEq)]
pub struct MachineState {
    /// What transitions and scripts name it by.
    pub name: String,
    /// What it plays.
    pub motion: Motion,
}

/// What a [`MachineState`] plays.
#[derive(Debug, Clone, PartialEq)]
pub enum Motion {
    /// One animation, by name.
    Animation(String),
    /// Animations blended by where a number value is between their points.
    Blend {
        /// The number value that chooses the blend.
        value: String,
        /// The animations and where each plays alone, in order.
        points: Vec<BlendPoint>,
    },
}

/// An animation of a [`Motion::Blend`] and the value where it plays alone.
#[derive(Debug, Clone, PartialEq)]
pub struct BlendPoint {
    /// The animation, by name.
    pub animation: String,
    /// The value where it plays alone.
    pub at: f32,
}

/// A way from a state to another.
#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    /// The state it leaves, or `None` for any state but `to`.
    pub from: Option<String>,
    /// The state it enters.
    pub to: String,
    /// What must hold to take it; all of them.
    pub conditions: Vec<Condition>,
    /// Seconds fading to the next state's animations; zero to cut.
    pub fade: f32,
    /// Whether it waits for the state to have played once.
    pub at_end: bool,
}

/// Something a [`Transition`] needs of a value.
#[derive(Debug, Clone, PartialEq)]
pub struct Condition {
    /// The value, by name.
    pub value: String,
    /// What it must be.
    pub test: Test,
}

/// What a [`Condition`] tests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Test {
    /// A number above this one.
    Above(f32),
    /// A number below this one.
    Below(f32),
    /// A flag with this value.
    Is(bool),
    /// A trigger set in this frame.
    Triggered,
}

impl StateMachine {
    /// The state called `name`.
    #[must_use]
    pub fn state(&self, name: &str) -> Option<&MachineState> {
        self.states.iter().find(|state| state.name == name)
    }

    fn kind_of(&self, name: &str) -> Option<ValueKind> {
        self.values
            .iter()
            .find(|value| value.name == name)
            .map(|value| value.kind)
    }
}

/// Reads the `.states` file at an asset path, or explains why it cannot.
type Loader = Box<dyn Fn(&str) -> Result<StateMachine, String>>;

/// A read file, or `None` if it could not be read.
type Loaded = Option<Rc<StateMachine>>;

thread_local! {
    static LOADER: RefCell<Option<Loader>> = const { RefCell::new(None) };
    static LOADED: RefCell<HashMap<String, Loaded>> = RefCell::new(HashMap::new());
}

/// Sets how `.states` files are read. Called once, before the game is
/// created.
pub fn set_loader(loader: impl Fn(&str) -> Result<StateMachine, String> + 'static) {
    LOADER.with(|current| {
        if let Ok(mut current) = current.try_borrow_mut() {
            *current = Some(Box::new(loader));
        }
    });
    LOADED.with(|loaded| {
        if let Ok(mut loaded) = loaded.try_borrow_mut() {
            loaded.clear();
        }
    });
}

/// The `.states` file at `path`, read the first time it is needed. `None`
/// if it cannot be read: the reason is written once.
fn load(path: &str) -> Loaded {
    if let Some(loaded) = LOADED.with(|loaded| loaded.borrow().get(path).cloned()) {
        return loaded;
    }
    let result = LOADER.with(|loader| match &*loader.borrow() {
        Some(loader) => loader(path),
        None => Err("state machines can only be loaded while a game runs".to_owned()),
    });
    let loaded = match result {
        Ok(machine) => Some(Rc::new(machine)),
        Err(reason) => {
            warn(&format!("cannot load the states \"{path}\": {reason}"));
            None
        }
    };
    LOADED.with(|cache| {
        cache.borrow_mut().insert(path.to_owned(), loaded.clone());
    });
    loaded
}

/// The machine of the animator `node`, if its `states` names one that can
/// be read.
fn machine_of<H: Host>(node: &H::Node) -> Option<Rc<StateMachine>> {
    H::with_animator(node, |animator| {
        animator
            .states
            .read(|path| if path.is_empty() { None } else { load(path) })
    })
    .flatten()
}

/// Whether the animator `node` runs a state machine: its `states` names a
/// file that can be read.
#[must_use]
pub fn runs_machine<H: Host>(node: &H::Node) -> bool {
    machine_of::<H>(node).is_some()
}

/// `func set_value`: sets the number value `name` of the animator's
/// machine.
pub fn set_value<H: Host>(node: &H::Node, name: &str, value: f32) {
    if expect_kind::<H>(node, name, ValueKind::Number) {
        H::with_animator(node, |animator| {
            animator.machine.borrow_mut().set_number(name, value);
        });
    }
}

/// `func set_flag`: sets the flag `name` of the animator's machine.
pub fn set_flag<H: Host>(node: &H::Node, name: &str, value: bool) {
    if expect_kind::<H>(node, name, ValueKind::Flag) {
        H::with_animator(node, |animator| {
            animator.machine.borrow_mut().set_flag(name, value);
        });
    }
}

/// `func trigger`: sets the trigger `name` of the animator's machine for
/// this frame.
pub fn trigger<H: Host>(node: &H::Node, name: &str) {
    if expect_kind::<H>(node, name, ValueKind::Trigger) {
        H::with_animator(node, |animator| {
            animator.machine.borrow_mut().set_trigger(name);
        });
    }
}

/// Whether the machine of `node` has a value `name` of `kind`; warns once
/// if it has no machine or the value is not one.
fn expect_kind<H: Host>(node: &H::Node, name: &str, kind: ValueKind) -> bool {
    let Some(machine) = machine_of::<H>(node) else {
        warn(&format!(
            "the value \"{name}\" is set on an animator without states"
        ));
        return false;
    };
    match machine.kind_of(name) {
        Some(found) if found == kind => true,
        Some(_) => {
            warn(&format!(
                "the value \"{name}\" of the states is not a {}",
                kind_name(kind)
            ));
            false
        }
        None => {
            warn(&format!("the states have no value \"{name}\""));
            false
        }
    }
}

const fn kind_name(kind: ValueKind) -> &'static str {
    match kind {
        ValueKind::Number => "number",
        ValueKind::Flag => "flag",
        ValueKind::Trigger => "trigger",
    }
}
