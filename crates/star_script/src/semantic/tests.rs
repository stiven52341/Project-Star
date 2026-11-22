use super::*;
use crate::ast::{ExprKind, MemberKind, ModifierKind, StatementKind};
use crate::parse_source;
use SemanticErrorKind as Kind;

/// Classes available to every test.
const PRELUDE: &str = "
class Enemy {
    var health = 10
    func alive() -> bool { return health > 0 }
    func hit() { health -= 1 }
}
";

fn check_text(source: &str) -> Result<Analysis, Vec<SemanticError>> {
    check(&parse_source(source).expect("valid syntax"))
}

/// Errors of a whole source file, prelude included.
fn errors(source: &str) -> Vec<SemanticErrorKind> {
    check_text(&format!("{PRELUDE}{source}"))
        .expect_err("source has semantic errors")
        .into_iter()
        .map(|error| error.kind)
        .collect()
}

fn assert_valid(source: &str) {
    if let Err(errors) = check_text(&format!("{PRELUDE}{source}")) {
        let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
        panic!("unexpected errors: {messages:#?}");
    }
}

/// Wraps statements in a method with parameters `e: Enemy?`, `n: int`, `speed: float`.
fn in_method(statements: &str) -> String {
    format!("class T {{\n func f(e: Enemy?, n: int, speed: float) {{\n{statements}\n}}\n}}")
}

fn body_errors(statements: &str) -> Vec<SemanticErrorKind> {
    errors(&in_method(statements))
}

fn assert_valid_body(statements: &str) {
    assert_valid(&in_method(statements));
}

fn mismatch(expected: &str, found: &str) -> SemanticErrorKind {
    Kind::TypeMismatch {
        expected: expected.to_owned(),
        found: found.to_owned(),
    }
}

// --- Null safety ---

#[test]
fn null_only_fits_nullable_types() {
    assert_eq!(
        body_errors("var a: Enemy = null"),
        [mismatch("Enemy", "null")]
    );
    assert_valid_body("var a: Enemy? = null");
}

#[test]
fn members_of_a_nullable_value_need_a_check() {
    assert_eq!(
        body_errors("e.hit()"),
        [Kind::PossiblyNull("Enemy?".into())]
    );
    assert_eq!(
        body_errors("var h = e.health"),
        [Kind::PossiblyNull("Enemy?".into())]
    );
}

#[test]
fn safe_access_gives_a_nullable_result() {
    assert_valid_body("e?.hit()");
    assert_eq!(
        body_errors("var h: int = e?.health"),
        [mismatch("int", "int?")]
    );
}

#[test]
fn coalescing_removes_the_null() {
    assert_valid_body("var h: int = e?.health ?? 0");
    assert_valid_body("var other: Enemy = e ?? Enemy()");
}

#[test]
fn null_checks_narrow_parameters_and_constants() {
    assert_valid_body("if e != null { e.hit() }");
    assert_valid_body("const c = e\nif c != null { c.hit() }");
    assert_valid_body("if null != e { e.hit() } else { e?.hit() }");
    assert_valid_body("if !(e == null) { e.hit() }");
}

#[test]
fn null_checks_do_not_narrow_variables_that_can_change() {
    assert_eq!(
        body_errors("var v = e\nif v != null { v.hit() }"),
        [Kind::PossiblyNullVariable("v".into())]
    );
}

#[test]
fn narrowing_ends_with_its_block() {
    assert_eq!(
        body_errors("if e != null { e.hit() }\ne.hit()"),
        [Kind::PossiblyNull("Enemy?".into())]
    );
}

#[test]
fn leaving_early_narrows_the_rest_of_the_block() {
    assert_valid_body("if e == null { return }\ne.hit()");
    assert_valid_body("while true { if e == null { break }\n e.hit() }");
    assert_valid_body("if e != null { } else { return }\ne.hit()");
}

#[test]
fn logical_operators_narrow_their_right_side() {
    assert_valid_body("if e != null && e.alive() { e.hit() }");
    assert_valid_body("if e == null || e.alive() { }");
    assert_eq!(
        body_errors("if e != null || e.alive() { }"),
        [Kind::PossiblyNull("Enemy?".into())]
    );
}

