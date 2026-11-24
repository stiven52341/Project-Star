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
fn declared_locals_are_listed_where_their_names_are_written() {
    let source = format!("{PRELUDE}{}", in_method("var turns = 2"));
    let analysis = check_text(&source).expect("valid");

    let turns = analysis
        .declared_locals()
        .find(|(_, local)| local.name == "turns")
        .expect("declared");

    assert_eq!(turns.0.text(&source), "turns");
    assert_eq!(turns.1.ty, Type::Int);
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
fn integer_literals_take_the_integer_type_needed() {
    assert_valid_body(
        "var small: short = -32768
         var big: bigint = 9223372036854775807
         small = small * 2 + 1
         big = 3 - big
         var count = 2147483647
         count = count + n
         for i in small..10 { small = i }",
    );
}

#[test]
fn integer_literals_must_fit_in_their_type() {
    assert_eq!(
        body_errors("var small: short = 40000\nvar count = 2147483648"),
        [
            Kind::IntegerOutOfRange {
                value: 40000,
                ty: "short".into(),
                min: -32768,
                max: 32767
            },
            Kind::IntegerOutOfRange {
                value: 2_147_483_648,
                ty: "int".into(),
                min: -2_147_483_648,
                max: 2_147_483_647
            }
        ]
    );
    assert_eq!(
        body_errors("var small: short = -32769"),
        [Kind::IntegerOutOfRange {
            value: -32769,
            ty: "short".into(),
            min: -32768,
            max: 32767
        }]
    );
}

#[test]
fn numbers_widen_by_themselves_without_losing_values() {
    assert_valid_body(
        "var small: short = 1
         var big: bigint = n
         big = small
         big += n
         var total = small + n
         total = total * 2
         var precise: double = speed
         precise = precise + n + speed
         var maybe: bigint? = small
         var same = small == n
         var less = n < precise
         for i in small..n { total = i }",
    );
}

#[test]
fn numbers_that_could_lose_values_need_as() {
    assert_eq!(
        body_errors("var lives: int = 0\nvar big: bigint = 1\nlives = big"),
        [mismatch("int", "bigint")]
    );
    assert_eq!(
        body_errors("var sum = speed + n"),
        [Kind::InvalidOperands {
            operator: "+",
            left: "float".into(),
            right: "int".into()
        }]
    );
    assert_eq!(
        body_errors("var lives = 1\nlives += speed"),
        [Kind::InvalidOperands {
            operator: "+=",
            left: "int".into(),
            right: "float".into()
        }]
    );
    assert_eq!(
        body_errors("var list: [bigint] = [n]\nvar copy: [int] = list"),
        [mismatch("[int]", "[bigint]")]
    );
    assert_valid_body(
        "var big = n as bigint + 1\nvar small = big as short\nvar x = small as float",
    );
}

#[test]
fn decimal_literals_take_double_where_it_is_needed() {
    assert_valid_body("var precise: double = 0.1\nprecise = 1.5 * precise + 2");
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

/// Errors of a whole source file of a game whose only action is `jump`.
fn errors_with_jump_action(source: &str) -> Vec<SemanticErrorKind> {
    let settings = crate::GameSettings {
        actions: vec![crate::InputAction {
            name: "jump".into(),
            bindings: vec!["Space".into()],
        }],
    };
    match check_with(&parse_source(source).expect("valid syntax"), &settings) {
        Ok(_) => Vec::new(),
        Err(errors) => errors.into_iter().map(|error| error.kind).collect(),
    }
}

#[test]
fn input_reads_declared_actions() {
    let source = "class G extends Game {\n override func update(delta: float) {\n \
                  if Input.is_down(\"jump\") || Input.just_pressed(\"jump\") { quit() }\n \
                  const released: bool = Input.just_released(\"jump\")\n \
                  const strength: float = Input.strength(\"jump\")\n \
                  const axis: float = Input.axis(\"jump\", \"jump\")\n }\n}";

    assert_eq!(errors_with_jump_action(source), []);
}

#[test]
fn misspelled_actions_are_compile_errors() {
    let source = "class G extends Game {\n override func update(delta: float) {\n \
                  if Input.just_pressed(\"jupm\") { quit() }\n }\n}";

    assert_eq!(
        errors_with_jump_action(source),
        [Kind::UnknownAction("jupm".into())]
    );
}

#[test]
fn both_actions_of_an_axis_are_checked() {
    let source = "class G extends Game {\n override func update(delta: float) {\n \
                  x = Input.axis(\"left\", \"jump\")\n }\n var x = 0.0\n}";

    assert_eq!(
        errors_with_jump_action(source),
        [Kind::UnknownAction("left".into())]
    );
}

#[test]
fn computed_action_names_are_checked_while_playing() {
    let source = "class G extends Game {\n var player = \"p1\"\n \
                  override func update(delta: float) {\n \
                  if Input.is_down(player + \"_jump\") { quit() }\n }\n}";

    assert_eq!(errors_with_jump_action(source), []);
}

#[test]
fn without_settings_no_action_is_declared() {
    assert_eq!(
        body_errors("const jumping = Input.is_down(\"jump\")"),
        [Kind::UnknownAction("jump".into())]
    );
}

#[test]
fn input_says_where_the_player_points() {
    assert_valid_body(
        "const x: float = Input.pointer_x()\n const y: float = Input.pointer_y()\n \
         const pointing: bool = Input.has_pointer()",
    );
}

#[test]
fn input_takes_an_action_name() {
    assert_eq!(
        errors_with_jump_action("class G { func f() { const x = Input.is_down(3) } }"),
        [Kind::TypeMismatch {
            expected: "string".into(),
            found: "int".into()
        }]
    );
}

// --- Nodes ---

#[test]
fn scripts_extend_nodes_and_add_them_to_the_game() {
    assert_valid(
        "class Player extends Sprite {\n var speed = 4.0\n \
         init() { super.init(\"Player\", \"sprites/hero.png\") }\n \
         override func start() { width = 2.0\n layer = 1 }\n \
         override func update(delta: float) {\n x += speed * delta\n rotation = 0.5\n \
         if x > 10.0 { remove() }\n }\n}\n\
         class MyGame extends Game {\n override func start() {\n \
         const level = Node(\"Level\")\n level.add_child(Player())\n add_child(level)\n \
         const found: Node? = level.find_child(\"Player\")\n \
         const count = level.children().length()\n \
         const top: Node? = level.parent()\n \
         Debug.print(level.name + count.to_string())\n }\n}",
    );
}

#[test]
fn node_fields_cannot_be_declared_again() {
    assert_eq!(
        errors("class Ship extends Node {\n var x = 1.0\n init() { super.init(\"Ship\") }\n}"),
        [Kind::MemberShadowsBase {
            name: "x".into(),
            base: "Node".into()
        }]
    );
}

#[test]
fn nodes_need_their_name_and_sprites_their_image() {
    assert_eq!(
        errors("class Ship extends Sprite {\n init() { }\n}"),
        [Kind::MissingSuperInit("Sprite".into())]
    );
    assert_eq!(
        body_errors("const sprite = Sprite(\"Hero\")"),
        [Kind::WrongArgumentCount {
            expected: 2,
            found: 1
        }]
    );
}

#[test]
fn sprites_and_the_background_have_colors() {
    assert_valid(
        "class G extends Game {\n override func start() {\n \
         background = Color(0.0, 0.0, 0.2, 1.0)\n \
         const ship = Sprite(\"Ship\", \"ship.png\")\n \
         ship.color = Color(1.0, 0.5, 0.5, 1.0)\n ship.color.a = 0.5\n \
         add_child(ship)\n }\n}",
    );
    assert_eq!(
        body_errors("const red = Color(1.0, 0.0, 0.0)"),
        [Kind::WrongArgumentCount {
            expected: 4,
            found: 3
        }]
    );
}

#[test]
fn games_are_seen_from_their_camera() {
    assert_valid(
        "class Shaky extends Camera {\n init() { super.init(\"Eye\") }\n \
         override func update(delta: float) { view_height = 4.0 }\n}\n\
         class G extends Game {\n override func start() {\n \
         const ship = Node(\"Ship\")\n const eye = Shaky()\n \
         ship.add_child(eye)\n add_child(ship)\n camera = eye\n }\n \
         override func update(delta: float) { camera = null }\n}",
    );
    assert_eq!(
        body_errors("const eye: Camera = Node(\"Eye\")"),
        [mismatch("Camera", "Node")]
    );
}

#[test]
fn games_show_3d_nodes_from_their_3d_camera() {
    assert_valid(
        "class Crate extends Mesh {\n init() {\n super.init(\"Crate\", \"cube\")\n \
         z = 2.0\n yaw = 0.5\n metallic = 1.0\n color = Color(1.0, 0.5, 0.0, 1.0)\n \
         body = Body.box(\"dynamic\", 1.0, 1.0, 1.0)\n }\n}\n\
         class G extends Game {\n override func start() {\n \
         const eye = Camera3D(\"Eye\")\n eye.look_at(0.0, 0.0, 0.0)\n eye.fov = 1.2\n \
         camera_3d = eye\n ambient = Color(0.2, 0.2, 0.2, 1.0)\n \
         const sun = Light(\"Sun\", \"directional\")\n sun.intensity = 2.0\n \
         const crate_mesh = Crate()\n crate_mesh.body?.apply_impulse_3d(0.0, 1.0, 0.0)\n \
         add_child(eye)\n add_child(sun)\n add_child(crate_mesh)\n \
         const ball = Node3D(\"Ball\")\n ball.body = Body.sphere(\"dynamic\", 0.5)\n \
         crate_mesh.add_child(ball)\n }\n}",
    );
    assert_eq!(
        body_errors("const eye: Camera3D = Camera(\"Eye\")"),
        [mismatch("Camera3D", "Camera")]
    );
    assert_eq!(
        body_errors("const flat: Node3D = Node(\"Flat\")"),
        [mismatch("Node3D", "Node")]
    );
}

#[test]
fn scenes_load_trees_that_may_be_missing() {
    assert_valid(
        "class G extends Game {\n override func start() {\n \
         const level = Scene.load(\"levels/one.scene\")\n \
         if level != null { add_child(level) }\n }\n}",
    );
    assert_eq!(
        errors(
            "class G extends Game {\n override func start() {\n \
             add_child(Scene.load(\"levels/one.scene\"))\n }\n}"
        ),
        [mismatch("Node", "Node?")]
    );
}

#[test]
fn sounds_are_loaded_and_may_be_missing() {
    assert_valid_body(
        "const jump = Sound.load(\"sounds/jump.wav\")\n \
         if jump != null {\n jump.volume = 0.5\n jump.looping = true\n jump.play()\n \
         if jump.is_playing() { jump.stop() }\n }\n \
         Audio.set_volume(Audio.volume() / 2.0)",
    );
    assert_eq!(
        body_errors("const jump: Sound = Sound.load(\"sounds/jump.wav\")"),
        [mismatch("Sound", "Sound?")]
    );
}

#[test]
fn nodes_have_bodies_that_name_what_they_touch() {
    assert_valid_body(
        "const ball = Sprite(\"Ball\", \"ball.png\")\n \
         ball.body = Body.circle(\"dynamic\", 0.5)\n \
         const solid = ball.body\n \
         if solid != null {\n solid.bounciness = 0.5\n solid.sensor = true\n \
         solid.velocity_x = solid.velocity_y + 1.0\n solid.apply_impulse(0.0, 2.0)\n \
         for other in solid.just_touched() {\n Debug.print(other.name)\n }\n \
         Debug.print(solid.kind + solid.shape)\n }\n \
         ball.body = Body.rectangle(\"static\", 2.0, 1.0)\n \
         ball.body = Body.capsule(\"kinematic\", 2.0, 0.5)\n \
         ball.body = null",
    );
    assert_eq!(
        body_errors("const solid: Body = Node(\"Ball\").body"),
        [mismatch("Body", "Body?")]
    );
}

/// The example of `docs/starscript.md`.
#[test]
fn the_documented_bodies_are_valid() {
    let settings = crate::GameSettings {
        actions: ["left", "right", "jump"]
            .map(|name| crate::InputAction {
                name: name.into(),
                bindings: vec!["Space".into()],
            })
            .into(),
    };
    let source = r#"
class Ball extends Sprite {
    init() {
        super.init("Ball", "sprites/ball.png")
        const solid = Body.circle("dynamic", 0.5)
        solid.bounciness = 0.6
        body = solid
    }
}

class Hero extends Sprite {
    init() {
        super.init("Hero", "sprites/hero.png")
        const solid = Body.capsule("dynamic", 2.0, 0.5)
        solid.fixed_rotation = true
        body = solid
    }

    override func update(delta: float) {
        const solid = body
        if solid == null { return }
        solid.velocity_x = Input.axis("left", "right") * 4.0
        if Input.just_pressed("jump") { solid.apply_impulse(0.0, 5.0) }
        for other in solid.just_touched() {
            if other.name == "Coin" { other.remove() }
        }
    }
}
"#;

    let checked = check_with(&parse_source(source).expect("valid syntax"), &settings);

    assert!(checked.is_ok(), "{:?}", checked.err());
}

#[test]
fn bodies_only_come_from_their_shapes() {
    assert_eq!(
        body_errors("const solid = Body()"),
        [Kind::PrivateMember {
            name: "init".into(),
            owner: "Body".into(),
        }]
    );
}

#[test]
fn sounds_only_come_from_files() {
    assert_eq!(
        body_errors("const jump = Sound()"),
        [Kind::PrivateMember {
            name: "init".into(),
            owner: "Sound".into(),
        }]
    );
}

#[test]
fn only_nodes_can_join_the_tree() {
    assert_eq!(
        errors("class G extends Game {\n override func start() { add_child(Enemy()) }\n}"),
        [mismatch("Node", "Enemy")]
    );
}

// --- Coroutines ---

/// `Lamp`, with an `async` method `wait_for(seconds) -> int`, and `extra`.
fn with_lamp(extra: &str) -> String {
    format!(
        "class Lamp {{\n async func wait_for(seconds: float) -> int {{\n await Time.wait(seconds)\n await Time.next_frame()\n return 1\n }}\n{extra}\n}}"
    )
}

#[test]
fn async_methods_await_other_async_methods_and_get_their_value() {
    assert_valid(&with_lamp(
        "async func blink() {\n var waited: int = await wait_for(0.5) + 1\n await blink()\n }",
    ));
}

#[test]
fn async_methods_called_alone_start_coroutines() {
    assert_valid(&with_lamp(
        "func start() {\n wait_for(1.0)\n Time.wait(2.0)\n var lamp: Lamp? = null\n lamp?.wait_for(1.0)\n }",
    ));
}

#[test]
fn async_calls_inside_expressions_need_await() {
    assert_eq!(
        errors(&with_lamp(
            "func f() {\n var n = wait_for(1.0)\n Debug.print(wait_for(1.0).to_string())\n }"
        )),
        [
            Kind::AsyncCallNotAwaited("wait_for".to_owned()),
            Kind::AsyncCallNotAwaited("wait_for".to_owned())
        ]
    );
}

#[test]
fn await_outside_async_methods_is_reported() {
    assert_eq!(
        errors(&with_lamp("func f() { await wait_for(1.0) }")),
        [Kind::AwaitOutsideAsync]
    );
}

#[test]
fn only_calls_of_async_methods_can_be_awaited() {
    assert_eq!(
        errors(&with_lamp(
            "func plain() {}\n async func f() {\n await plain()\n var n = await 3\n }"
        )),
        [Kind::AwaitWithoutAsyncCall, Kind::AwaitWithoutAsyncCall]
    );
}

#[test]
fn await_cannot_wait_through_safe_access() {
    assert_eq!(
        errors(&with_lamp(
            "async func f(other: Lamp?) { await other?.wait_for(1.0) }"
        )),
        [Kind::NullSafeAwait]
    );
}

#[test]
fn overrides_are_async_exactly_when_their_base_is() {
    let source = "class Base { async func a() {}\n func b() {} }\n\
                  class Child extends Base { override func a() {}\n override async func b() {} }\n\
                  class G extends Game { override async func update(delta: float) {} }";

    assert_eq!(
        errors(source),
        [
            Kind::AsyncOverrideMismatch {
                name: "a".to_owned(),
                base: "Base".to_owned(),
                base_is_async: true
            },
            Kind::AsyncOverrideMismatch {
                name: "b".to_owned(),
                base: "Base".to_owned(),
                base_is_async: false
            },
            Kind::AsyncOverrideMismatch {
                name: "update".to_owned(),
                base: "Game".to_owned(),
                base_is_async: false
            }
        ]
    );
}

#[test]
fn constructors_cannot_be_async() {
    assert_eq!(
        errors("class A { async init() {} }"),
        [Kind::InvalidModifier {
            modifier: ModifierKind::Async,
            member: "`init`"
        }]
    );
}

#[test]
fn awaited_values_are_still_checked() {
    assert_eq!(
        body_errors("await missing()"),
        [
            Kind::AwaitOutsideAsync,
            Kind::UnknownName("missing".to_owned())
        ]
    );
}

// --- Parallel functions ---

/// `Paths`, with the `parallel` function `sum(values: [int]) -> int`, and
/// `extra`.
fn with_paths(extra: &str) -> String {
    format!(
        "class Paths {{\n parallel static func sum(values: [int]) -> int {{\n var total = 0\n for value in values {{ total += value.abs() }}\n return total\n }}\n{extra}\n}}"
    )
}

#[test]
fn parallel_functions_are_awaited_from_coroutines() {
    assert_valid(&with_paths(
        "static async func use_it() {\n var total: int = await sum([1, 2]) + 1\n }",
    ));
}

#[test]
fn parallel_functions_call_each_other_directly() {
    assert_valid(&with_paths(
        "const LIMIT = 3\n private parallel static func twice(values: [int]) -> [int?] {\n var result: [int?] = [sum(values), null, LIMIT]\n return result\n }",
    ));
}

#[test]
fn parallel_functions_need_await() {
    assert_eq!(
        errors(&with_paths(
            "static func start() {\n sum([1])\n var n = sum([2])\n }"
        )),
        [
            Kind::ParallelCallNotAwaited("sum".to_owned()),
            Kind::ParallelCallNotAwaited("sum".to_owned())
        ]
    );
}

#[test]
fn parallel_functions_only_take_and_give_values() {
    assert_eq!(
        errors(
            "class Rock {}\nclass P {\n parallel static func f(rock: Rock, rocks: [Rock?]) -> Rock { return rock }\n parallel static func g(n: int) {}\n}"
        ),
        [
            Kind::NotPlainValue("Rock".to_owned()),
            Kind::NotPlainValue("[Rock?]".to_owned()),
            Kind::NotPlainValue("Rock".to_owned()),
            Kind::ParallelWithoutResult("g".to_owned())
        ]
    );
}

#[test]
fn parallel_functions_only_use_values_and_other_parallel_functions() {
    let source = "class Rock {}\nclass P {\n static func plain() -> int { return 1 }\n \
                  async func later() {}\n \
                  parallel static func f(n: int) -> int {\n Debug.print(\"no\")\n var rock = Rock()\n await later()\n return plain()\n }\n}";

    assert_eq!(
        errors(source),
        [
            Kind::NotParallel("print".to_owned()),
            Kind::NotParallel("Rock".to_owned()),
            Kind::AwaitOutsideAsync,
            Kind::SelfUnavailable,
            Kind::NotParallel("plain".to_owned())
        ]
    );
}

#[test]
fn parallel_goes_only_with_static_and_never_with_async() {
    assert_eq!(
        errors(
            "class P {\n parallel func f() -> int { return 1 }\n parallel static async func g() -> int { return 1 }\n}"
        ),
        [
            Kind::InvalidModifier {
                modifier: ModifierKind::Parallel,
                member: "a method that is not static"
            },
            Kind::InvalidModifier {
                modifier: ModifierKind::Async,
                member: "a parallel function"
            }
        ]
    );
}

#[test]
fn native_classes_are_listed_with_their_members_and_bases() {
    let classes = native_classes();

    let node = classes.find(NODE_CLASS).unwrap();
    let sprite = classes.find(SPRITE_CLASS).unwrap();
    assert!(classes.is_subclass(sprite, node));
    assert!(classes.find_member(sprite, "x").is_some());
    assert!(
        classes
            .iter()
            .all(|(_, class)| class.origin == ClassOrigin::Native)
    );
}
