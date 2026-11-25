//! Lowering and running small programs.

use std::cell::RefCell;
use std::rc::Rc;

use star_script::check_source;

use crate::{LowerErrorKind, Output, Program, RunError};

/// What `source` prints when interpreted, or why it stopped.
fn run(source: &str) -> Result<String, RunError> {
    let checked = check_source(source).expect("a valid program");
    let program = Program::lower(&checked).expect("a program the interpreter runs");
    let (buffer, output) = buffer();
    program.run(&output)?;
    Ok(String::from_utf8(buffer.take()).expect("UTF-8 output"))
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
    let node = "class Player extends Node {\n init() {\n super.init(\"player\")\n }\n}\nclass Main {\n static func main() {\n }\n}\n";
    assert_eq!(
        lowering_error(node),
        LowerErrorKind::Unsupported("classes that extend engine classes")
    );
}

#[test]
fn subclasses_override_and_call_their_base() {
    // The subclass is written first: tables are built bases first.
    let source = "class Loud extends Quiet {\n var volume: int\n \
                  init(level: int) {\n super.init(\"loud\")\n volume = level\n }\n \
                  override func say() -> string {\n return super.say() + \"!\" + volume.to_string()\n }\n}\n\
                  class Quiet {\n var word: string\n init(word: string) {\n self.word = word\n }\n \
                  func say() -> string {\n return word\n }\n}\n\
                  class Main {\n static func main() {\n const voice: Quiet = Loud(3)\n \
                  Debug.print(voice.say())\n Debug.print((voice is Loud).to_string())\n \
                  const quiet = Quiet(\"shh\")\n Debug.print(((quiet as Loud) == null).to_string())\n }\n}\n";
    assert_eq!(run(source).ok().as_deref(), Some("loud!3\ntrue\ntrue\n"));
}

#[test]
fn lists_are_shared_and_nulls_fall_back() {
    let source = "class Main {\n const SIZE = 2 * 3\n static func main() {\n \
                  var list = [1, 2]\n const alias = list\n alias.push(SIZE)\n \
                  Debug.print(list.length().to_string())\n \
                  var maybe: [int]? = null\n Debug.print((maybe?.length() ?? -1).to_string())\n \
                  for value in list {\n list.clear()\n Debug.print(value.to_string())\n }\n \
                  Debug.print(list.is_empty().to_string())\n }\n}\n";
    // The loop goes through the elements the list had when it started.
    assert_eq!(run(source).ok().as_deref(), Some("3\n-1\n1\n2\n6\ntrue\n"));
}

#[test]
fn objects_are_built_in_two_phases() {
    let source = "class Box {\n var label: string\n var size = 1\n \
                  init(label: string) {\n var upper = label.to_upper()\n \
                  self.label = upper\n grow()\n Debug.print(upper + \" \" + size.to_string())\n }\n \
                  private func grow() {\n size *= 3\n }\n}\n\
                  class Main {\n static func main() {\n const box = Box(\"a\")\n \
                  box.size += 1\n Debug.print(box.label + box.size.to_string())\n }\n}\n";
    // `upper`, declared before the object exists, is still read after it.
    assert_eq!(run(source).ok().as_deref(), Some("A 3\nA4\n"));
}

#[test]
fn reading_past_the_end_is_a_fault() {
    let source = "class Main {\n static func main() {\n var list = [1]\n \
                  list[0] += 1\n Debug.print(list[0].to_string())\n list[1] = 0\n }\n}\n";
    let error = run_with_output(source);
    assert_eq!(error.0, "2\n");
    assert_eq!(
        error.1.map(|error| error.to_string()),
        Some("StarScript runtime error: index 1 is out of range for a list of length 1".to_owned())
    );
}

/// What `source` prints, and why it stopped if it did not end.
fn run_with_output(source: &str) -> (String, Option<RunError>) {
    let checked = check_source(source).expect("a valid program");
    let program = Program::lower(&checked).expect("a program the interpreter runs");
    let (buffer, output) = buffer();
    let result = program.run(&output);
    (
        String::from_utf8(buffer.take()).expect("UTF-8 output"),
        result.err(),
    )
}

/// A buffer to print to, and the output that writes to it.
fn buffer() -> (Rc<RefCell<Vec<u8>>>, Output) {
    let buffer = Rc::new(RefCell::new(Vec::new()));
    let output: Output = Rc::clone(&buffer) as Output;
    (buffer, output)
}

#[test]
fn clones_share_the_program_and_runs_share_the_output() {
    let source = "class Main {\n static func main() {\n Debug.print(\"again\")\n }\n}\n";
    let checked = check_source(source).expect("a valid program");
    let program = Program::lower(&checked).expect("a program the interpreter runs");
    let copy = program.clone();
    let (buffer, output) = buffer();
    program.run(&output).expect("a run without errors");
    copy.run(&output).expect("a run without errors");
    assert_eq!(buffer.take(), b"again\nagain\n");
}
