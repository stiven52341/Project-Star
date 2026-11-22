//! Indented text view of a syntax tree, for tools and tests.

use std::fmt::Display;

use super::{
    Block, ClassDecl, ElseBranch, IfStatement, Member, MemberKind, Parameter, SourceFile,
    Statement, StatementKind,
};

const INDENT: &str = "  ";

/// Describes `file` one node per line, nesting shown by indentation.
///
/// Expressions are fully parenthesized (`(+ 1 (* 2 3))`), so the outline shows
/// exactly how the parser understood the code.
#[must_use]
pub fn outline(file: &SourceFile) -> String {
    let mut writer = OutlineWriter::default();
    for class in &file.classes {
        writer.class(class);
    }
    writer.text
}

#[derive(Default)]
struct OutlineWriter {
    text: String,
    depth: usize,
}

impl OutlineWriter {
    fn line(&mut self, content: impl Display) {
        self.text.push_str(&INDENT.repeat(self.depth));
        self.text.push_str(&content.to_string());
        self.text.push('\n');
    }

    fn nested(&mut self, write: impl FnOnce(&mut Self)) {
        self.depth += 1;
        write(self);
        self.depth -= 1;
    }

    fn class(&mut self, class: &ClassDecl) {
        match &class.base {
            Some(base) => self.line(format!("class {} extends {base}", class.name)),
            None => self.line(format!("class {}", class.name)),
        }
        self.nested(|writer| {
            for member in &class.members {
                writer.member(member);
            }
        });
    }

    fn member(&mut self, member: &Member) {
        let modifiers: String = member
            .modifiers
            .iter()
            .map(|modifier| modifier.kind.to_string() + " ")
            .collect();
        match &member.kind {
            MemberKind::Variable(variable) => self.line(format!("{modifiers}{variable}")),
            MemberKind::Method(method) => {
                let return_type = method
                    .return_type
                    .as_ref()
                    .map(|type_ref| format!(" -> {type_ref}"))
                    .unwrap_or_default();
                self.line(format!(
                    "{modifiers}func {}({}){return_type}",
                    method.name,
                    parameter_list(&method.parameters)
                ));
                self.block(&method.body);
            }
            MemberKind::Constructor(constructor) => {
                self.line(format!(
                    "{modifiers}init({})",
                    parameter_list(&constructor.parameters)
                ));
                self.block(&constructor.body);
            }
        }
    }

    fn block(&mut self, block: &Block) {
        self.nested(|writer| {
            for statement in &block.statements {
                writer.statement(statement);
            }
        });
    }

    fn statement(&mut self, statement: &Statement) {
        match &statement.kind {
            StatementKind::Variable(variable) => self.line(variable),
            StatementKind::Assignment {
                target,
                operator,
                value,
            } => self.line(format!("({} {target} {value})", operator.symbol())),
            StatementKind::Expression(expression) => self.line(expression),
            StatementKind::If(if_statement) => self.if_statement(if_statement, "if"),
            StatementKind::While { condition, body } => {
                self.line(format!("while {condition}"));
                self.block(body);
            }
            StatementKind::For {
                variable,
                iterable,
                body,
            } => {
                self.line(format!("for {variable} in {iterable}"));
                self.block(body);
            }
            StatementKind::Return(Some(value)) => self.line(format!("return {value}")),
            StatementKind::Return(None) => self.line("return"),
            StatementKind::Break => self.line("break"),
            StatementKind::Continue => self.line("continue"),
        }
    }

    fn if_statement(&mut self, if_statement: &IfStatement, keyword: &str) {
        self.line(format!("{keyword} {}", if_statement.condition));
        self.block(&if_statement.then_block);
        match &if_statement.else_branch {
            Some(ElseBranch::Block(block)) => {
                self.line("else");
                self.block(block);
            }
            Some(ElseBranch::If(nested)) => self.if_statement(nested, "else if"),
            None => {}
        }
    }
}

fn parameter_list(parameters: &[Parameter]) -> String {
    parameters
        .iter()
        .map(Parameter::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}
