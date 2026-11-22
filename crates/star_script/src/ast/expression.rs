//! Expressions: code that produces a value.

use std::fmt;

use super::{Name, TypeRef};
use crate::Span;

/// A piece of code that produces a value.
#[derive(Debug, Clone, PartialEq)]
pub struct Expr {
    /// Identifies the expression within its file, so later stages can attach
    /// information (such as its type) to it.
    pub id: ExprId,
    /// What the expression computes.
    pub kind: ExprKind,
    /// The whole expression, parentheses included.
    pub span: Span,
}

/// Unique identifier of an expression within a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExprId(u32);

impl ExprId {
    pub(crate) const fn new(index: u32) -> Self {
        Self(index)
    }
}

/// Every kind of expression.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprKind {
    /// A value written directly, such as `3` or `"hola"`.
    Literal(Literal),
    /// A variable, field, constant or class, referred to by name.
    Name(String),
    /// `self`: the object running the method.
    SelfValue,
    /// `super`: the base-class part of `self`.
    Super,
    /// `-value` or `!value`.
    Unary {
        /// Which operation.
        operator: UnaryOperator,
        /// The value it applies to.
        operand: Box<Expr>,
    },
    /// `left operator right`.
    Binary {
        /// Which operation.
        operator: BinaryOperator,
        /// Left-hand value.
        left: Box<Expr>,
        /// Right-hand value.
        right: Box<Expr>,
    },
    /// `value is Type`: whether the value is an object of that class.
    TypeTest {
        /// The value checked.
        value: Box<Expr>,
        /// The class it is checked against.
        type_ref: TypeRef,
    },
    /// `value as Type`: conversion to another type.
    Cast {
        /// The value converted.
        value: Box<Expr>,
        /// The type it is converted to.
        type_ref: TypeRef,
    },
    /// `callee(arguments)`: calls a method or creates an object.
    Call {
        /// What is called.
        callee: Box<Expr>,
        /// The values passed, in order.
        arguments: Vec<Expr>,
    },
    /// `object.name`, or `object?.name` which gives `null` when the object is `null`.
    Member {
        /// The object.
        object: Box<Expr>,
        /// The field or method.
        name: Name,
        /// Whether written with `?.`.
        null_safe: bool,
    },
    /// `object[index]`: an element of a list.
    Index {
        /// The list.
        object: Box<Expr>,
        /// Position of the element.
        index: Box<Expr>,
    },
    /// `[a, b, c]`: a list.
    Array(Vec<Expr>),
}

impl fmt::Display for Expr {
    /// Writes the expression fully parenthesized, e.g. `(+ 1 (* 2 3))`, so its
    /// structure can be read without knowing operator precedence.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ExprKind::Literal(literal) => literal.fmt(formatter),
            ExprKind::Name(name) => formatter.write_str(name),
            ExprKind::SelfValue => formatter.write_str("self"),
            ExprKind::Super => formatter.write_str("super"),
            ExprKind::Unary { operator, operand } => {
                write!(formatter, "({} {operand})", operator.symbol())
            }
            ExprKind::Binary {
                operator,
                left,
                right,
            } => write!(formatter, "({} {left} {right})", operator.symbol()),
            ExprKind::TypeTest { value, type_ref } => {
                write!(formatter, "(is {value} {type_ref})")
            }
            ExprKind::Cast { value, type_ref } => write!(formatter, "(as {value} {type_ref})"),
            ExprKind::Call { callee, arguments } => {
                write!(formatter, "(call {callee}")?;
                write_each(formatter, arguments)
            }
            ExprKind::Member {
                object,
                name,
                null_safe,
            } => {
                let access = if *null_safe { "?." } else { "." };
                write!(formatter, "({access} {object} {name})")
            }
            ExprKind::Index { object, index } => write!(formatter, "(index {object} {index})"),
            ExprKind::Array(elements) => {
                formatter.write_str("(array")?;
                write_each(formatter, elements)
            }
        }
    }
}

/// Writes ` item` for each item, then the closing parenthesis.
fn write_each(formatter: &mut fmt::Formatter<'_>, items: &[Expr]) -> fmt::Result {
    for item in items {
        write!(formatter, " {item}")?;
    }
    formatter.write_str(")")
}

/// A value written directly in the code.
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    /// `42`
    Integer(i64),
    /// `3.14`
    Float(f64),
    /// `"hola"`
    String(String),
    /// `true` or `false`
    Bool(bool),
    /// `null`
    Null,
}

impl fmt::Display for Literal {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(value) => write!(formatter, "{value}"),
            Self::Float(value) => write!(formatter, "{value:?}"),
            Self::String(value) => write!(formatter, "{value:?}"),
            Self::Bool(value) => write!(formatter, "{value}"),
            Self::Null => formatter.write_str("null"),
        }
    }
}

/// Operators written before a single value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    /// `-`: changes the sign.
    Negate,
    /// `!`: logical not.
    Not,
}

impl UnaryOperator {
    /// How the operator is written.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Negate => "-",
            Self::Not => "!",
        }
    }
}

/// Operators written between two values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    /// `??`: the left value, or the right one if the left is `null`.
    Coalesce,
    /// `||`
    Or,
    /// `&&`
    And,
    /// `==`
    Equal,
    /// `!=`
    NotEqual,
    /// `<`
    Less,
    /// `<=`
    LessEqual,
    /// `>`
    Greater,
    /// `>=`
    GreaterEqual,
    /// `..`: the integers from the left value up to, not including, the right one.
    Range,
    /// `+`
    Add,
    /// `-`
    Subtract,
    /// `*`
    Multiply,
    /// `/`
    Divide,
    /// `%`
    Remainder,
}

impl BinaryOperator {
    /// How the operator is written.
    #[must_use]
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Coalesce => "??",
            Self::Or => "||",
            Self::And => "&&",
            Self::Equal => "==",
            Self::NotEqual => "!=",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::Range => "..",
            Self::Add => "+",
            Self::Subtract => "-",
            Self::Multiply => "*",
            Self::Divide => "/",
            Self::Remainder => "%",
        }
    }
}
