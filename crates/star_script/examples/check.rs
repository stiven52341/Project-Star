//! Checks a StarScript file and prints its classes with every type resolved,
//! or its errors.
//!
//! ```text
//! cargo run -p star_script --example check -- path/to/file.star
//! ```

use std::process::ExitCode;
use std::{env, fs};

use star_script::semantic::{ClassInfo, ClassOrigin, ClassTable, ParameterInfo};
use star_script::{LineIndex, check_source};

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
    match check_source(&source) {
        Ok(checked) => {
            let classes = checked.analysis.classes();
            for (_, class) in classes.iter() {
                if matches!(class.origin, ClassOrigin::Script { .. }) {
                    print_class(classes, class);
                }
            }
            println!("no errors");
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

fn print_class(classes: &ClassTable, class: &ClassInfo) {
    let base = class
        .base
        .map(|base| format!(" extends {}", classes.class(base).name))
        .unwrap_or_default();
    println!("class {}{base}", class.name);
    for constant in &class.constants {
        println!(
            "  const {}: {}",
            constant.name,
            classes.type_name(&constant.ty)
        );
    }
    for field in &class.fields {
        println!("  var {}: {}", field.name, classes.type_name(&field.ty));
    }
    if let Some(constructor) = &class.constructor {
        println!("  init({})", parameters(classes, &constructor.parameters));
    }
    for method in &class.methods {
        println!(
            "  func {}({}) -> {}",
            method.name,
            parameters(classes, &method.parameters),
            classes.type_name(&method.return_type)
        );
    }
}

fn parameters(classes: &ClassTable, parameters: &[ParameterInfo]) -> String {
    parameters
        .iter()
        .map(|parameter| format!("{}: {}", parameter.name, classes.type_name(&parameter.ty)))
        .collect::<Vec<_>>()
        .join(", ")
}
