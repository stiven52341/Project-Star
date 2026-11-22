//! Prints the syntax tree of a StarScript file, or its errors.
//!
//! ```text
//! cargo run -p star_script --example parse -- path/to/file.star
//! ```

use std::process::ExitCode;
use std::{env, fs};

use star_script::{LineIndex, ast, parse_source};

const DEFAULT_SCRIPT: &str = "crates/star_script/examples/scripts/player.star";

fn main() -> ExitCode {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| DEFAULT_SCRIPT.to_owned());
    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    match parse_source(&source) {
        Ok(file) => {
            print!("{}", ast::outline(&file));
            ExitCode::SUCCESS
        }
        Err(errors) => {
            let lines = LineIndex::new(&source);
            for error in &errors {
                eprintln!("{}", error.report(&path, &lines));
            }
            eprintln!("{} errors", errors.len());
            ExitCode::FAILURE
        }
    }
}
