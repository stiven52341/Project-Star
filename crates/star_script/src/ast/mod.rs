//! Abstract syntax tree: the structure of a StarScript program.
//!
//! Every node keeps the [`Span`] of the code it came from, so later stages can
//! point error messages at the exact place.

mod declaration;
mod expression;
mod outline;
mod statement;

use std::fmt;

pub use declaration::{
    ClassDecl, ConstructorDecl, Member, MemberKind, MethodDecl, Modifier, ModifierKind, Parameter,
    VariableDecl, VariableKind,
};
pub use expression::{BinaryOperator, Expr, ExprId, ExprKind, Literal, UnaryOperator};
pub use outline::outline;
pub use statement::{AssignOperator, Block, ElseBranch, IfStatement, Statement, StatementKind};

use crate::Span;

/// A whole `.star` file.
#[derive(Debug, Clone, PartialEq)]
pub struct SourceFile {
    /// The classes declared in the file, in source order.
    pub classes: Vec<ClassDecl>,
}

/// A name written in the source, such as a class, variable or method name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Name {
    /// The name itself.
    pub text: String,
    /// Where it is written.
    pub span: Span,
}

impl fmt::Display for Name {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.text)
    }
}

/// A type written in the source, such as `float`, `[Enemy]` or `Player?`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeRef {
    /// Which type.
    pub kind: TypeRefKind,
    /// Where it is written.
    pub span: Span,
}

/// Every way of writing a type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeRefKind {
    /// A type by name: `int`, `Player`.
    Named(String),
    /// `[Element]`: a list.
    List(Box<TypeRef>),
    /// `Type?`: the type or `null`.
    Nullable(Box<TypeRef>),
}

impl fmt::Display for TypeRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            TypeRefKind::Named(name) => formatter.write_str(name),
            TypeRefKind::List(element) => write!(formatter, "[{element}]"),
            TypeRefKind::Nullable(inner) => write!(formatter, "{inner}?"),
        }
    }
}
