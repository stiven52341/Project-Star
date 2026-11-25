//! The package of a project played compiled (decision 75): its generated
//! Rust as a binary of its own, built in the player's target folder, so
//! the engine the warm-up built for the player is reused and only the
//! game's code is compiled.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use star_script::codegen::GeneratedProgram;
use star_script::package::Package;

use super::{BUILD_DIRECTORY, ENGINE_CRATES, ENGINE_LOCKFILE};

/// Name of the package of a project whose name has no letter to keep.
const FALLBACK_PACKAGE_NAME: &str = "game";

/// Writes the package of the project called `project` with `program`
/// (only what changed) and gives its folder and the name of its program.
///
/// # Errors
///
/// Returns the error reading the engine's lockfile or writing the files.
pub(super) fn write_game(
    project: &str,
    program: &GeneratedProgram,
) -> io::Result<(PathBuf, String)> {
    let name = package_name(project);
    let directory = Path::new(BUILD_DIRECTORY).join(&name);
    write_package(&name, program, Path::new(ENGINE_CRATES), &directory)?;
    Ok((directory, name))
}

/// Writes the package `name`, with `program` as its only binary, in
/// `directory`, using the engine crates in `engine`.
fn write_package(
    name: &str,
    program: &GeneratedProgram,
    engine: &Path,
    directory: &Path,
) -> io::Result<()> {
    let lockfile = fs::read_to_string(ENGINE_LOCKFILE)?;
    let mut package = Package::new(name, engine)?.with_lockfile(lockfile);
    package.add_program(name, program.clone())?;
    package.write(directory)
}

/// A valid package name from a project's name: `My Game` → `my_game`. It
/// never starts with `_`, so it is never the player's.
pub(super) fn package_name(project: &str) -> String {
    let name: String = project
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        name
    } else if name.chars().any(|c| c.is_ascii_alphanumeric()) {
        format!("{FALLBACK_PACKAGE_NAME}_{name}")
    } else {
        FALLBACK_PACKAGE_NAME.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn package_names_come_from_the_project_name() {
        assert_eq!(package_name("My Game"), "my_game");
        assert_eq!(package_name("2048"), "game_2048");
        assert_eq!(package_name("¡Ñ!"), "game");
    }

    #[test]
    fn games_share_the_engine_lockfile() {
        let package = Path::new(BUILD_DIRECTORY).join("editor_test_lockfile");
        let program = star_script::compile_source("class Main {\n static func main() {}\n}\n")
            .expect("valid program");

        write_package(
            "editor_test_lockfile",
            &program,
            Path::new(ENGINE_CRATES),
            &package,
        )
        .expect("written");

        let written = fs::read_to_string(package.join("Cargo.lock")).expect("lockfile");
        let engine = fs::read_to_string(ENGINE_LOCKFILE).expect("engine lockfile");
        assert_eq!(written, engine);
    }
}
