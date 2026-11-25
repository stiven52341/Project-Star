//! Lowering and running small programs.

use star_script::check_source;

use crate::{LowerErrorKind, Program, RunError};

/// What `source` prints when interpreted, or why it stopped.
fn run(source: &str) -> Result<String, RunError> {
    let checked = check_source(source).expect("a valid program");
    let program = Program::lower(&checked).expect("a program the interpreter runs");
    let mut output = Vec::new();
    program.run(&mut output)?;
    Ok(String::from_utf8(output).expect("UTF-8 output"))
}

/// Why `source` cannot be prepared.
fn lowering_error(source: &str) -> LowerErrorKind {
    let checked = check_source(source).expect("a valid program");
    Program::lower(&checked)
        .expect_err("a program the interpreter refuses")
        .kind
}

#[test]
fn main_prints_lines() {
    let source = "class Main {\n static func main() {\n Debug.print(\"Hola\")\n \
                  Debug.print(twice(21).to_string())\n }\n \
                  static func twice(value: int) -> int {\n return value * 2\n }\n}\n";
    assert_eq!(run(source).ok().as_deref(), Some("Hola\n42\n"));
}

#[test]
fn loops_leave_and_skip() {
    let source = "class Main {\n static func main() {\n var total = 0\n \
                  for i in 0..10 {\n if i == 2 { continue }\n if i == 5 { break }\n \
                  total += i\n }\n while total < 100 { total *= 3 }\n \
                  Debug.print(total.to_string())\n }\n}\n";
    // 0 + 1 + 3 + 4 = 8, then 24, 72, 216.
    assert_eq!(run(source).ok().as_deref(), Some("216\n"));
}

#[test]
fn runtime_errors_stop_the_program() {
    let source = "class Main {\n static func main() {\n var zero = 0\n \
                  Debug.print(\"before\")\n Debug.print((1 % zero).to_string())\n }\n}\n";
    let error = run(source).expect_err("a runtime error");
    assert_eq!(
        error.to_string(),
        "StarScript runtime error: integer remainder by zero"
    );
}

#[test]
fn programs_need_one_main() {
    let source = "class Tools {\n static func helper() {\n }\n}\n";
    assert_eq!(lowering_error(source), LowerErrorKind::NoEntryPoint);
}

#[test]
fn what_cannot_run_yet_is_refused_before_running() {
    let constants = "class Main {\n const LIVES = 3\n static func main() {\n \
                     Debug.print(LIVES.to_string())\n }\n}\n";
    assert_eq!(
        lowering_error(constants),
        LowerErrorKind::Unsupported("constants")
    );
    let lists = "class Main {\n static func main() {\n var items = [1, 2]\n }\n}\n";
    assert_eq!(lowering_error(lists), LowerErrorKind::Unsupported("lists"));
}