#[test]
fn null_operations_on_values_that_are_never_null_are_reported() {
    assert_eq!(
        body_errors("var a = Enemy()\na?.hit()"),
        [Kind::NeverNull("Enemy".into())]
    );
    assert_eq!(
        body_errors("var h = n ?? 0"),
        [Kind::NeverNull("int".into())]
    );
    assert_eq!(
        body_errors("if n == null { }"),
        [Kind::NeverNull("int".into())]
    );
}

#[test]
fn cannot_assign_through_safe_access() {
    assert_eq!(
        body_errors("e?.health = 3"),
        [Kind::AssignThroughSafeAccess]
    );
}

#[test]
fn type_of_null_alone_cannot_be_inferred() {
    assert_eq!(body_errors("var x = null"), [Kind::CannotInferNull]);
    assert_eq!(body_errors("var x = []"), [Kind::CannotInferEmptyList]);
    assert_valid_body("var x: [int] = []\nvar y = [e, null]\nvar z = [1, 2]");
}

#[test]
fn objects_cannot_be_left_empty() {
    assert_eq!(
        body_errors("var a: Enemy"),
        [Kind::MissingValue("a".into())]
    );
    assert_valid_body("var a: Enemy?\nvar b: int\nvar c: [Enemy]");
}

// --- Types ---

#[test]
fn integer_literals_become_floats_where_needed() {
    assert_valid_body("var a: float = 2\nvar b = speed * 2\nvar c = 2 * speed\nvar d: float = -3");
    assert_valid_body("if speed > 0 { }");
}

#[test]
fn int_and_float_do_not_mix_implicitly() {
    let error =
        check_text(&format!("{PRELUDE}{}", in_method("var f: float = n"))).expect_err("has errors");

    assert_eq!(error[0].kind, mismatch("float", "int"));
    assert!(error[0].to_string().contains("as float"));
    assert_eq!(
        body_errors("var x = speed + n"),
        [Kind::InvalidOperands {
            operator: "+",
            left: "float".into(),
            right: "int".into()
        }]
    );
    assert_valid_body("var x = speed + n as float");
}

#[test]
fn text_is_joined_with_plus() {
    assert_valid_body("var s = \"a\" + \"b\"\ns += \"c\"");
    assert_eq!(
        body_errors("var s = \"vidas: \" + n"),
        [Kind::InvalidOperands {
            operator: "+",
            left: "string".into(),
            right: "int".into()
        }]
    );
}

#[test]
fn inferred_types_are_recorded() {
    let source = format!("{PRELUDE}{}", in_method("var a = 1.5\nvar b = [n]"));
    let file = parse_source(&source).expect("valid syntax");
    let analysis = check(&file).expect("valid");
    let MemberKind::Method(method) = &file.classes[1].members[0].kind else {
        panic!("expected a method");
    };
    let local_type = |index: usize| {
        let StatementKind::Variable(variable) = &method.body.statements[index].kind else {
            panic!("expected a variable");
        };
        let id = analysis.declared_local(&variable.name).expect("declared");
        analysis.local(id).expect("exists").ty.clone()
    };

    assert_eq!(local_type(0), Type::Float);
    assert_eq!(local_type(1), Type::list(Type::Int));
}

#[test]
fn conditions_must_be_bool() {
    assert_eq!(body_errors("if n { }"), [mismatch("bool", "int")]);
    assert_eq!(body_errors("while e { }"), [mismatch("bool", "Enemy?")]);
}

#[test]
fn compound_assignment_needs_matching_numbers() {
    assert_valid_body("var a = 1\na += 2\na %= 2");
    assert_eq!(
        body_errors("var a = 1\na += 2.5"),
        [Kind::InvalidOperands {
            operator: "+=",
            left: "int".into(),
            right: "float".into()
        }]
    );
}

#[test]
fn casts_between_numbers_and_related_classes() {
    assert_valid(
        "class Boss extends Enemy {}
         class T { func f(enemy: Enemy, boss: Boss) {
             var a = 3.7 as int
             var b: Boss? = enemy as Boss
             var c: Enemy = boss as Enemy
         } }",
    );
    assert_eq!(
        body_errors("var a = \"3\" as int"),
        [Kind::InvalidCast {
            from: "string".into(),
            to: "int".into()
        }]
    );
}

