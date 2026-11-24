//! Classes and their members.

use std::fmt;

use super::{Block, Expr, Name, TypeRef};
use crate::{Keyword, Span};

/// `class Name extends Base { members }`.
#[derive(Debug, Clone, PartialEq)]
pub struct ClassDecl {
    /// Name of the class.
    pub name: Name,
    /// Class it inherits from, if any.
    pub base: Option<Name>,
    /// Fields, constants, methods and constructors, in source order.
    pub members: Vec<Member>,
    /// The whole declaration.
    pub span: Span,
}

/// One member of a class, with its decorators and modifiers.
#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    /// `@name` or `@name(arguments)` on the lines before it, in source
    /// order.
    pub decorators: Vec<Decorator>,
    /// `private`, `static`, `override`, `async`, `parallel` and `export`,
    /// in source order.
    pub modifiers: Vec<Modifier>,
    /// What the member is.
    pub kind: MemberKind,
    /// The whole member, modifiers included.
    pub span: Span,
}

impl Member {
    /// Whether the member was declared with `kind`.
    #[must_use]
    pub fn has_modifier(&self, kind: ModifierKind) -> bool {
        self.modifiers.iter().any(|modifier| modifier.kind == kind)
    }
}

/// `@name(arguments)` before a member: something the engine (or a
/// decorator class) does with it.
#[derive(Debug, Clone, PartialEq)]
pub struct Decorator {
    /// Its name: `range`, `tooltip`… or a class that extends `Decorator`.
    pub name: Name,
    /// The values in parentheses; none without them.
    pub arguments: Vec<Expr>,
    /// From `@` to the end.
    pub span: Span,
}

/// Every kind of class member.
#[derive(Debug, Clone, PartialEq)]
pub enum MemberKind {
    /// A field (`var`) or a class constant (`const`).
    Variable(VariableDecl),
    /// `func name(parameters) -> Type { body }`.
    Method(MethodDecl),
    /// `init(parameters) { body }`.
    Constructor(ConstructorDecl),
}

/// A modifier written before a member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Modifier {
    /// Which modifier.
    pub kind: ModifierKind,
    /// Where it is written.
    pub span: Span,
}

/// Every modifier a member can have.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModifierKind {
    /// Only usable inside its class.
    Private,
    /// Belongs to the class instead of each object.
    Static,
    /// Redefines a method of the base class.
    Override,
    /// A method that can wait across frames, with `await`.
    Async,
    /// A static function that runs on another thread.
    Parallel,
    /// A field of a node that scenes, and so the editor's inspector, set.
    Export,
}

impl ModifierKind {
    /// The modifier spelled by `keyword`, if it is one.
    #[must_use]
    pub const fn from_keyword(keyword: Keyword) -> Option<Self> {
        match keyword {
            Keyword::Private => Some(Self::Private),
            Keyword::Static => Some(Self::Static),
            Keyword::Override => Some(Self::Override),
            Keyword::Async => Some(Self::Async),
            Keyword::Parallel => Some(Self::Parallel),
            Keyword::Export => Some(Self::Export),
            _ => None,
        }
    }

    /// The keyword that spells the modifier.
    #[must_use]
    pub const fn keyword(self) -> Keyword {
        match self {
            Self::Private => Keyword::Private,
            Self::Static => Keyword::Static,
            Self::Override => Keyword::Override,
            Self::Async => Keyword::Async,
            Self::Parallel => Keyword::Parallel,
            Self::Export => Keyword::Export,
        }
    }
}

impl fmt::Display for ModifierKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.keyword().fmt(formatter)
    }
}

/// `var name: Type = value` or `const NAME = value`, as a field or a local.
#[derive(Debug, Clone, PartialEq)]
pub struct VariableDecl {
    /// Whether the value can change.
    pub kind: VariableKind,
    /// Name of the variable.
    pub name: Name,
    /// Declared type; `None` when it is inferred from the value.
    pub type_annotation: Option<TypeRef>,
    /// Initial value; always present for constants.
    pub initializer: Option<Expr>,
}

impl fmt::Display for VariableDecl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{} {}", self.kind, self.name)?;
        if let Some(type_annotation) = &self.type_annotation {
            write!(formatter, ": {type_annotation}")?;
        }
        if let Some(initializer) = &self.initializer {
            write!(formatter, " = {initializer}")?;
        }
        Ok(())
    }
}

/// Whether a variable can be reassigned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableKind {
    /// `var`: can change.
    Var,
    /// `const`: fixed once set.
    Const,
}

impl fmt::Display for VariableKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Var => "var",
            Self::Const => "const",
        })
    }
}

/// `func name(parameters) -> Type { body }`.
#[derive(Debug, Clone, PartialEq)]
pub struct MethodDecl {
    /// Name of the method.
    pub name: Name,
    /// Parameters, in order.
    pub parameters: Vec<Parameter>,
    /// Type of the returned value; `None` when it returns nothing.
    pub return_type: Option<TypeRef>,
    /// The code of the method.
    pub body: Block,
}

/// `init(parameters) { body }`: runs when an object is created.
#[derive(Debug, Clone, PartialEq)]
pub struct ConstructorDecl {
    /// Parameters, in order.
    pub parameters: Vec<Parameter>,
    /// The code of the constructor.
    pub body: Block,
}

/// `name: Type` in a parameter list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    /// Name of the parameter.
    pub name: Name,
    /// Its type, always written.
    pub type_annotation: TypeRef,
    /// The whole parameter.
    pub span: Span,
}

impl fmt::Display for Parameter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.name, self.type_annotation)
    }
}
