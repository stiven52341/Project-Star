//! What semantic analysis learns about a program: the result later stages use.

use std::collections::HashMap;

use super::{BuiltinMethod, ClassId, ClassTable, Type};
use crate::Span;
use crate::ast::{Expr, ExprId, Name};

/// Identifies a local variable, parameter or loop variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LocalId(usize);

/// A local variable, parameter or loop variable.
#[derive(Debug, Clone, PartialEq)]
pub struct LocalInfo {
    /// Its name.
    pub name: String,
    /// Its declared or inferred type.
    pub ty: Type,
    /// Whether it can be assigned after its declaration (only `var`).
    pub is_mutable: bool,
}

/// What a name, member access or call refers to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// A local variable, parameter or loop variable.
    Local(LocalId),
    /// A field of `class`, at this index of its fields.
    Field {
        /// The class that declares the field.
        class: ClassId,
        /// Index into its fields.
        index: usize,
    },
    /// A constant of `class`.
    Constant {
        /// The class that declares the constant.
        class: ClassId,
        /// Index into its constants.
        index: usize,
    },
    /// A method of `class`.
    Method {
        /// The class that declares the method.
        class: ClassId,
        /// Index into its methods.
        index: usize,
    },
    /// A method of a basic type or list.
    Builtin(BuiltinMethod),
    /// The class itself, as in `Debug.print(…)`.
    Class(ClassId),
    /// A call that creates an object: `Player(…)`.
    Constructor(ClassId),
    /// `super.init(…)` of this base class.
    SuperConstructor(ClassId),
}

/// What is known about one expression.
#[derive(Debug, Clone, PartialEq)]
pub struct ExprInfo {
    /// Its type (after any narrowing such as `if x != null`).
    pub ty: Type,
    /// What it refers to, for names, member accesses and callees.
    pub resolution: Option<Resolution>,
}

/// Result of semantic analysis: every class, the type of every expression
/// and what every name refers to.
#[derive(Debug, Clone, Default)]
pub struct Analysis {
    pub(super) classes: ClassTable,
    pub(super) expressions: HashMap<ExprId, ExprInfo>,
    pub(super) locals: Vec<LocalInfo>,
    pub(super) declarations: HashMap<Span, LocalId>,
}

impl Analysis {
    /// Every class of the program, native ones included.
    #[must_use]
    pub const fn classes(&self) -> &ClassTable {
        &self.classes
    }

    /// What is known about `expression`.
    #[must_use]
    pub fn expression(&self, expression: &Expr) -> Option<&ExprInfo> {
        self.expressions.get(&expression.id)
    }

    /// The type of `expression`.
    #[must_use]
    pub fn type_of(&self, expression: &Expr) -> Option<&Type> {
        self.expression(expression).map(|info| &info.ty)
    }

    /// The local declared with `name` (a `var`, parameter or loop variable).
    #[must_use]
    pub fn declared_local(&self, name: &Name) -> Option<LocalId> {
        self.declarations.get(&name.span).copied()
    }

    /// The local with that identifier.
    #[must_use]
    pub fn local(&self, id: LocalId) -> Option<&LocalInfo> {
        self.locals.get(id.0)
    }

    pub(super) fn add_local(&mut self, declaration: &Name, local: LocalInfo) -> LocalId {
        self.locals.push(local);
        let id = LocalId(self.locals.len() - 1);
        self.declarations.insert(declaration.span, id);
        id
    }

    pub(super) fn record(&mut self, expression: &Expr, ty: Type, resolution: Option<Resolution>) {
        self.expressions
            .insert(expression.id, ExprInfo { ty, resolution });
    }
}
