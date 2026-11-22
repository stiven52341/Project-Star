//! Prints the tokens of a StarScript file, or its lexical errors.
//!
//! ```text
//! cargo run -p star_script --example tokenize -- path/to/file.star
//! ```

use std::process::ExitCode;
use std::{env, fs};

use star_script::{CompileError, LineIndex, TokenKind, tokenize};

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
    let lines = LineIndex::new(&source);
    match tokenize(&source) {
        Ok(tokens) => {
            for token in &tokens {
                let position = lines.position(token.span.start());
                let separator = if token.kind == TokenKind::Newline {
                    "\n"
                } else {
                    ""
                };
                println!(
                    "{:>3}:{:<3} {}{separator}",
                    position.line, position.column, token.kind
                );
            }
            println!("{} tokens, no errors", tokens.len());
            ExitCode::SUCCESS
        }
        Err(errors) => {
            for error in errors.iter().cloned().map(CompileError::from) {
                eprintln!("{}", error.report(&path, &lines));
            }
            eprintln!("{} errors", errors.len());
            ExitCode::FAILURE
        }
    }
}
