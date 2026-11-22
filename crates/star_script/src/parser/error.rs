//! Mistakes the parser can find in the structure of the code.

use std::error::Error;
use std::fmt;

use crate::ast::ModifierKind;
use crate::{Span, TokenKind};

/// A syntax mistake and where it is.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    /// What is wrong.
    pub kind: ParseErrorKind,
    /// The code that is wrong.
    pub span: Span,
}

impl ParseError {
    pub(super) const fn new(kind: ParseErrorKind, span: Span) -> Self {
        Self { kind, span }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.kind.fmt(formatter)
    }
}

impl Error for ParseError {}

/// Every kind of syntax mistake.
#[derive(Debug, Clone, PartialEq)]
pub enum ParseErrorKind {
    /// Something other than what the grammar allows at this point.
    Unexpected {
        /// What could have been written here.
        expected: Expected,
        /// What was written instead.
        found: TokenKind,
    },
    /// `a < b < c`: comparisons, type tests and ranges cannot be chained.
    ChainedOperators,
    /// `f() = 3`: only variables, fields and list elements can be assigned.
    InvalidAssignmentTarget,
    /// `if x = 3`: assignment where a condition was expected.
    AssignmentInCondition,
    /// `private private var x`.
    DuplicateModifier(ModifierKind),
    /// `const LIMIT` without `= value`.
    ConstantWithoutValue,
    /// Nesting so deep that it is surely a mistake, and could exhaust the stack.
    TooDeeplyNested,
}

impl fmt::Display for ParseErrorKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unexpected { expected, found } => {
                write!(formatter, "expected {expected}, found {found}")?;
                if matches!(found, TokenKind::Keyword(keyword) if keyword.is_reserved()) {
                    formatter.write_str(" (reserved for a future version of the language)")?;
                }
                Ok(())
            }
            Self::ChainedOperators => formatter.write_str(
                "comparisons and ranges cannot be chained; combine them with `&&` or `||`",
            ),
            Self::InvalidAssignmentTarget => formatter
                .write_str("only variables, fields and list elements can be assigned a value"),
            Self::AssignmentInCondition => {
                formatter.write_str("`=` assigns a value; use `==` to compare")
            }
            Self::DuplicateModifier(modifier) => {
                write!(formatter, "modifier `{modifier}` is repeated")
            }
            Self::ConstantWithoutValue => {
                formatter.write_str("a `const` needs a value: `const NAME = value`")
            }
            Self::TooDeeplyNested => formatter.write_str("code is nested too deeply"),
        }
    }
}

/// What the parser was looking for when it found something else.
#[derive(Debug, Clone, PartialEq)]
pub enum Expected {
    /// A specific token, such as `)`.
    Token(TokenKind),
    /// The name of a class, variable, method or parameter.
    Name,
    /// The name of a type.
    Type,
    /// Any expression.
    Expression,
    /// The start of a class member.
    ClassMember,
    /// The start of a class.
    Class,
    /// A line break or `;`.
    EndOfStatement,
}

impl fmt::Display for Expected {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Token(kind) => kind.fmt(formatter),
            Self::Name => formatter.write_str("a name"),
            Self::Type => formatter.write_str("a type"),
            Self::Expression => formatter.write_str("an expression"),
            Self::ClassMember => {
                formatter.write_str("a class member (`var`, `const`, `func` or `init`)")
            }
            Self::Class => formatter.write_str("`class` (all code lives inside classes)"),
            Self::EndOfStatement => formatter.write_str("a line break or `;`"),
        }
    }
}
