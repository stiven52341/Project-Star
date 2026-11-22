//! Compiles a StarScript program to Rust, builds it with Cargo and runs it.
//! The program needs a class with `static func main()`.
//!
//! ```text
//! cargo run -p star_script --example run -- path/to/file.star
//! cargo run -p star_script --example run -- --rust path/to/file.star
//! ```
//!
//! With `--rust` it only prints the generated Rust code.

use std::path::Path;
use std::process::{Command, ExitCode};
use std::{env, fs};

use star_script::package::Package;
use star_script::{LineIndex, compile_source};

const DEFAULT_SCRIPT: &str = "crates/star_script/examples/scripts/hello.star";
const RUNTIME: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../star_script_runtime");
/// Where the generated packages are written, inside the workspace's `target`.
const OUTPUT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/star_script_run");
const SHOW_RUST_FLAG: &str = "--rust";

fn main() -> ExitCode {
    let mut arguments: Vec<String> = env::args().skip(1).collect();
    let show_rust = arguments
        .first()
        .is_some_and(|first| first == SHOW_RUST_FLAG);
    if show_rust {
        arguments.remove(0);
    }
    let path = arguments
        .first()
        .cloned()
        .unwrap_or_else(|| DEFAULT_SCRIPT.to_owned());
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let code = match compile_source(&source) {
        Ok(code) => code,
        Err(errors) => {
            let lines = LineIndex::new(&source);
            for error in &errors {
                eprintln!("{}", error.report(&path, &lines));
            }
            return ExitCode::FAILURE;
        }
    };
    if show_rust {
        print!("{code}");
        return ExitCode::SUCCESS;
    }
    match build_and_run(&program_name(&path), code) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn build_and_run(name: &str, code: String) -> Result<ExitCode, String> {
    let output = Path::new(OUTPUT);
    let directory = output.join(name);
    let mut package = Package::new(name, Path::new(RUNTIME)).map_err(|e| e.to_string())?;
    package.add_program(name, code).map_err(|e| e.to_string())?;
    package
        .write(&directory)
        .map_err(|error| format!("cannot write {}: {error}", directory.display()))?;
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let status = Command::new(cargo)
        .args(["run", "--quiet", "--manifest-path"])
        .arg(directory.join("Cargo.toml"))
        // Every program shares one build of the runtime.
        .env("CARGO_TARGET_DIR", output.join("target"))
        .status()
        .map_err(|error| format!("cannot run cargo: {error}"))?;
    Ok(if status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// A valid package name from the file name: `My Game.star` → `my_game`.
fn program_name(path: &str) -> String {
    let stem = Path::new(path)
        .file_stem()
        .map(|stem| stem.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let name: String = stem
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    if name.starts_with(|c: char| c.is_ascii_alphabetic()) {
        name
    } else {
        format!("star_{name}")
    }
}
