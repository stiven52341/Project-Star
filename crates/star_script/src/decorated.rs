//! Decorators of the program: classes that extend `Decorator`, written
//! before methods that return nothing (`@Once`, `@Cooldown(0.5)`).
//!
//! StarScript has no functions as values, so a decorator cannot wrap a
//! method the way a closure would. Instead, after parsing, each decorated
//! method is rewritten in the syntax tree, and the rest of the compiler
//! checks and translates the result as any other code:
//!
//! ```text
//! @Cooldown(0.5)                    private var @0@shoot = Cooldown(0.5)
//! func shoot() { … }        →       func shoot() {
//!                                       if @0@shoot.before("shoot") {
//!                                           shoot@body()
//!                                           @0@shoot.after("shoot")
//!                                       }
//!                                   }
//!                                   private func shoot@body() { … }
//! ```
//!
//! Each object keeps its own decorator objects, so they can remember
//! things (when it last ran, whether it ran once). Several decorators nest,
//! the first one outermost. The hidden names have `@`, which no name
//! written in StarScript can have, and become Rust names the generator
//! keeps for itself. The `Decorator` class is written in StarScript too,
//! and added to every program.

use crate::ast::{
    Block, ClassDecl, Decorator, Expr, ExprId, ExprKind, IfStatement, Literal, Member, MemberKind,
    MethodDecl, Modifier, ModifierKind, Name, SourceFile, Statement, StatementKind, VariableDecl,
    VariableKind,
};
use crate::semantic::ENGINE_DECORATORS;
use crate::{Span, parser, tokenize};

/// The class every decorator of the program extends.
pub const DECORATOR_CLASS: &str = "Decorator";

/// `class Decorator`: by default it lets every call run and does nothing
/// after it.
const DECORATOR_SOURCE: &str = "class Decorator {
    func before(method: string) -> bool {
        return true
    }

    func after(method: string) {
    }
}
";

/// Where the `Decorator` class is placed among the program's texts: far
/// after every file, so no place of it is taken for a place of theirs.
const DECORATOR_START: usize = usize::MAX / 2;

/// The method of a decorator called before the decorated one.
const BEFORE: &str = "before";
/// The method of a decorator called after the decorated one.
const AFTER: &str = "after";

/// Whether `name` is one the compiler made up for a decorated method: no
/// name written in StarScript has `@`. Tools leave them out of what they
/// show.
#[must_use]
pub fn is_hidden_name(name: &str) -> bool {
    name.contains('@')
}

/// The name of the hidden field that keeps the decorator at `index` of the
/// method `method`.
#[must_use]
pub fn decorator_field(index: usize, method: &str) -> String {
    format!("@{index}@{method}")
}

/// The name of the hidden method that keeps the code of the decorated
/// method `method`.
fn body_method(method: &str) -> String {
    format!("{method}@body")
}

/// Whether `decorator` is a decorator of the program rather than one of
/// the engine's (`@range`…).
#[must_use]
pub fn is_program_decorator(decorator: &Decorator) -> bool {
    !ENGINE_DECORATORS.contains(&decorator.name.text.as_str())
}

/// Prepares `file` to be checked and translated: adds the `Decorator`
/// class and rewrites its decorated methods. The compiler does it before
/// checking a program; tools that only read its syntax see it as written.
pub fn prepare(file: &mut SourceFile) {
    let mut ids = Ids(file.next_expression_id);
    file.classes.push(decorator_class(&mut ids));
    for class in &mut file.classes {
        decorate(class, &mut ids);
    }
    file.next_expression_id = ids.0;
}

/// Numbers new expressions.
struct Ids(u32);

impl Ids {
    fn expression(&mut self, kind: ExprKind, span: Span) -> Expr {
        let id = ExprId::new(self.0);
        self.0 += 1;
        Expr { id, kind, span }
    }
}

/// The `Decorator` class, read from its StarScript.
fn decorator_class(ids: &mut Ids) -> ClassDecl {
    let mut tokens = tokenize(DECORATOR_SOURCE).unwrap_or_default();
    for token in &mut tokens {
        token.span = token.span.shifted(DECORATOR_START);
    }
    match parser::parse_numbering_from(&tokens, ids.0) {
        Ok((mut file, next_id)) => {
            ids.0 = next_id;
            file.classes
                .pop()
                .unwrap_or_else(|| empty_class(DECORATOR_CLASS))
        }
        // Its text is fixed and tested, so this never happens.
        Err(_) => empty_class(DECORATOR_CLASS),
    }
}

