//! Blueprints: ProjectStar classes written as node graphs instead of text.
//!
//! A blueprint is one class. Each of its methods is a [`Graph`] of nodes
//! joined by wires: **execution** wires say in which order things happen and
//! **data** wires carry values. Every execution pin takes at most one wire,
//! so the order is always a tree that reads as structured code (`if`/`else`,
//! no jumps), which is how blueprints are translated to StarScript.
//!
//! - [`model`]: what a blueprint is, as plain data the editor can change.
//! - [`Node::pins`]: the pins of each kind of node.
//! - [`validate`]: the rules a blueprint must follow.
//! - [`Blueprint::from_text`] / [`Blueprint::to_text`]: `.blueprint` files.
//!
//! The format is described in `docs/starscript.md`, section 5.

mod file;
pub mod model;
mod pins;
mod validation;

pub use file::{BlueprintFileError, FORMAT_VERSION};
pub use model::{
    Blueprint, Call, CallTarget, Graph, Link, Literal, Node, NodeId, NodeKind, Operator, Parameter,
    PinRef, Variable,
};
pub use pins::{
    BRANCH_FALSE, BRANCH_TRUE, CONDITION, Direction, EXECUTION_INPUT, EXECUTION_OUTPUT, LEFT, Pin,
    PinKind, RESULT, RIGHT, TARGET, VALUE,
};
pub use validation::{BlueprintError, BlueprintErrorKind, validate};
