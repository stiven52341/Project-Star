//! Compiles a StarScript program to Rust, builds it with Cargo and runs it.
//! The program needs an entry point: a class with `static func main()`, or a
//! game (a class that extends `Game`), which opens a window of the engine.
//!
//! ```text
//! cargo run -p star_script --example run -- path/to/file.star
//! cargo run -p star_script --example run -- --rust path/to/file.star
//! ```
//!
//! With `--rust` it only prints the generated Rust code. If the script's
//! folder has a `project.toml`, its `[input.actions]` are the game's actions.
//! The program runs in the script's folder, so a game finds its images in
//! the `assets` folder next to it, as in a project of the editor.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};
use std::{env, fs};

use star_script::codegen::GeneratedProgram;
use star_script::package::Package;
use star_script::{GameSettings, InputAction, LineIndex, compile_source_with};

const DEFAULT_SCRIPT: &str = "crates/star_script/examples/scripts/hello.star";
/// The `crates` folder of ProjectStar, with the runtime and the engine.
const ENGINE_CRATES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/..");
/// Where the generated packages are written, inside the workspace's `target`.
const OUTPUT: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../target/star_script_run");
const SHOW_RUST_FLAG: &str = "--rust";
/// The project settings file, looked for next to the script.
const PROJECT_FILE: &str = "project.toml";

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
    let settings = match game_settings(Path::new(&path)) {
        Ok(settings) => settings,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let program = match compile_source_with(&source, &settings) {
        Ok(program) => program,
        Err(errors) => {
            let lines = LineIndex::new(&source);
            for error in &errors {
                eprintln!("{}", error.report(&path, &lines));
            }
            return ExitCode::FAILURE;
        }
    };
    if show_rust {
        print!("{}", program.code);
        return ExitCode::SUCCESS;
    }
    let folder = Path::new(&path)
        .parent()
        .filter(|folder| !folder.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    match build_and_run(&program_name(&path), program, folder) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn build_and_run(name: &str, program: GeneratedProgram, folder: &Path) -> Result<ExitCode, String> {
    let output = Path::new(OUTPUT);
    let directory = output.join(name);
    let mut package = Package::new(name, Path::new(ENGINE_CRATES)).map_err(|e| e.to_string())?;
    package
        .add_program(name, program)
        .map_err(|e| e.to_string())?;
    package
        .write(&directory)
        .map_err(|error| format!("cannot write {}: {error}", directory.display()))?;
    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let status = Command::new(cargo)
        .args(["run", "--quiet", "--manifest-path"])
        .arg(directory.join("Cargo.toml"))
        // Every program shares one build of the runtime.
        .env("CARGO_TARGET_DIR", output.join("target"))
        .current_dir(folder)
        .status()
        .map_err(|error| format!("cannot run cargo: {error}"))?;
    Ok(if status.success() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

/// The settings of the project the script belongs to: none if its folder
/// has no `project.toml`.
fn game_settings(script: &Path) -> Result<GameSettings, String> {
    let project = script.parent().map_or_else(
        || PathBuf::from(PROJECT_FILE),
        |folder| folder.join(PROJECT_FILE),
    );
    if !project.is_file() {
        return Ok(GameSettings::default());
    }
    let text = fs::read_to_string(&project)
        .map_err(|error| format!("cannot read {}: {error}", project.display()))?;
    let table: toml::Table = text
        .parse()
        .map_err(|error| format!("{}: {error}", project.display()))?;
    let actions = match table.get("input").and_then(|input| input.get("actions")) {
        None => return Ok(GameSettings::default()),
        Some(toml::Value::Table(actions)) => actions,
        Some(_) => {
            return Err(format!(
                "{}: `input.actions` must be a table",
                project.display()
            ));
        }
    };
    let actions = actions
        .iter()
        .map(|(name, bindings)| input_action(name, bindings))
        .collect::<Result<_, _>>()
        .map_err(|error| format!("{}: {error}", project.display()))?;
    Ok(GameSettings {
        actions,
        main_scene: None,
    })
}

/// `name = ["Space", "GamepadSouth"]`
fn input_action(name: &str, bindings: &toml::Value) -> Result<InputAction, String> {
    let not_a_list = || format!("the action \"{name}\" must be a list of binding names");
    let bindings = bindings
        .as_array()
        .ok_or_else(not_a_list)?
        .iter()
        .map(|binding| binding.as_str().map(str::to_owned).ok_or_else(not_a_list))
        .collect::<Result<_, _>>()?;
    Ok(InputAction {
        name: name.to_owned(),
        bindings,
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
