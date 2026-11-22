//! Statements: the steps inside a method.

use super::{Expr, Name, VariableDecl};
use crate::Span;

/// `{ statements }`.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// The statements, in order.
    pub statements: Vec<Statement>,
    /// The whole block, braces included.
    pub span: Span,
}

/// One step of a method.
#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    /// What the step does.
    pub kind: StatementKind,
    /// The whole statement.
    pub span: Span,
}

/// Every kind of statement.
#[derive(Debug, Clone, PartialEq)]
pub enum StatementKind {
    /// A local `var` or `const`.
    Variable(VariableDecl),
    /// `target = value`, `target += value`, …
    Assignment {
        /// What receives the value: a variable, field or indexed element.
        target: Expr,
        /// `=` or a compound operator such as `+=`.
        operator: AssignOperator,
        /// The value assigned.
        value: Expr,
    },
    /// An expression evaluated for its effect, usually a call.
    Expression(Expr),
    /// `if condition { … } else { … }`.
    If(IfStatement),
    /// `while condition { … }`.
    While {
        /// Checked before each repetition.
        condition: Expr,
        /// Repeated while the condition holds.
        body: Block,
    },
    /// `for variable in iterable { … }`.
    For {
        /// Takes each value in turn.
        variable: Name,
        /// A range such as `0..10` or a list.
        iterable: Expr,
        /// Runs once per value.
        body: Block,
    },
    /// `return` or `return value`.
    Return(Option<Expr>),
    /// `break`: leaves the innermost loop.
    Break,
    /// `continue`: skips to the next repetition of the innermost loop.
    Continue,
}

/// `if condition { … }` with its optional `else`.
#[derive(Debug, Clone, PartialEq)]
pub struct IfStatement {
    /// Decides which branch runs.
    pub condition: Expr,
    /// Runs when the condition holds.
    pub then_block: Block,
    /// Runs otherwise.
    pub else_branch: Option<ElseBranch>,
    /// The whole statement, `else` branches included.
    pub span: Span,
}

/// What follows `else`.
#[derive(Debug, Clone, PartialEq)]
pub enum ElseBranch {
    /// `else { … }`.
    Block(Block),
    /// `else if …`.
    If(Box<IfStatement>),
}

/// `=` and the compound assignments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssignOperator {
    /// `=`
    Assign,
    /// `+=`
    Add,
    /// `-=`
    Subtract,
    /// `*=`
    Multiply,
    /// `/=`
    Divide,
    /// `%=`
    Remainder,
}

impl AssignOperator {
    /// How the operator is written.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Assign => "=",
            Self::Add => "+=",
            Self::Subtract => "-=",
            Self::Multiply => "*=",
            Self::Divide => "/=",
            Self::Remainder => "%=",
        }
    }
}