fn empty_class(name: &str) -> ClassDecl {
    let span = Span::new(DECORATOR_START, DECORATOR_START);
    ClassDecl {
        name: Name {
            text: name.to_owned(),
            span,
        },
        base: None,
        members: Vec::new(),
        span,
    }
}

/// Rewrites the methods of `class` that the program's decorators decorate.
fn decorate(class: &mut ClassDecl, ids: &mut Ids) {
    let mut added = Vec::new();
    for member in &mut class.members {
        if can_be_decorated(member) {
            added.extend(wrap(member, ids));
        }
    }
    class.members.extend(added);
}

/// Whether `member` is a method the program's decorators decorate and can
/// wrap: one that returns nothing and runs on an object at once. The
/// others keep their decorators for the analysis to report.
fn can_be_decorated(member: &Member) -> bool {
    let MemberKind::Method(method) = &member.kind else {
        return false;
    };
    let runs_at_once_on_an_object = !member.modifiers.iter().any(|modifier| {
        matches!(
            modifier.kind,
            ModifierKind::Static | ModifierKind::Async | ModifierKind::Parallel
        )
    });
    method.return_type.is_none()
        && runs_at_once_on_an_object
        && member.decorators.iter().any(is_program_decorator)
}

/// Rewrites the decorated method `member` to call its decorators around
/// its code, moved to a hidden method. Returns the hidden members.
fn wrap(member: &mut Member, ids: &mut Ids) -> Vec<Member> {
    let MemberKind::Method(method) = &mut member.kind else {
        return Vec::new();
    };
    let name = method.name.text.clone();
    let span = method.name.span;
    let mut hidden = Vec::new();
    let mut fields = Vec::new();
    for (index, decorator) in member
        .decorators
        .iter_mut()
        .filter(|decorator| is_program_decorator(decorator))
        .enumerate()
    {
        let field = decorator_field(index, &name);
        // The values given to the decorator now make it, once per object.
        let arguments = std::mem::take(&mut decorator.arguments);
        let class = ids.expression(
            ExprKind::Name(decorator.name.text.clone()),
            decorator.name.span,
        );
        let make = ids.expression(
            ExprKind::Call {
                callee: Box::new(class),
                arguments,
            },
            decorator.span,
        );
        hidden.push(private_member(
            MemberKind::Variable(VariableDecl {
                kind: VariableKind::Var,
                name: Name {
                    text: field.clone(),
                    span: decorator.span,
                },
                type_annotation: None,
                initializer: Some(make),
            }),
            decorator.span,
        ));
        fields.push((field, decorator.span));
    }
    // Members are told apart by where their names are, so the hidden
    // method is placed at its code.
    let code_span = method.body.span;
    let code = MethodDecl {
        name: Name {
            text: body_method(&name),
            span: code_span,
        },
        parameters: method.parameters.clone(),
        return_type: None,
        body: std::mem::replace(
            &mut method.body,
            Block {
                statements: Vec::new(),
                span,
            },
        ),
    };
    let arguments = method
        .parameters
        .iter()
        .map(|parameter| ids.expression(ExprKind::Name(parameter.name.text.clone()), span))
        .collect();
    let run = statement(
        StatementKind::Expression(call(ids, None, &body_method(&name), arguments, span)),
        span,
    );
    let mut statements = vec![run];
    for (field, decorator_span) in fields.iter().rev() {
        statements = around(ids, field, &name, statements, *decorator_span);
    }
    method.body.statements = statements;
    hidden.push(private_member(MemberKind::Method(code), code_span));
    hidden
}

/// `if field.before("method") { inner; field.after("method") }`.
fn around(
    ids: &mut Ids,
    field: &str,
    method: &str,
    mut inner: Vec<Statement>,
    span: Span,
) -> Vec<Statement> {
    let name = |ids: &mut Ids| {
        vec![ids.expression(ExprKind::Literal(Literal::String(method.to_owned())), span)]
    };
    let before_arguments = name(ids);
    let condition = call(ids, Some(field), BEFORE, before_arguments, span);
    let after_arguments = name(ids);
    let after = call(ids, Some(field), AFTER, after_arguments, span);
    inner.push(statement(StatementKind::Expression(after), span));
    vec![statement(
        StatementKind::If(IfStatement {
            condition,
            then_block: Block {
                statements: inner,
                span,
            },
            else_branch: None,
            span,
        }),
        span,
    )]
}

/// `object.method(arguments)`, or `method(arguments)` without an object.
fn call(
    ids: &mut Ids,
    object: Option<&str>,
    method: &str,
    arguments: Vec<Expr>,
    span: Span,
) -> Expr {
    let callee = match object {
        Some(object) => {
            let object = ids.expression(ExprKind::Name(object.to_owned()), span);
            ExprKind::Member {
                object: Box::new(object),
                name: Name {
                    text: method.to_owned(),
                    span,
                },
                null_safe: false,
            }
        }
        None => ExprKind::Name(method.to_owned()),
    };
    let callee = ids.expression(callee, span);
    ids.expression(
        ExprKind::Call {
            callee: Box::new(callee),
            arguments,
        },
        span,
    )
}

