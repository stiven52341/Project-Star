//! Support code for the Rust generated from StarScript.
//!
//! Generated games depend on this crate; StarScript programmers never see
//! it. It provides what the language needs beyond plain Rust: shared lists,
//! object fields that can change through shared references, equality as
//! StarScript defines it, the state of the game's input actions, the scenes
//! a game loads, the sounds it plays, coroutines (`async` methods), `parallel` functions, and the few
//! runtime errors that cannot be caught at compile time.
//!
//! It has no dependencies, so a game built from StarScript only needs the
//! Rust toolchain and the engine.

pub mod audio;
pub mod bodies;
pub mod color;
mod equality;
mod fault;
mod field;
pub mod input;
mod list;
pub mod node_classes;
pub mod nodes;
mod operations;
pub mod parallel;
pub mod placement;
mod plain;
pub mod scene_fields;
pub mod scenes;
pub mod tasks;

pub use equality::StarEq;
pub use fault::{fault, proven};
pub use field::Field;
pub use list::List;
pub use operations::{
    Integer, concat, float_to_string, int_divide, int_remainder, print, text_length,
};
pub use plain::Plain;
pub use tasks::Task;
