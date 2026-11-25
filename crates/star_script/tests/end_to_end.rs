//! End-to-end tests: StarScript programs compiled to Rust, built with Cargo
//! and run, comparing what they print with what is expected.
//!
//! Each `programs/NAME.star` comes with `NAME.out`, its expected output, and
//! optionally `NAME.fault`: the runtime error it must stop with, and
//! `NAME.actions`: the input actions it may read, one per line.

// Test code: a failed step should stop the test with its explanation.
#![allow(clippy::expect_used, clippy::panic)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use common::{ENGINE_CRATES, binary, build, compile_with, package_directory};
use star_script::package::Package;
use star_script::{GameSettings, InputAction};

const PROGRAMS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/programs");
const EXAMPLE_SCRIPT: &str = include_str!("../examples/scripts/player.star");

/// A program and how it must behave.
struct Expectation {
    name: String,
    output: String,
    fault: Option<String>,
}

#[test]
fn programs_behave_as_expected() {
    let directory = package_directory("star_script_end_to_end");
    // Programs removed since the last run must not linger.
    let _ = fs::remove_dir_all(directory.join("src"));
    let mut package = Package::new("end_to_end", Path::new(ENGINE_CRATES)).expect("valid name");
    let mut expectations = Vec::new();
    for path in star_files() {
        let name = file_stem(&path);
        let source = fs::read_to_string(&path).expect("readable program");
        package
            .add_program(&name, compile_with(&name, &source, &settings(&path)))
            .expect("valid program name");
        expectations.push(Expectation {
            output: read_expected(&path, "out").unwrap_or_default(),
            fault: read_expected(&path, "fault").map(|fault| fault.trim().to_owned()),
            name,
        });
    }
    let (name, source, output) = example_program();
    package
        .add_program(name, compile_with(name, &source, &GameSettings::default()))
        .expect("valid program name");
    expectations.push(Expectation {
        name: name.to_owned(),
        output: output.replace("\r\n", "\n"),
        fault: None,
    });
    package.write(&directory).expect("package written");

    build(&directory);

    let failures: Vec<String> = expectations
        .iter()
        .filter_map(|expectation| run(&directory, expectation).err())
        .collect();
    assert!(failures.is_empty(), "\n{}", failures.join("\n\n"));
}

/// The example script of the crate, with a `main` that uses it, which the
/// interpreter's conformance test runs too.
fn example_program() -> (&'static str, String, &'static str) {
    let main = include_str!("example/player_main.star");
    (
        "example_player",
        format!("{EXAMPLE_SCRIPT}\n{main}"),
        include_str!("example/player_main.out"),
    )
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
    }
}

fn star_files() -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = fs::read_dir(PROGRAMS)
        .expect("programs folder")
        .map(|entry| entry.expect("readable entry").path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "star")
        })
        .collect();
    files.sort();
    files
}

fn file_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .expect("UTF-8 file name")
        .to_owned()
}

/// The expected output (or fault) of `program`, with `\n` line ends: Git
/// may check the files out with `\r\n` on Windows, but programs print `\n`
/// on every system.
fn read_expected(program: &Path, extension: &str) -> Option<String> {
    fs::read_to_string(program.with_extension(extension))
        .ok()
        .map(|text| text.replace("\r\n", "\n"))
}

fn run(directory: &Path, expectation: &Expectation) -> Result<(), String> {
    let name = &expectation.name;
    let binary = binary(directory, name);
    let output = Command::new(&binary)
        .output()
        .map_err(|error| format!("{name}: cannot run {}: {error}", binary.display()))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if stdout != expectation.output {
        return Err(format!(
            "{name}: unexpected output\n--- expected\n{}--- found\n{stdout}--- errors\n{stderr}",
            expectation.output
        ));
    }
    match (&expectation.fault, output.status.success()) {
        (None, true) => Ok(()),
        (None, false) => Err(format!("{name}: failed unexpectedly:\n{stderr}")),
        (Some(_), true) => Err(format!("{name}: should have stopped with an error")),
        (Some(fault), false) if stderr.contains(&format!("StarScript runtime error: {fault}")) => {
            Ok(())
        }
        (Some(fault), false) => Err(format!(
            "{name}: expected the error `{fault}`, found:\n{stderr}"
        )),
    }
}