const fn statement(kind: StatementKind, span: Span) -> Statement {
    Statement { kind, span }
}

fn private_member(kind: MemberKind, span: Span) -> Member {
    Member {
        decorators: Vec::new(),
        modifiers: vec![Modifier {
            kind: ModifierKind::Private,
            span,
        }],
        kind,
        span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_source;

    #[test]
    fn the_decorator_class_reads() {
        let mut ids = Ids(0);

        let class = decorator_class(&mut ids);

        assert_eq!(class.name.text, DECORATOR_CLASS);
        assert_eq!(class.members.len(), 2);
        assert!(ids.0 > 0);
    }

    #[test]
    fn decorated_methods_call_their_decorators_around_their_code() {
        let mut file = parse_source(
            "class Door extends Node {\n  @Log\n  @Once\n  func open(force: int) {\n    \
             Debug.print(force.to_string())\n  }\n}\n",
        )
        .expect("valid");

        prepare(&mut file);

        let door = &file.classes[0];
        let names: Vec<String> = door
            .members
            .iter()
            .map(|member| match &member.kind {
                MemberKind::Variable(variable) => variable.name.text.clone(),
                MemberKind::Method(method) => method.name.text.clone(),
                MemberKind::Constructor(_) => "init".to_owned(),
            })
            .collect();
        assert_eq!(names, ["open", "@0@open", "@1@open", "open@body"]);
        let MemberKind::Method(open) = &door.members[0].kind else {
            panic!("a method");
        };
        let body = crate::ast::outline(&SourceFile {
            classes: vec![ClassDecl {
                members: vec![door.members[0].clone()],
                ..door.clone()
            }],
            next_expression_id: file.next_expression_id,
        });
        assert!(
            body.contains("if (call (. @0@open before) \"open\")"),
            "{body}"
        );
        assert!(
            body.contains("if (call (. @1@open before) \"open\")"),
            "{body}"
        );
        assert!(body.contains("(call open@body force)"), "{body}");
        assert_eq!(open.parameters.len(), 1);
    }

    #[test]
    fn the_decorators_program_checks() {
        let source = include_str!("../tests/programs/decorators.star");

        if let Err(errors) = crate::compile_source(source) {
            panic!("{errors:?}");
        }
    }

    fn mistakes(source: &str) -> Vec<String> {
        crate::check_source(source)
            .err()
            .unwrap_or_default()
            .iter()
            .map(ToString::to_string)
            .collect()
    }

    #[test]
    fn unknown_decorators_are_reported_once() {
        let found = mistakes("class A {\n  @Ghost(1)\n  func go() {}\n}\n");

        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("there is no decorator `@Ghost`"),
            "{found:?}"
        );
    }

    #[test]
    fn decorators_are_classes_that_extend_decorator_on_plain_methods() {
        let found = mistakes(
            "class Plain {}\nclass Once extends Decorator {}\n\
             class A {\n  @Plain\n  func a() {}\n  @Once\n  func b() -> int {\n    return 1\n  }\n  \
             @Once\n  static func c() {}\n}\n",
        );

        assert_eq!(found.len(), 3, "{found:?}");
        assert!(found[0].contains("`Plain` is not a decorator"), "{found:?}");
        assert!(
            found[1].contains("`@Once` only goes on methods that return nothing"),
            "{found:?}"
        );
        assert!(
            found[2].contains("`@Once` only goes on methods that return nothing"),
            "{found:?}"
        );
    }

    #[test]
    fn decorator_arguments_are_checked_as_their_init_takes_them() {
        let found = mistakes(
            "class Wait extends Decorator {\n  var seconds: float\n  init(seconds: float) {\n    \
             self.seconds = seconds\n  }\n}\nclass A {\n  @Wait(\"long\")\n  func go() {}\n}\n",
        );

        assert_eq!(found.len(), 1, "{found:?}");
        assert!(
            found[0].contains("expected `float`, found `string`"),
            "{found:?}"
        );
    }

    #[test]
    fn methods_that_return_values_are_left_as_they_are() {
        let mut file =
            parse_source("class A {\n  @Once\n  func count() -> int {\n    return 1\n  }\n}\n")
                .expect("valid");

        prepare(&mut file);

        assert_eq!(file.classes[0].members.len(), 1);
    }
}
