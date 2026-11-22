//! Tests of the shape of the generated code. Whether it compiles and runs
//! as expected is tested end to end in `tests/end_to_end.rs`.

use crate::compile_source;

fn rust(source: &str) -> String {
    match compile_source(source) {
        Ok(code) => code,
        Err(errors) => {
            let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
            panic!("compile errors: {messages:#?}");
        }
    }
}

#[test]
fn each_class_gets_a_module() {
    let code = rust("class Player { var lives = 3 }\nclass Enemy {}");

    assert!(code.contains("pub mod Player {"));
    assert!(code.contains("pub mod Enemy {"));
    assert!(code.contains("pub lives: ::star_script_runtime::Field<i64>,"));
}

#[test]
fn rust_keywords_are_escaped() {
    let code = rust("class struct { var type = 1\n func fn(None: int) {} }");

    assert!(code.contains("pub mod r#struct {"));
    assert!(code.contains("pub r#type: "));
    assert!(code.contains("pub fn r#fn(this_: &Ref, None_: i64)"));
}

#[test]
fn subclasses_extend_the_trait_of_their_base() {
    let code = rust("class A { func f() {} }\nclass B extends A { override func f() {} }");

    assert!(code.contains("pub trait Object: crate::classes::A::Object + HasFields {"));
    assert!(code.contains("crate::classes::B::Methods::f(&(self as crate::classes::B::Ref))"));
}

#[test]
fn integers_wrap_around() {
    let code = rust("class A { func f(a: int) -> int { return a * 2 + 1 } }");

    assert!(code.contains("((a).wrapping_mul(2_i64)).wrapping_add(1_i64)"));
}

#[test]
fn entry_point_calls_static_main() {
    let code = rust("class Game { static func main() { Debug.print(\"hola\") } }");

    assert!(code.contains("pub fn main() {"));
    assert!(code.contains("crate::classes::Game::Methods::main();"));
}

#[test]
fn programs_without_main_have_no_entry_point() {
    assert!(!rust("class A {}").contains("pub fn main"));
}

#[test]
fn only_one_class_can_have_main() {
    let errors =
        compile_source("class A { static func main() {} }\nclass B { static func main() {} }")
            .expect_err("two entry points");

    assert_eq!(
        errors[0].to_string(),
        "only one class can declare `static func main()`"
    );
}

#[test]
fn generated_code_has_no_license_notice() {
    // Generated code belongs to the game's author (CLAUDE.md 3.6).
    assert!(!rust("class A {}").contains("GPL"));
}