#[test]
fn type_tests_narrow_to_the_subclass() {
    assert_valid(
        "class Boss extends Enemy { func roar() {} }
         class T { func f(enemy: Enemy?) {
             if enemy is Boss { enemy.roar() }
         } }",
    );
    assert_eq!(
        body_errors("if n is Enemy { }"),
        [Kind::InvalidOperand {
            operator: "is",
            operand: "int".into()
        }]
    );
}

#[test]
fn builtin_methods_have_types() {
    assert_valid_body(
        "var a: int = \"hola\".length()\nvar b: string = n.to_string()\nvar c = speed.sqrt().floor()",
    );
    assert_eq!(
        body_errors("var l = [1]\nl.push(\"x\")"),
        [mismatch("int", "string")]
    );
    assert_eq!(
        body_errors("var l = [1]\nvar last: int = l.pop()"),
        [mismatch("int", "int?")]
    );
    assert_eq!(
        body_errors("n.explode()"),
        [Kind::UnknownMember {
            owner: "int".into(),
            name: "explode".into()
        }]
    );
}

#[test]
fn lists_are_indexed_with_integers() {
    assert_valid_body("var l = [1, 2]\nvar a: int = l[0]\nl[1] = 3");
    assert_eq!(
        body_errors("var a = n[0]"),
        [Kind::NotIndexable("int".into())]
    );
    assert_eq!(
        body_errors("var l = [1]\nvar a = l[1.5]"),
        [mismatch("int", "float")]
    );
}

#[test]
fn for_goes_through_ranges_and_lists() {
    assert_valid_body("for i in 0..n { var x: int = i }\nfor h in [1.5] { var y: float = h }");
    assert_eq!(
        body_errors("for i in n { }"),
        [Kind::InvalidOperand {
            operator: "for",
            operand: "int".into()
        }]
    );
    assert_eq!(body_errors("var r = 0..3"), [Kind::RangeOutsideFor]);
}

#[test]
fn methods_that_return_nothing_have_no_value() {
    assert_eq!(body_errors("var x = Debug.print(\"a\")"), [Kind::NoValue]);
}

// --- Names ---

#[test]
fn unknown_names_types_and_members() {
    assert_eq!(body_errors("ghost()"), [Kind::UnknownName("ghost".into())]);
    assert_eq!(
        body_errors("var a: Ghost? = null"),
        [Kind::UnknownType("Ghost".into())]
    );
    assert_eq!(
        body_errors("Enemy().fly()"),
        [Kind::UnknownMember {
            owner: "Enemy".into(),
            name: "fly".into()
        }]
    );
}

#[test]
fn names_cannot_be_declared_twice() {
    assert_eq!(
        body_errors("var a = 1\nif true { var a = 2 }"),
        [Kind::DuplicateLocal("a".into())]
    );
    assert_eq!(body_errors("var n = 1"), [Kind::DuplicateLocal("n".into())]);
    assert_eq!(
        errors("class Enemy {}"),
        [Kind::DuplicateClass("Enemy".into())]
    );
    assert_eq!(errors("class int {}"), [Kind::DuplicateClass("int".into())]);
    assert_eq!(
        errors("class A { var x = 1\n func x() {} }"),
        [Kind::DuplicateMember("x".into())]
    );
}

#[test]
fn sibling_blocks_can_reuse_names() {
    assert_valid_body("if true { var a = 1 } else { var a = 2 }");
}

#[test]
fn parameters_may_hide_fields() {
    assert_valid("class A { var speed = 1.0\n init(speed: float) { self.speed = speed } }");
}

#[test]
fn constants_parameters_and_loop_variables_cannot_change() {
    assert_eq!(body_errors("n = 3"), [Kind::AssignToImmutable("n".into())]);
    assert_eq!(
        body_errors("const k = 1\nk = 2"),
        [Kind::AssignToImmutable("k".into())]
    );
    assert_eq!(
        body_errors("for i in 0..3 { i = 1 }"),
        [Kind::AssignToImmutable("i".into())]
    );
    assert_eq!(
        errors("class A { const M = 1\n func f() { M = 2 } }"),
        [Kind::AssignToImmutable("M".into())]
    );
}

