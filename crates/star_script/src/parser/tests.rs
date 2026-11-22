use super::*;
use crate::ast::{MemberKind, ModifierKind, outline};
use crate::{CompileError, parse_source, tokenize};

/// Parses a single expression, which must use every token.
fn expression(source: &str) -> String {
    try_expression(source).expect("valid expression")
}

fn try_expression(source: &str) -> ParseResult<String> {
    let tokens = tokenize(source).expect("no lexical errors");
    let mut parser = Parser::new(&tokens, 0);
    let expression = parser.expression()?;
    assert!(
        parser.at(&TokenKind::EndOfFile),
        "unparsed code after {expression}"
    );
    Ok(expression.to_string())
}

fn expression_error(source: &str) -> ParseErrorKind {
    try_expression(source).expect_err("invalid expression").kind
}

fn file_outline(source: &str) -> String {
    outline(&parse_source(source).expect("valid source"))
}

/// The outline of the statements of a method body, unindented.
fn body(statements: &str) -> String {
    let source = format!("class T {{\n func f() {{\n{statements}\n}}\n}}");
    file_outline(&source)
        .lines()
        .skip(2)
        .map(|line| line.strip_prefix("    ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}

fn errors(source: &str) -> Vec<ParseErrorKind> {
    parse_source(source)
        .expect_err("source has errors")
        .into_iter()
        .map(|error| match error {
            CompileError::Parse(error) => error.kind,
            other => panic!("unexpected error from another stage: {other}"),
        })
        .collect()
}

fn unexpected(expected: Expected, found: TokenKind) -> ParseErrorKind {
    ParseErrorKind::Unexpected { expected, found }
}

// --- Expressions ---

#[test]
fn multiplication_binds_tighter_than_addition() {
    assert_eq!(expression("1 + 2 * 3"), "(+ 1 (* 2 3))");
    assert_eq!(expression("1 * 2 + 3"), "(+ (* 1 2) 3)");
}

#[test]
fn operators_of_equal_precedence_group_left() {
    assert_eq!(expression("a - b - c"), "(- (- a b) c)");
    assert_eq!(expression("a / b * c"), "(* (/ a b) c)");
}

#[test]
fn parentheses_change_grouping() {
    assert_eq!(expression("(1 + 2) * 3"), "(* (+ 1 2) 3)");
}

#[test]
fn logical_and_binds_tighter_than_or() {
    assert_eq!(expression("a || b && c"), "(|| a (&& b c))");
}

#[test]
fn comparisons_bind_looser_than_arithmetic() {
    assert_eq!(
        expression("a + 1 < b * 2 && ok"),
        "(&& (< (+ a 1) (* b 2)) ok)"
    );
    assert_eq!(expression("x % 2 == 0"), "(== (% x 2) 0)");
}

#[test]
fn ranges_bind_looser_than_addition() {
    assert_eq!(expression("0..n + 1"), "(.. 0 (+ n 1))");
}

#[test]
fn unary_operators_bind_tightest() {
    assert_eq!(expression("-a * !b"), "(* (- a) (! b))");
    assert_eq!(expression("--a"), "(- (- a))");
}

#[test]
fn casts_bind_tighter_than_multiplication() {
    assert_eq!(
        expression("lives as float * 2.0"),
        "(* (as lives float) 2.0)"
    );
    assert_eq!(expression("-x as int"), "(as (- x) int)");
}

#[test]
fn type_tests_compare_with_a_class() {
    assert_eq!(
        expression("body is Player && alive"),
        "(&& (is body Player) alive)"
    );
}

#[test]
fn calls_members_and_indexing_chain() {
    assert_eq!(
        expression("Input.is_pressed(\"right\")"),
        "(call (. Input is_pressed) \"right\")"
    );
    assert_eq!(
        expression("enemies[0].target.hit(1, 2,)"),
        "(call (. (. (index enemies 0) target) hit) 1 2)"
    );
    assert_eq!(expression("Player(250.0)"), "(call Player 250.0)");
}

#[test]
fn literals_and_special_values() {
    assert_eq!(
        expression("[1, 2.5, \"a\", true, false, null, self, super.update()]"),
        "(array 1 2.5 \"a\" true false null self (call (. super update)))"
    );
}

#[test]
fn an_operator_at_the_end_of_a_line_continues_the_expression() {
    assert_eq!(expression("a +\n b"), "(+ a b)");
}

#[test]
fn comparisons_cannot_be_chained() {
    assert_eq!(
        expression_error("a < b < c"),
        ParseErrorKind::ChainedOperators
    );
    assert_eq!(
        expression_error("a == b != c"),
        ParseErrorKind::ChainedOperators
    );
    assert_eq!(
        expression_error("0..1..2"),
        ParseErrorKind::ChainedOperators
    );
}

#[test]
fn comparisons_of_different_levels_can_be_combined() {
    assert_eq!(expression("a < b == c"), "(== (< a b) c)");
}

#[test]
fn missing_operand_is_reported() {
    assert_eq!(
        expression_error("1 +"),
        unexpected(Expected::Expression, TokenKind::EndOfFile)
    );
}

#[test]
fn absurdly_deep_nesting_is_an_error_not_a_crash() {
    let source = format!("{}1{}", "(".repeat(1000), ")".repeat(1000));

    assert_eq!(expression_error(&source), ParseErrorKind::TooDeeplyNested);
}

#[test]
fn coalescing_groups_right_and_binds_tighter_than_comparison() {
    assert_eq!(expression("a ?? b ?? c"), "(?? a (?? b c))");
    assert_eq!(expression("a ?? 0 > 3"), "(> (?? a 0) 3)");
    assert_eq!(expression("a ?? b + 1"), "(?? a (+ b 1))");
}

#[test]
fn safe_member_access_chains() {
    assert_eq!(
        expression("player?.target?.hit()"),
        "(call (?. (?. player target) hit))"
    );
}

#[test]
fn super_init_is_a_member_call() {
    assert_eq!(expression("super.init(1)"), "(call (. super init) 1)");
}

#[test]
fn nullable_and_list_types() {
    assert_eq!(
        body("var a: [Enemy?]? = null\nvar b = x as Player?"),
        "var a: [Enemy?]? = null\nvar b = (as x Player?)"
    );
}

// --- Statements ---

#[test]
fn local_variables_with_and_without_types() {
    assert_eq!(
        body("var a = 1\nvar b: float = 2.0\nvar c: int\nconst D = a + 1"),
        "var a = 1\nvar b: float = 2.0\nvar c: int\nconst D = (+ a 1)"
    );
}

#[test]
fn every_assignment_operator() {
    assert_eq!(
        body("a = 1\na += 1\na -= 1\na *= 2\na /= 2\na %= 2\nself.x = 3\nlist[0] = 4"),
        "(= a 1)\n(+= a 1)\n(-= a 1)\n(*= a 2)\n(/= a 2)\n(%= a 2)\n(= (. self x) 3)\n(= (index list 0) 4)"
    );
}

#[test]
fn if_else_if_else() {
    assert_eq!(
        body("if a { x() } else if b { y() } else { z() }"),
        "if a\n  (call x)\nelse if b\n  (call y)\nelse\n  (call z)"
    );
}

#[test]
fn else_may_start_on_the_next_line() {
    assert_eq!(
        body("if a {\n x()\n}\nelse {\n y()\n}"),
        "if a\n  (call x)\nelse\n  (call y)"
    );
}

#[test]
fn braces_may_start_on_the_next_line() {
    assert_eq!(
        file_outline(
            "class A\n{\n  func f()\n  {\n    while true\n    {\n      break\n    }\n  }\n}"
        ),
        "class A\n  func f()\n    while true\n      break\n"
    );
}

#[test]
fn loops_and_jumps() {
    assert_eq!(
        body("while i < 10 { i += 1; continue }\nfor x in 0..3 { break }"),
        "while (< i 10)\n  (+= i 1)\n  continue\nfor x in (.. 0 3)\n  break"
    );
}

#[test]
fn return_with_and_without_value() {
    assert_eq!(
        body("if a { return }\nreturn b + 1"),
        "if a\n  return\nreturn (+ b 1)"
    );
}

#[test]
fn semicolons_separate_statements_on_one_line() {
    assert_eq!(body("a(); b() ;; c()"), "(call a)\n(call b)\n(call c)");
}

#[test]
fn statements_on_one_line_need_a_separator() {
    assert_eq!(
        errors("class T { func f() { a() b() } }"),
        [unexpected(
            Expected::EndOfStatement,
            TokenKind::Identifier("b".into())
        )]
    );
}

#[test]
fn only_variables_fields_and_elements_can_be_assigned() {
    assert_eq!(
        errors("class T { func f() { f() = 3\n 1 += 2 } }"),
        [
            ParseErrorKind::InvalidAssignmentTarget,
            ParseErrorKind::InvalidAssignmentTarget
        ]
    );
}

#[test]
fn assignment_in_a_condition_gets_a_specific_error() {
    assert_eq!(
        errors("class T { func f() { if x = 3 { } } }"),
        [ParseErrorKind::AssignmentInCondition]
    );
}

#[test]
fn constants_need_a_value() {
    assert_eq!(
        errors("class T { const A\n func f() { const B } }"),
        [
            ParseErrorKind::ConstantWithoutValue,
            ParseErrorKind::ConstantWithoutValue
        ]
    );
}

// --- Declarations ---

#[test]
fn empty_file_has_no_classes() {
    assert_eq!(file_outline("\n// nothing yet\n"), "");
}

#[test]
fn several_classes_with_and_without_base() {
    assert_eq!(
        file_outline("class Enemy extends Sprite {}\nclass Score {}"),
        "class Enemy extends Sprite\nclass Score\n"
    );
}

#[test]
fn methods_constructors_and_parameters() {
    assert_eq!(
        file_outline(
            "class V {\n init(x: float, y: float,) {}\n func length() -> float { return x }\n}"
        ),
        "class V\n  init(x: float, y: float)\n  func length() -> float\n    return x\n"
    );
}

#[test]
fn modifiers_are_recorded_in_order() {
    let file = parse_source("class A { private static var count = 0 }").expect("valid");
    let member = &file.classes[0].members[0];

    assert!(member.has_modifier(ModifierKind::Private));
    assert!(member.has_modifier(ModifierKind::Static));
    assert!(!member.has_modifier(ModifierKind::Override));
    assert!(matches!(member.kind, MemberKind::Variable(_)));
}

#[test]
fn repeated_modifiers_are_reported() {
    assert_eq!(
        errors("class A { private private var x = 1 }"),
        [ParseErrorKind::DuplicateModifier(ModifierKind::Private)]
    );
}

#[test]
fn code_outside_classes_is_reported() {
    assert_eq!(
        errors("var x = 1"),
        [unexpected(
            Expected::Class,
            TokenKind::Keyword(Keyword::Var)
        )]
    );
}

#[test]
fn unknown_member_is_reported() {
    assert_eq!(
        errors("class A { print(1) }"),
        [unexpected(
            Expected::ClassMember,
            TokenKind::Identifier("print".into())
        )]
    );
}

#[test]
fn unclosed_class_is_reported() {
    assert_eq!(
        errors("class A {\n var x = 1\n"),
        [unexpected(
            Expected::Token(TokenKind::RightBrace),
            TokenKind::EndOfFile
        )]
    );
}

#[test]
fn reserved_keywords_explain_themselves() {
    let error = parse_source("class A { var match = 1 }").expect_err("has errors");

    assert_eq!(
        error[0].to_string(),
        "expected a name, found keyword `match` (reserved for a future version of the language)"
    );
}

// --- Recovery ---

#[test]
fn every_broken_statement_is_reported_and_the_rest_still_parses() {
    let source = "class A {\n func f() {\n  var = 1\n  ok()\n  x = \n }\n func g() {}\n}";
    let tokens = tokenize(source).expect("no lexical errors");
    let mut parser = Parser::new(&tokens, 0);

    let file = parser.source_file();

    assert_eq!(parser.errors.len(), 2);
    assert_eq!(
        outline(&file),
        "class A\n  func f()\n    (call ok)\n  func g()\n"
    );
}

#[test]
fn a_broken_member_is_skipped_whole() {
    let source = "class A {\n func (x) { if a { b() } }\n var y = 2\n}";
    let tokens = tokenize(source).expect("no lexical errors");
    let mut parser = Parser::new(&tokens, 0);

    let file = parser.source_file();

    assert_eq!(parser.errors.len(), 1);
    assert_eq!(outline(&file), "class A\n  var y = 2\n");
}

#[test]
fn each_stray_closing_brace_is_reported_without_hanging() {
    assert_eq!(
        errors("}\n}}\nclass A {}"),
        vec![unexpected(Expected::Class, TokenKind::RightBrace); 3]
    );
}

#[test]
fn empty_token_list_is_an_empty_file() {
    assert_eq!(parse(&[]), Ok(crate::ast::SourceFile { classes: vec![] }));
}

#[test]
fn lexical_errors_are_reported_without_parse_errors() {
    let errors = parse_source("class A { var x = @ }").expect_err("has errors");

    assert!(matches!(errors[..], [CompileError::Lex(_)]));
}

#[test]
fn spans_cover_the_whole_node() {
    let source = "class A {\n  var speed = (1 + 2) * 3\n}";
    let file = parse_source(source).expect("valid");
    let member = &file.classes[0].members[0];
    let MemberKind::Variable(variable) = &member.kind else {
        panic!("expected a variable");
    };

    assert_eq!(file.classes[0].span.text(source), source);
    assert_eq!(member.span.text(source), "var speed = (1 + 2) * 3");
    let initializer = variable.initializer.as_ref().expect("has a value");
    assert_eq!(initializer.span.text(source), "(1 + 2) * 3");
}

#[test]
fn example_script_parses_as_expected() {
    let expected = "\
class Character
  var x: float = 0.0
  var name: string
  init(name: string)
    (= (. self name) name)
  func move(direction: float, delta: float)
    (+= x (* (* direction (call speed)) delta))
  func speed() -> float
    return 100.0
class Player extends Character
  const MAX_LIVES = 3
  var boost: float = 2
  var target: Enemy?
  var inventory: [string] = (array)
  private var lives = MAX_LIVES
  init(name: string)
    (call (. super init) name)
    (call (. inventory push) \"espada\")
  override func speed() -> float
    return (* (call (. super speed)) boost)
  func hit()
    (-= lives 1)
    if (<= lives 0)
      (call (. Debug print) (+ name \" ha caído\"))
  func is_alive() -> bool
    return (> lives 0)
  func attack()
    const enemy = target
    if (== enemy null)
      return
    (call (. enemy hit_by) self)
  func target_name() -> string
    return (?? (?. target name) \"nadie\")
  func countdown(from: int)
    for i in (.. 0 from)
      if (== (% i 2) 0)
        continue
      (call (. Debug print) (+ \"Quedan \" (call (. (- from i) to_string))))
class Enemy extends Character
  var health = 10
  init()
    (call (. super init) \"Enemigo\")
  func hit_by(player: Player)
    (-= health 1)
    if (== health 0)
      (= (. player target) null)
  func touch(other: Character)
    if (&& (is other Player) (call (. other is_alive)))
      (call (. other hit))
";

    assert_eq!(
        file_outline(include_str!("../../examples/scripts/player.star")),
        expected
    );
}
