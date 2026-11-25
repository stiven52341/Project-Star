//! Conformance: the interpreter runs the programs that the compiler's
//! end-to-end test builds and runs (`star_script/tests/programs`), and
//! must print the same `.out` and stop with the same `.fault` (rule 3.7 of
//! `CLAUDE.md`).

// Test code: a failed step should stop the test with its explanation.
#![allow(clippy::expect_used, clippy::panic)]

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use star_script::check_source;
use star_script_interpreter::{Output, Program, RunError};

const PROGRAMS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../star_script/tests/programs");

/// The programs the interpreter covers so far: each step of Phase 14 of
/// `docs/roadmap.md` adds the ones its part of the language completes.
const COVERED: [&str; 8] = [
    "basics",
    "classes",
    "coroutines",
    "decorators",
    "division_fault",
    "functions",
    "index_fault",
    "objects",
];

#[test]
fn covered_programs_behave_as_compiled() {
    let failures: Vec<String> = COVERED
        .iter()
        .filter_map(|name| check(name).err())
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}

#[test]
fn every_covered_program_exists() {
    for name in COVERED {
        assert!(program(name).is_file(), "missing program {name}");
    }
}

fn program(name: &str) -> PathBuf {
    Path::new(PROGRAMS).join(name).with_extension("star")
}

/// The expected output (or fault) of `program`, with `\n` line ends.
fn read_expected(program: &Path, extension: &str) -> Option<String> {
    fs::read_to_string(program.with_extension(extension))
        .ok()
        .map(|text| text.replace("\r\n", "\n"))
}

fn check(name: &str) -> Result<(), String> {
    let path = program(name);
    let source = fs::read_to_string(&path).expect("readable program");
    let checked = check_source(&source).map_err(|errors| format!("{name}: {errors:?}"))?;
    let program = Program::lower(&checked).map_err(|error| format!("{name}: {error}"))?;
    let buffer = Rc::new(RefCell::new(Vec::new()));
    let output: Output = Rc::clone(&buffer) as Output;
    let result = program.run(&output);
    let printed = String::from_utf8_lossy(&buffer.borrow()).into_owned();
    let expected = read_expected(&path, "out").unwrap_or_default();
    if printed != expected {
        return Err(format!(
            "{name}: unexpected output\n--- expected\n{expected}--- found\n{printed}"
        ));
    }
    let fault = read_expected(&path, "fault").map(|fault| fault.trim().to_owned());
    match (fault, result) {
        (None, Ok(())) => Ok(()),
        (None, Err(error)) => Err(format!("{name}: failed unexpectedly: {error}")),
        (Some(_), Ok(())) => Err(format!("{name}: should have stopped with an error")),
        (Some(fault), Err(RunError::Fault(error))) if error.message() == fault => Ok(()),
        (Some(fault), Err(error)) => Err(format!(
            "{name}: expected the error `{fault}`, found: {error}"
        )),
    }
}