#[test]
fn private_members_stay_inside_their_class() {
    let classes = "class Safe { private var code = 1234\n func same(other: Safe) -> int { return other.code } }";
    assert_valid(classes);
    assert_eq!(
        errors(&format!(
            "{classes}\nclass Thief {{ func f(s: Safe) {{ var c = s.code }} }}"
        )),
        [Kind::PrivateMember {
            name: "code".into(),
            owner: "Safe".into()
        }]
    );
}

#[test]
fn only_calls_can_be_statements() {
    assert_eq!(body_errors("n + 1"), [Kind::UnusedValue]);
}

// --- Static code ---

#[test]
fn static_methods_have_no_object() {
    assert_eq!(
        errors("class A { var x = 1\n static func f() { x = 2 } }"),
        [Kind::SelfUnavailable]
    );
    assert_eq!(
        errors("class A { static func f() { var a = self } }"),
        [Kind::SelfUnavailable]
    );
    assert_valid(
        "class A { static func make() -> A { return A() }\n func f() { var a = A.make() } }",
    );
}

#[test]
fn instance_and_static_members_are_called_their_own_way() {
    assert_eq!(body_errors("Enemy.hit()"), [Kind::NotStatic("hit".into())]);
    assert_eq!(
        errors("class A { static func s() {}\n func f(a: A) { a.s() } }"),
        [Kind::StaticThroughObject("s".into())]
    );
    assert_valid_body("Debug.print(\"hola\")");
}

#[test]
fn field_values_cannot_use_other_fields() {
    assert_eq!(
        errors("class A { var a = 1\n var b = a + 1 }"),
        [Kind::SelfUnavailable]
    );
    assert_valid("class A { const BASE = 2\n var b = BASE * 3\n var e = Enemy() }");
}

#[test]
fn constants_only_hold_constant_values() {
    assert_valid("class A { const A = 2\n const B: float = A as float * 1.5\n const C = -A }");
    assert_eq!(errors("class A { const E = Enemy() }"), [Kind::NotConstant]);
    assert_eq!(
        errors("class A { const X = Y\n const Y = 1 }"),
        [Kind::ConstantUsedBeforeDeclaration("Y".into())]
    );
}

// --- Control flow ---

#[test]
fn methods_with_a_result_must_always_return() {
    assert_eq!(
        errors("class A { func f(x: int) -> int { if x > 0 { return 1 } } }"),
        [Kind::MissingReturn("f".into())]
    );
    assert_valid("class A { func f(x: int) -> int { if x > 0 { return 1 } else { return 2 } } }");
}

#[test]
fn return_values_match_the_method() {
    assert_eq!(body_errors("return 1"), [Kind::UnexpectedReturnValue]);
    assert_eq!(
        errors("class A { func f() -> int { return } }"),
        [Kind::MissingReturnValue("int".into())]
    );
    assert_eq!(
        errors("class A { func f() -> int { return \"no\" } }"),
        [mismatch("int", "string")]
    );
}

#[test]
fn break_and_continue_need_a_loop() {
    assert_eq!(body_errors("break"), [Kind::BreakOutsideLoop]);
    assert_eq!(body_errors("continue"), [Kind::ContinueOutsideLoop]);
    assert_valid_body("while true { if n > 0 { break } else { continue } }");
}

// --- Calls ---

#[test]
fn arguments_match_parameters() {
    assert_eq!(
        body_errors("Debug.print(\"a\", \"b\")"),
        [Kind::WrongArgumentCount {
            expected: 1,
            found: 2
        }]
    );
    assert_eq!(body_errors("Debug.print(n)"), [mismatch("string", "int")]);
}

#[test]
fn only_methods_and_classes_can_be_called() {
    assert_eq!(body_errors("n()"), [Kind::NotCallable]);
    assert_eq!(body_errors("var h = Enemy().health()"), [Kind::NotCallable]);
    assert_eq!(
        body_errors("var m = Enemy().alive"),
        [Kind::NotAValue("alive".into())]
    );
}

