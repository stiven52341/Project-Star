//! What the end-to-end tests share: compiling StarScript, building the
//! generated package with Cargo and finding its programs.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use star_script::codegen::GeneratedProgram;
use star_script::{GameSettings, compile_source_with};

/// The `crates` folder of ProjectStar, with the runtime and the engine.
pub const ENGINE_CRATES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");

/// Where a test writes its package: in Cargo's folder for test files.
/// Programs written by an earlier version of the test are removed, or they
/// would be built too; its build is kept, to be reused.
pub fn package_directory(name: &str) -> PathBuf {
    let directory = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    // Missing the first time.
    let _ = std::fs::remove_dir_all(directory.join("src"));
    directory
}

pub fn compile_with(name: &str, source: &str, settings: &GameSettings) -> GeneratedProgram {
    compile_source_with(source, settings).unwrap_or_else(|errors| {
        let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
        panic!("{name} does not compile: {messages:#?}")
    })
}

/// Builds the package in `directory`, failing the test if it does not build.
pub fn build(directory: &Path) {
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args(["build", "--quiet", "--manifest-path"])
        .arg(directory.join("Cargo.toml"))
        .env("CARGO_TARGET_DIR", directory.join("target"))
        .output()
        .expect("cargo runs");
    assert!(
        output.status.success(),
        "the generated Rust does not build:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// The executable of the program `name`, once built.
pub fn binary(directory: &Path, name: &str) -> PathBuf {
    directory
        .join("target")
        .join("debug")
        .join(format!("{name}{}", env::consts::EXE_SUFFIX))
}
