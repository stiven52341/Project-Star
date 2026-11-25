//! A blueprint with every kind of node keeps its conformance program up
//! to date (decision 73): `star_script/tests/programs/blueprint_every_node.star`
//! starts with its translation, so the compiled end-to-end test and the
//! interpreter's conformance test run it both ways. The templates of the
//! editor only use a few kinds of node; the editor checks theirs.

// Test code: a failed step should stop the test with its explanation.
#![allow(clippy::expect_used, clippy::panic)]

use star_blueprint::{Blueprint, translate, validate};

const EVERY_NODE: &str = include_str!("every_node.blueprint");
const PROGRAM: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../star_script/tests/programs/blueprint_every_node.star"
);

#[test]
fn the_blueprint_with_every_node_has_its_conformance_program() {
    let blueprint = Blueprint::from_text(EVERY_NODE).expect("a valid file");
    assert_eq!(validate(&blueprint), Ok(()));
    let translation = translate(&blueprint).expect("translatable");
    let program = std::fs::read_to_string(PROGRAM).unwrap_or_default();
    assert!(
        program.starts_with(translation.source()),
        "{PROGRAM} must start with the translation of the blueprint:\n{}",
        translation.source()
    );
}