#[test]
fn constructors_check_their_arguments() {
    let class = "class P { init(name: string) {} }";
    assert_valid(&format!(
        "{class}\nclass T {{ func f() {{ var p = P(\"a\") }} }}"
    ));
    assert_eq!(
        errors(&format!(
            "{class}\nclass T {{ func f() {{ var p = P() }} }}"
        )),
        [Kind::WrongArgumentCount {
            expected: 1,
            found: 0
        }]
    );
    assert_eq!(
        errors("class S { private init() {} }\nclass T { func f() { var s = S() } }"),
        [Kind::PrivateMember {
            name: "init".into(),
            owner: "S".into()
        }]
    );
}

// --- Inheritance ---

#[test]
fn redefined_methods_need_override_and_the_same_signature() {
    let base = "class Base { func f(x: int) -> int { return x } }";
    assert_valid(&format!(
        "{base}\nclass Ok extends Base {{ override func f(x: int) -> int {{ return 2 }} }}"
    ));
    assert_eq!(
        errors(&format!(
            "{base}\nclass A extends Base {{ func f(x: int) -> int {{ return 2 }} }}"
        )),
        [Kind::MissingOverride {
            name: "f".into(),
            base: "Base".into()
        }]
    );
    assert_eq!(
        errors(&format!(
            "{base}\nclass B extends Base {{ override func f(x: float) -> int {{ return 2 }} }}"
        )),
        [Kind::OverrideMismatch {
            name: "f".into(),
            base: "Base".into()
        }]
    );
    assert_eq!(
        errors("class C { override func g() {} }"),
        [Kind::OverrideWithoutBase("g".into())]
    );
}

#[test]
fn fields_cannot_reuse_base_names() {
    assert_eq!(
        errors("class Boss extends Enemy { var health = 99 }"),
        [Kind::MemberShadowsBase {
            name: "health".into(),
            base: "Enemy".into()
        }]
    );
}

#[test]
fn base_members_are_inherited() {
    assert_valid(
        "class Boss extends Enemy { func rage() { health += 5\n hit()\n super.hit() } }
         class T { func f(b: Boss) { var alive: bool = b.alive()\n var e: Enemy = b } }",
    );
}

#[test]
fn invalid_inheritance_is_reported() {
    assert_eq!(
        errors("class A extends B {}\nclass B extends A {}"),
        [Kind::CyclicInheritance("A".into())]
    );
    assert_eq!(
        errors("class A extends int {}"),
        [Kind::CannotExtend("int".into())]
    );
    assert_eq!(
        errors("class A extends Ghost {}"),
        [Kind::UnknownType("Ghost".into())]
    );
}

#[test]
fn modifiers_must_fit_the_member() {
    assert_eq!(
        errors("class A { override var x = 1\n static var y = 2\n static override func f() {} }"),
        [
            Kind::InvalidModifier {
                modifier: ModifierKind::Override,
                member: "a field"
            },
            Kind::InvalidModifier {
                modifier: ModifierKind::Static,
                member: "a field"
            },
            Kind::InvalidModifier {
                modifier: ModifierKind::Override,
                member: "a static method"
            },
            Kind::OverrideWithoutBase("f".into()),
        ]
    );
}

// --- Construction ---

#[test]
fn object_fields_need_a_value() {
    assert_eq!(
        errors("class A { var e: Enemy }"),
        [Kind::FieldNotInitialized("e".into())]
    );
    assert_valid("class A { var e: Enemy\n init(e: Enemy) { self.e = e } }");
    assert_valid("class A { var e: Enemy?\n var n: int\n var l: [Enemy] }");
}

#[test]
fn init_must_assign_object_fields_directly_in_its_body() {
    assert_eq!(
        errors("class A { var e: Enemy\n init(c: bool) { if c { e = Enemy() } } }"),
        [
            Kind::FieldNotInitialized("e".into()),
            Kind::SelfBeforeInitialization("`e`".into())
        ]
    );
}

