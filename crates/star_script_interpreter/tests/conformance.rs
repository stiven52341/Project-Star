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

/// The programs of the compiler's end-to-end test: every one of them must
/// behave the same interpreted, so a feature cannot reach one side only.
fn programs() -> Vec<PathBuf> {
    let mut programs: Vec<PathBuf> = fs::read_dir(PROGRAMS)
        .expect("the folder of programs")
        .map(|entry| entry.expect("an entry of the folder").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "star")
        })
        .collect();
    programs.sort();
    programs
}

#[test]
fn every_program_behaves_as_compiled() {
    let programs = programs();
    assert!(!programs.is_empty(), "no programs in {PROGRAMS}");
    let failures: Vec<String> = programs
        .iter()
        .filter_map(|path| check(path).err())
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}

/// The expected output (or fault) of `program`, with `\n` line ends.
fn read_expected(program: &Path, extension: &str) -> Option<String> {
    fs::read_to_string(program.with_extension(extension))
        .ok()
        .map(|text| text.replace("\r\n", "\n"))
}

fn check(path: &Path) -> Result<(), String> {
    let name = path
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    let source = fs::read_to_string(path).expect("readable program");
    let checked = check_source(&source).map_err(|errors| format!("{name}: {errors:?}"))?;
    let program = Program::lower(&checked).map_err(|error| format!("{name}: {error}"))?;
    let buffer = Rc::new(RefCell::new(Vec::new()));
    let output: Output = Rc::clone(&buffer) as Output;
    let result = program.run(&output);
    let printed = String::from_utf8_lossy(&buffer.borrow()).into_owned();
    let expected = read_expected(path, "out").unwrap_or_default();
    if printed != expected {
        return Err(format!(
            "{name}: unexpected output\n--- expected\n{expected}--- found\n{printed}"
        ));
    }
    let fault = read_expected(path, "fault").map(|fault| fault.trim().to_owned());
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
