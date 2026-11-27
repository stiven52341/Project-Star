//! Conformance: the interpreter runs the programs that the compiler's
//! end-to-end test builds and runs (`star_script/tests/programs`), and
//! must print the same `.out` and stop with the same `.fault` (rule 3.7 of
//! `CLAUDE.md`), and the crate's example script with the same `main`. A program's `.actions`, if it has one, names the input
//! actions it may read, one per line.

// Test code: a failed step should stop the test with its explanation.
#![allow(clippy::expect_used, clippy::panic)]

use std::cell::RefCell;
use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use star_script::{GameSettings, InputAction, check_source_with};
use star_script_interpreter::{Output, Program, RunError};

/// The stack programs run with: what the main thread of a desktop gets,
/// where the player runs games (test threads get less). Deep calls need it.
const PROGRAM_STACK: usize = 8 * 1024 * 1024;

const PROGRAMS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../star_script/tests/programs");
/// The compiler crate's example script, and the `main` its end-to-end test
/// runs it with.
const EXAMPLE_SCRIPT: &str = include_str!("../../star_script/examples/scripts/player.star");
const EXAMPLE_MAIN: &str = include_str!("../../star_script/tests/example/player_main.star");
const EXAMPLE_OUTPUT: &str = include_str!("../../star_script/tests/example/player_main.out");

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
        .filter_map(|path| on_program_stack(path.clone()).err())
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}

#[test]
fn the_example_script_behaves_as_compiled() {
    let expected = Expected {
        output: EXAMPLE_OUTPUT.replace("\r\n", "\n"),
        fault: None,
    };
    let source = format!("{EXAMPLE_SCRIPT}\n{EXAMPLE_MAIN}");
    if let Err(failure) = run(
        "example_player",
        &source,
        &GameSettings::default(),
        &expected,
    ) {
        panic!("{failure}");
    }
}

/// How a program must behave.
struct Expected {
    output: String,
    /// The runtime error it must stop with, if any.
    fault: Option<String>,
}

/// The expected output (or fault) of `program`, with `\n` line ends.
fn read_expected(program: &Path, extension: &str) -> Option<String> {
    fs::read_to_string(program.with_extension(extension))
        .ok()
        .map(|text| text.replace("\r\n", "\n"))
}

/// The settings of `program`: the actions its `.actions` names, with no
/// bindings, since no one presses them.
fn settings(program: &Path) -> GameSettings {
    let actions = read_expected(program, "actions").unwrap_or_default();
    GameSettings {
        actions: actions
            .lines()
            .map(|name| InputAction {
                name: name.to_owned(),
                bindings: Vec::new(),
            })
            .collect(),
        main_scene: None,
        ..Default::default()
    }
}

/// [`check`] on a thread with the stack of a desktop's main thread.
fn on_program_stack(path: PathBuf) -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(PROGRAM_STACK)
        .spawn(move || check(&path))
        .expect("a thread for the program")
        .join()
        .unwrap_or_else(|_| Err("the program's thread panicked".to_owned()))
}

fn check(path: &Path) -> Result<(), String> {
    let name = path
        .file_stem()
        .map_or_else(String::new, |stem| stem.to_string_lossy().into_owned());
    let source = fs::read_to_string(path).expect("readable program");
    let expected = Expected {
        output: read_expected(path, "out").unwrap_or_default(),
        fault: read_expected(path, "fault").map(|fault| fault.trim().to_owned()),
    };
    run(&name, &source, &settings(path), &expected)
}

/// Runs the program `name` interpreted and compares it with `expected`.
fn run(
    name: &str,
    source: &str,
    settings: &GameSettings,
    expected: &Expected,
) -> Result<(), String> {
    let checked =
        check_source_with(source, settings).map_err(|errors| format!("{name}: {errors:?}"))?;
    let program = Program::lower(&checked).map_err(|error| format!("{name}: {error}"))?;
    let buffer = Rc::new(RefCell::new(Vec::new()));
    let output: Output = Rc::clone(&buffer) as Output;
    let result = program.run(&output);
    let printed = String::from_utf8_lossy(&buffer.borrow()).into_owned();
    if printed != expected.output {
        return Err(format!(
            "{name}: unexpected output\n--- expected\n{}--- found\n{printed}",
            expected.output
        ));
    }
    match (&expected.fault, result) {
        (None, Ok(())) => Ok(()),
        (None, Err(error)) => Err(format!("{name}: failed unexpectedly: {error}")),
        (Some(_), Ok(())) => Err(format!("{name}: should have stopped with an error")),
        (Some(fault), Err(RunError::Fault(error))) if error.message() == fault => Ok(()),
        (Some(fault), Err(error)) => Err(format!(
            "{name}: expected the error `{fault}`, found: {error}"
        )),
    }
}