#[test]
fn self_is_unusable_until_the_object_is_complete() {
    assert_eq!(
        errors("class A { var e: Enemy\n init() { f()\n e = Enemy() }\n func f() {} }"),
        [Kind::SelfBeforeInitialization("`e`".into())]
    );
    assert_valid(
        "class A { var e: Enemy\n init() { var x = Enemy()\n e = x\n f() }\n func f() {} }",
    );
    assert_eq!(
        errors("class A { var e: Enemy\n init() { return } }"),
        [
            Kind::FieldNotInitialized("e".into()),
            Kind::ReturnBeforeInitialization
        ]
    );
}

#[test]
fn subclasses_must_call_the_base_constructor_when_it_needs_arguments() {
    let base = "class Base { var name: string\n init(name: string) { self.name = name } }";
    assert_valid(&format!(
        "{base}\nclass Ok extends Base {{ init() {{ super.init(\"x\")\n Debug.print(name) }} }}"
    ));
    assert_eq!(
        errors(&format!("{base}\nclass A extends Base {{ init() {{ }} }}")),
        [Kind::MissingSuperInit("Base".into())]
    );
    assert_eq!(
        errors(&format!("{base}\nclass B extends Base {{ }}")),
        [Kind::MissingSuperInit("Base".into())]
    );
    assert_eq!(
        errors(&format!(
            "{base}\nclass C extends Base {{ init() {{ Debug.print(name)\n super.init(\"x\") }} }}"
        )),
        [Kind::SelfBeforeInitialization("`super.init(…)`".into())]
    );
    assert_eq!(
        errors(&format!(
            "{base}\nclass D extends Base {{ init() {{ super.init(\"x\")\n super.init(\"y\") }}\n func f() {{ super.init(\"z\") }} }}"
        )),
        [Kind::MisplacedSuperInit, Kind::MisplacedSuperInit]
    );
}

#[test]
fn own_fields_get_their_value_before_the_base_constructor() {
    let base = "class Base { var name: string\n init(name: string) { self.name = name } }";
    assert_valid(&format!(
        "{base}\nclass Ok extends Base {{ var e: Enemy\n init() {{ e = Enemy()\n super.init(\"x\") }} }}"
    ));
    assert_eq!(
        errors(&format!(
            "{base}\nclass A extends Base {{ var e: Enemy\n init() {{ super.init(\"x\")\n e = Enemy() }} }}"
        )),
        [Kind::SuperInitBeforeFields("`e`".into())]
    );
}

// --- Results ---

#[test]
fn calls_record_what_they_call() {
    let source = "class A { func f() { Debug.print(\"x\") } }";
    let file = parse_source(source).expect("valid syntax");
    let analysis = check(&file).expect("valid");
    let MemberKind::Method(method) = &file.classes[0].members[0].kind else {
        panic!("expected a method");
    };
    let StatementKind::Expression(call) = &method.body.statements[0].kind else {
        panic!("expected a call");
    };
    let ExprKind::Call { callee, .. } = &call.kind else {
        panic!("expected a call");
    };
    let debug = analysis.classes().find("Debug").expect("native class");

    assert_eq!(
        analysis.expression(callee).and_then(|info| info.resolution),
        Some(Resolution::Method {
            class: debug,
            index: 0
        })
    );
    assert_eq!(analysis.type_of(call), Some(&Type::Void));
}

#[test]
fn errors_come_in_source_order() {
    let errors = check_text("class A { var a: Ghost? = null\n func f() { break } }\nclass A {}")
        .expect_err("has errors");
    let starts: Vec<usize> = errors.iter().map(|error| error.span.start()).collect();

    let mut sorted = starts.clone();
    sorted.sort_unstable();
    assert_eq!(starts, sorted);
    assert_eq!(errors.len(), 3);
}

#[test]
fn example_script_is_valid() {
    let source = include_str!("../../examples/scripts/player.star");
    if let Err(errors) = check_text(source) {
        let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
        panic!("example script has errors: {messages:#?}");
    }
}

#[test]
fn games_extend_the_engine_class() {
    assert_valid(
        "class MyGame extends Game {\n var frames = 0\n override func start() { Debug.print(\"go\") }\n override func update(delta: float) {\n frames += 1\n if frames == 3 { quit() }\n }\n}",
    );
    assert_eq!(
        errors("class Bad extends Game { func update(delta: float) {} }"),
        [Kind::MissingOverride {
            name: "update".into(),
            base: "Game".into()
        }]
    );
}
